//! The crew loop: Planner -> Workers (parallel per wave) -> Reviewer -> fixes.
//!
//! Agents run as interactive Claude Code sessions in visible panes. Each one
//! writes a JSON file; a `Stop` hook tells us its turn ended, then we read it.
//! If the file is missing (the agent asked a question, hit a permission
//! prompt...), we keep waiting: the user can answer in the pane.

use crate::{
    budget::{self, Mode, Tune},
    e2e,
    hooks::HookServer,
    plan::{read_json, Bar, Fix, Gate, Plan, Report, Review},
    prompts,
    review::Metrics,
    setup, AgentState, CrewEvent, Phase, Role,
};
use anyhow::{anyhow, bail, Result};
use futures_util::future::join_all;
use serde::de::DeserializeOwned;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::watch;

/// Where agents run. Implemented by the app on top of its pane manager.
pub trait AgentHost: Send + Sync + 'static {
    /// Starts `claude --agent <role> --settings <settings> "<prompt>"` in a new pane.
    /// `tune` replaces the role's model and effort for this agent (Claude only).
    fn spawn(&self, key: &str, role: Role, prompt: &str, settings: &Path, tune: Option<Tune>) -> Result<String>;
    fn close(&self, pane: &str);
    /// Whether the CLI running `role` reports lifecycle through Claude Code hooks.
    /// Without hooks, completion is detected by polling the output file.
    fn has_hooks(&self, _role: Role) -> bool {
        true
    }
}

pub struct Engine<H: AgentHost> {
    pub host: Arc<H>,
    pub hooks: Arc<HookServer>,
    pub root: PathBuf,
    pub emit: Arc<dyn Fn(CrewEvent) + Send + Sync>,
    pub cancel: watch::Receiver<bool>,
    pub max_rounds: u32,
    pub agent_timeout: Duration,
    /// Run the project's e2e suite before each review when it is configured.
    pub run_e2e: bool,
    /// Agent threads (run + lots). Empty = no threads.
    pub threads: crate::threading::RunThreads,
    /// How heavy this run is (picked by the orchestrator).
    pub mode: Mode,
    /// The week runs ahead of its allowance: spare Opus.
    pub eco: bool,
}

pub(crate) struct Run {
    pub(crate) id: String,
    pub(crate) dir: PathBuf,
    pub(crate) task: String,
}

/// What a successful run hands to the user.
#[derive(Debug, Clone)]
pub struct Outcome {
    pub review: Review,
    pub gate: Gate,
}

impl<H: AgentHost> Engine<H> {
    pub(crate) fn say(&self, run: &Run, phase: Phase, round: u32, message: impl Into<String>) {
        let message = message.into();
        self.threads.event(&run.id, &message);
        (self.emit)(CrewEvent { run: run.id.clone(), phase, round, message, review: None, gate: None });
    }

    pub(crate) fn rel(&self, p: &Path) -> String {
        p.strip_prefix(&self.root).unwrap_or(p).display().to_string().replace('\\', "/")
    }

    pub async fn run(&self, task: &str) -> Result<Outcome> {
        self.run_resumable(task, None).await
    }

    /// Runs a task, or resumes run `id`: every agent whose output file already
    /// exists is skipped, so an interrupted run continues where it stopped.
    pub async fn run_resumable(&self, task: &str, id: Option<String>) -> Result<Outcome> {
        let id = id.unwrap_or_else(|| SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0).to_string());
        let run = Run { dir: setup::run_dir(&self.root, &id), id, task: task.to_string() };
        std::fs::create_dir_all(run.dir.join("reports"))?;
        std::fs::create_dir_all(run.dir.join("agents"))?;
        self.threads.open_run(&run.id, task);
        let mut metrics = Metrics::new(budget::effective(self.mode, self.eco), self.eco);
        let result = self.run_inner(&run, &mut metrics).await;
        metrics.passed = result.is_ok();
        let _ = std::fs::write(run.dir.join("metrics.json"), serde_json::to_string_pretty(&metrics).unwrap_or_default());
        match result {
            Ok(out) => {
                self.threads.close_run(&run.id, &out.gate.summary_for_user);
                let msg = format!("Prêt pour toi : {}", out.gate.summary_for_user);
                (self.emit)(CrewEvent { run: run.id.clone(), phase: Phase::Done, round: 0, message: msg, review: Some(out.review.clone()), gate: Some(out.gate.clone()) });
                Ok(out)
            }
            Err(e) => {
                let phase = if *self.cancel.borrow() { Phase::Cancelled } else { Phase::Failed };
                self.say(&run, phase, 0, format!("{e:#}"));
                Err(e)
            }
        }
    }

    async fn run_inner(&self, run: &Run, m: &mut Metrics) -> Result<Outcome> {
        let mode = m.mode;
        let plan_path = run.dir.join("plan.json");
        let plan: Plan = if mode == Mode::Light {
            // A small task needs no planner: one lot, the task itself.
            self.say(run, Phase::Planning, 0, "Run léger : un seul worker, sans planner ni Teacher séparé");
            let plan = Plan::single(&run.task);
            if !plan_path.exists() {
                std::fs::write(&plan_path, serde_json::to_string_pretty(&plan)?)?;
            }
            plan
        } else {
            self.say(run, Phase::Planning, 0, "Planner au travail");
            let prompt = prompts::planner(&run.task, &self.rel(&plan_path)) + &prompts::run_thread_note(&run.id);
            self.agent(run, "planner", Role::Planner, &prompt, &plan_path).await?
        };
        plan.validate()?;
        self.threads.open_lots(&run.id, &plan);

        let bar_path = run.dir.join("bar.json");
        let bar: Option<Bar> = if mode == Mode::Light {
            None
        } else {
            self.say(run, Phase::Calibrating, 0, "Teacher fixe la barre de validation");
            let prompt = prompts::teacher_bar(&run.task, &self.rel(&plan_path), &self.rel(&bar_path), e2e::configured(&self.root));
            let bar: Bar = self.agent(run, "teacher-bar", Role::Teacher, &prompt, &bar_path).await?;
            self.say(run, Phase::Calibrating, 0, format!("Barre : {} critère(s), risque {}. Te déranger quand : {}", bar.criteria.len(), or(&bar.risk, "medium"), bar.notify_user_when));
            Some(bar)
        };
        let high = bar.as_ref().is_some_and(|b| b.risk.eq_ignore_ascii_case("high"));
        // A risky run gets the deep review, unless the week is short of allowance.
        let deep = mode == Mode::Deep || (high && !self.eco);

        let waves = plan.effective_waves();
        self.say(run, Phase::Working, 0, format!("{} lots en {} vague(s)", plan.lots.len(), waves.len()));
        let mut reports: BTreeMap<String, PathBuf> = BTreeMap::new();
        for (i, wave) in waves.iter().enumerate() {
            self.say(run, Phase::Working, 0, format!("Vague {} : {}", i + 1, wave.join(", ")));
            let jobs: Vec<(String, Vec<Fix>)> = wave.iter().map(|id| (id.clone(), vec![])).collect();
            reports.extend(self.workers(run, &plan, &jobs, 0).await?);
        }

        for round in 0..self.max_rounds {
            m.rounds = round + 1;
            self.say(run, Phase::Reviewing, round, format!("Revue{}, tour {}", if deep { " renforcée" } else { "" }, round + 1));
            let e2e_note = self.e2e_gate(run, round).await;
            let list: Vec<String> = reports.values().map(|p| self.rel(p)).collect();
            let previous: Vec<String> = (0..round).map(|r| self.rel(&run.dir.join(format!("review-{r}.json")))).collect();
            let review = self.review(run, &plan, &self.rel(&plan_path), round, &list, &previous, e2e_note.as_deref(), deep).await?;
            m.count(&review.fixes);
            let old: Vec<String> = review.fixes.iter().filter(|f| f.pre_existing()).map(|f| format!("{} : {}", f.r#where, f.problem)).collect();
            if !old.is_empty() {
                self.threads.event(&run.id, &format!("Déjà là avant ce run, non bloquant :\n- {}", old.join("\n- ")));
            }

            let fixes: Vec<Fix> = if review.passed() {
                let Some(bar_file) = bar.as_ref().map(|_| self.rel(&bar_path)) else {
                    let summary = or(&review.summary, "Revue PASS.").to_string();
                    return Ok(Outcome { review, gate: Gate { ready: true, met: vec![], unmet: vec![], summary_for_user: summary } });
                };
                self.say(run, Phase::Gating, round, "Teacher vérifie la barre avant de te solliciter");
                let review_path = run.dir.join(format!("review-{round}.json"));
                let gate_path = run.dir.join(format!("gate-{round}.json"));
                let prompt = prompts::teacher_gate(&run.task, &bar_file, &self.rel(&review_path), e2e_note.as_deref(), &self.rel(&gate_path));
                let mut gate: Gate = self.agent(run, &format!("teacher-gate-{round}"), Role::Teacher, &prompt, &gate_path).await?;
                if gate.ready {
                    if high {
                        gate.summary_for_user.push_str("\nRisque élevé : relis le diff toi-même avant de valider.");
                    }
                    return Ok(Outcome { review, gate });
                }
                gate.unmet.into_iter().map(|u| Fix { lot: u.lot, r#where: u.criterion, problem: u.problem, fix: u.fix, severity: "important".into() }).collect()
            } else {
                review.fixes.into_iter().filter(Fix::blocks).collect()
            };
            if round + 1 == self.max_rounds {
                bail!(
                    "après {} tours la barre n'est toujours pas atteinte : l'approche est sans doute à revoir avec l'utilisateur plutôt que de relancer. Voir {}",
                    self.max_rounds,
                    self.rel(&run.dir)
                );
            }
            let mut by_lot: BTreeMap<String, Vec<Fix>> = BTreeMap::new();
            for f in fixes {
                let lot = if plan.lot(&f.lot).is_some() { f.lot.clone() } else { plan.lots[0].id.clone() };
                by_lot.entry(lot).or_default().push(f);
            }
            if by_lot.is_empty() {
                bail!("renvoi sans correction exploitable");
            }
            self.say(run, Phase::Fixing, round, format!("Corrections : {}", by_lot.keys().cloned().collect::<Vec<_>>().join(", ")));
            let jobs: Vec<(String, Vec<Fix>)> = by_lot.into_iter().collect();
            self.threads.fixes(&run.id, "reviewer", &jobs);
            let done = self.workers(run, &plan, &jobs, round + 1).await?;
            for (_, path) in &done {
                if let Ok(Some(r)) = read_json::<Report>(path) {
                    m.resolve(&r.resolved);
                }
            }
            reports.extend(done);
        }
        unreachable!("loop returns or bails")
    }

    /// Runs the full e2e suite (if configured) and returns a summary for the Reviewer.
    async fn e2e_gate(&self, run: &Run, round: u32) -> Option<String> {
        if !self.run_e2e || !e2e::configured(&self.root) {
            return None;
        }
        self.say(run, Phase::Reviewing, round, "Tests e2e : suite complète (harnes-e2e)");
        let report = e2e::run(&self.root).await;
        let _ = std::fs::write(run.dir.join(format!("e2e-{round}.json")), serde_json::to_string_pretty(&report).unwrap_or_default());
        let note = report.describe();
        self.say(run, Phase::Reviewing, round, note.clone());
        Some(note)
    }

    /// Runs one worker per job in parallel; a failed worker becomes a "blocked" report.
    async fn workers(&self, run: &Run, plan: &Plan, jobs: &[(String, Vec<Fix>)], round: u32) -> Result<Vec<(String, PathBuf)>> {
        let futs = jobs.iter().map(|(id, fixes)| async move {
            let lot = plan.lot(id).ok_or_else(|| anyhow!("unknown lot {id}"))?;
            let path = run.dir.join("reports").join(format!("{id}-r{round}.json"));
            let prompt = prompts::worker(plan, lot, &self.rel(&path), fixes, &crate::threading::lot_thread(&run.id, id));
            let key = format!("worker-{id}-r{round}");
            if let Err(e) = self.agent::<Report>(run, &key, Role::Worker, &prompt, &path).await {
                if *self.cancel.borrow() {
                    return Err(e);
                }
                let r = Report { lot: id.clone(), status: "blocked".into(), files: vec![], tests: String::new(), notes: format!("{e:#}"), e2e: None, resolved: vec![] };
                std::fs::write(&path, serde_json::to_string_pretty(&r)?)?;
            }
            Ok::<_, anyhow::Error>((id.clone(), path))
        });
        join_all(futs).await.into_iter().collect()
    }

    /// Starts one agent and waits until it has written a valid `out` file.
    /// Its model follows the run's mode and the week's pace.
    pub(crate) async fn agent<T: DeserializeOwned>(&self, run: &Run, name: &str, role: Role, prompt: &str, out: &Path) -> Result<T> {
        let tune = budget::tune(budget::effective(self.mode, self.eco), self.eco, role);
        self.agent_tuned(run, name, role, prompt, out, tune).await
    }

    pub(crate) async fn agent_tuned<T: DeserializeOwned>(&self, run: &Run, name: &str, role: Role, prompt: &str, out: &Path, tune: Option<Tune>) -> Result<T> {
        if let Ok(Some(done)) = read_json::<T>(out) {
            return Ok(done); // resumed run: this step is already done
        }
        let key = format!("{}-{name}", run.id);
        let settings = run.dir.join("agents").join(format!("{name}.settings.json"));
        std::fs::write(&settings, serde_json::to_string_pretty(&self.hooks.settings_for(&key))?)?;
        let mut rx = self.hooks.registry.register(&key);
        // Each role gets its method packs (OS) named in its first prompt.
        let prompt = format!("{prompt}{}", harnes_os::role_note(&harnes_os::Library::open_default(), role.agent()));
        let pane = self.host.spawn(&key, role, &prompt, &settings, tune)?;
        if !self.host.has_hooks(role) {
            self.hooks.registry.set(&key, AgentState::Working, false);
        }
        let result = self.wait_output(run, name, &mut rx, out).await;
        self.host.close(&pane);
        self.hooks.registry.forget(&key);
        result
    }

    async fn wait_output<T: DeserializeOwned>(
        &self,
        run: &Run,
        name: &str,
        rx: &mut watch::Receiver<crate::hooks::Turn>,
        out: &Path,
    ) -> Result<T> {
        let mut cancel = self.cancel.clone();
        let deadline = tokio::time::Instant::now() + self.agent_timeout;
        let mut seen_stops = 0;
        let mut warned_blocked = false;
        loop {
            tokio::select! {
                changed = rx.changed() => { changed.map_err(|_| anyhow!("{name} : canal d'état fermé"))?; }
                _ = cancel.changed() => {}
                // Providers without hooks: the output file is the only signal.
                _ = tokio::time::sleep(Duration::from_secs(2)) => {
                    if let Ok(Some(v)) = read_json::<T>(out) {
                        return Ok(v);
                    }
                    continue;
                }
                _ = tokio::time::sleep_until(deadline) => bail!("{name} : délai dépassé"),
            }
            if *cancel.borrow() {
                bail!("run annulé");
            }
            let turn = *rx.borrow();
            match turn.state {
                AgentState::Exited => bail!("{name} s'est arrêté sans écrire {}", self.rel(out)),
                AgentState::Blocked if !warned_blocked => {
                    warned_blocked = true;
                    self.say(run, Phase::Working, 0, format!("{name} attend une réponse dans son pane"));
                }
                AgentState::Working => warned_blocked = false,
                _ => {}
            }
            if turn.stops > seen_stops {
                seen_stops = turn.stops;
                match read_json::<T>(out) {
                    Ok(Some(v)) => return Ok(v),
                    Ok(None) => self.say(run, Phase::Working, 0, format!("{name} a fini son tour sans écrire {} : réponds-lui dans son pane", self.rel(out))),
                    Err(e) => self.say(run, Phase::Working, 0, format!("{name} : {e:#}")),
                }
            }
        }
    }
}

/// `s`, or `fallback` when it is blank.
fn or<'a>(s: &'a str, fallback: &'a str) -> &'a str {
    if s.trim().is_empty() {
        fallback
    } else {
        s
    }
}
