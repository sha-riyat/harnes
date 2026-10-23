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
    fn spawn(
        &self,
        key: &str,
        role: Role,
        prompt: &str,
        settings: &Path,
        tune: Option<Tune>,
    ) -> Result<String>;
    /// Same, but in an explicit directory (a lot worktree). Hosts that cannot
    /// change directory fall back to `spawn`, which is safe for tests and for
    /// providers whose working directory is fixed.
    fn spawn_in(
        &self,
        key: &str,
        role: Role,
        prompt: &str,
        settings: &Path,
        tune: Option<Tune>,
        cwd: Option<&Path>,
    ) -> Result<String> {
        let _ = cwd;
        self.spawn(key, role, prompt, settings, tune)
    }
    /// Starts the agent on a free OpenCode `model`. The prompt is in
    /// `prompt_file`: it never goes on a command line. Hosts without OpenCode
    /// keep this default, an error, and the engine runs the step on Claude.
    fn spawn_free(
        &self,
        key: &str,
        role: Role,
        prompt_file: &Path,
        settings: &Path,
        model: &str,
        cwd: Option<&Path>,
    ) -> Result<String> {
        let _ = (key, role, prompt_file, settings, model, cwd);
        bail!("aucun hôte OpenCode")
    }
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
    /// Project id used in telemetry events.
    pub project: String,
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
    /// The week runs ahead of its allowance: skip deep reviews.
    pub eco: bool,
    /// Unified event log. None keeps the crew usable in tests and headless.
    pub telemetry: Option<Arc<harnes_telemetry::Telemetry>>,
    /// Review-gated project memory. None disables harvesting/recall.
    pub memory: Option<Arc<harnes_memory::Memory>>,
    /// Per-lot git worktrees. None or inactive disables isolation.
    pub worktrees: Option<harnes_worktree::Worktrees>,
}

pub(crate) struct Run {
    pub(crate) id: String,
    pub(crate) dir: PathBuf,
    pub(crate) task: String,
    /// A small change: one worker, a short review.
    pub(crate) light: bool,
    /// The Teacher rated the run high risk (set once the bar is known).
    pub(crate) high_risk: std::sync::atomic::AtomicBool,
}

/// What a successful run hands to the user.
#[derive(Debug, Clone)]
pub struct Outcome {
    pub review: Review,
    pub gate: Gate,
}

impl<H: AgentHost> Engine<H> {
    pub(crate) fn event(&self, _run: &Run, event: harnes_telemetry::Event) {
        if let Some(t) = &self.telemetry {
            let mut event = event;
            if event.project.is_empty() {
                event.project = self.project.clone();
            }
            let _ = t.append(event);
        }
    }

    pub(crate) fn say(&self, run: &Run, phase: Phase, round: u32, message: impl Into<String>) {
        let message = message.into();
        self.threads.event(&run.id, &message);
        self.event(
            run,
            harnes_telemetry::Event::new(&run.id, "", "phase.changed")
                .status(format!("{:?}", phase).to_lowercase())
                .summary(message.clone())
                .detail(serde_json::json!({"phase": format!("{:?}", phase).to_lowercase(), "round": round})),
        );
        (self.emit)(CrewEvent {
            run: run.id.clone(),
            phase,
            round,
            message,
            review: None,
            gate: None,
        });
    }

    pub(crate) fn rel(&self, p: &Path) -> String {
        p.strip_prefix(&self.root)
            .unwrap_or(p)
            .display()
            .to_string()
            .replace('\\', "/")
    }

    pub async fn run(&self, task: &str) -> Result<Outcome> {
        self.run_resumable(task, None).await
    }

    /// Runs a task, or resumes run `id`: every agent whose output file already
    /// exists is skipped, so an interrupted run continues where it stopped.
    pub async fn run_resumable(&self, task: &str, id: Option<String>) -> Result<Outcome> {
        let id = id.unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
                .to_string()
        });
        let run = Run {
            dir: setup::run_dir(&self.root, &id),
            id,
            task: task.to_string(),
            light: budget::effective(self.mode, self.eco) == Mode::Light,
            high_risk: Default::default(),
        };
        std::fs::create_dir_all(run.dir.join("reports"))?;
        std::fs::create_dir_all(run.dir.join("agents"))?;
        let _ = std::fs::write(run.dir.join("task.txt"), task);
        self.threads.open_run(&run.id, task);
        self.event(
            &run,
            harnes_telemetry::Event::new(&run.id, "", "run.started")
                .summary(task.to_string())
                .detail(serde_json::json!({"mode": format!("{:?}", self.mode).to_lowercase(), "eco": self.eco})),
        );
        let mut metrics = Metrics::new(budget::effective(self.mode, self.eco), self.eco);
        // A resumed run must keep the harness it started with: otherwise its
        // numbers mix two. Say so, and mark the metrics so comparisons skip it.
        metrics.harness = crate::harness::fingerprint(&self.root);
        let fp_file = run.dir.join("harness.fp");
        match std::fs::read_to_string(&fp_file) {
            Ok(before) if before.trim() != metrics.harness => {
                metrics.harness_changed = true;
                self.say(
                    &run,
                    Phase::Working,
                    0,
                    format!("Le harnais a changé depuis le début de ce run ({} → {}) : ses mesures ne sont plus comparables.", before.trim(), metrics.harness),
                );
                self.event(
                    &run,
                    harnes_telemetry::Event::new(&run.id, "", "run.harness.changed")
                        .status("warn")
                        .summary(format!("{} → {}", before.trim(), metrics.harness)),
                );
            }
            Ok(_) => {}
            Err(_) => {
                let _ = std::fs::write(&fp_file, &metrics.harness);
            }
        }
        let result = self.run_inner(&run, &mut metrics).await;
        metrics.passed = result.is_ok();
        let _ = std::fs::write(
            run.dir.join("metrics.json"),
            serde_json::to_string_pretty(&metrics).unwrap_or_default(),
        );
        if let Some(memory) = &self.memory {
            match memory.harvest_run(&self.root, &run.id, self.telemetry.as_deref()) {
                Ok(Some(c)) => {
                    self.event(
                        &run,
                        harnes_telemetry::Event::new(&run.id, "", "memory.candidate")
                            .summary(format!("candidat mémoire #{} : {}", c.id, c.title))
                            .detail(serde_json::json!({"candidate": c.id, "confidence": c.confidence, "source": c.source})),
                    );
                }
                Ok(None) => {}
                Err(e) => {
                    self.event(
                        &run,
                        harnes_telemetry::Event::new(&run.id, "", "error")
                            .status("error")
                            .summary(format!("mémoire : {e:#}")),
                    );
                }
            }
        }
        match result {
            Ok(out) => {
                self.threads.close_run(&run.id, &out.gate.summary_for_user);
                self.event(
                    &run,
                    harnes_telemetry::Event::new(&run.id, "", "run.completed")
                        .status("ok")
                        .summary(out.gate.summary_for_user.clone())
                        .detail(serde_json::json!({"review_passed": out.review.passed(), "gate_ready": out.gate.ready})),
                );
                let msg = format!("Prêt pour toi : {}", out.gate.summary_for_user);
                (self.emit)(CrewEvent {
                    run: run.id.clone(),
                    phase: Phase::Done,
                    round: 0,
                    message: msg,
                    review: Some(out.review.clone()),
                    gate: Some(out.gate.clone()),
                });
                Ok(out)
            }
            Err(e) => {
                let phase = if *self.cancel.borrow() {
                    Phase::Cancelled
                } else {
                    Phase::Failed
                };
                self.event(
                    &run,
                    harnes_telemetry::Event::new(
                        &run.id,
                        "",
                        if phase == Phase::Cancelled {
                            "run.cancelled"
                        } else {
                            "run.failed"
                        },
                    )
                    .status("error")
                    .summary(format!("{e:#}")),
                );
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
            self.say(
                run,
                Phase::Planning,
                0,
                "Run léger : un seul worker, sans planner ni Teacher séparé",
            );
            let plan = Plan::single(&run.task);
            if !plan_path.exists() {
                std::fs::write(&plan_path, serde_json::to_string_pretty(&plan)?)?;
            }
            plan
        } else {
            self.say(run, Phase::Planning, 0, "Planner au travail");
            let prompt = prompts::planner(&run.task, &self.rel(&plan_path))
                + &prompts::run_thread_note(&run.id);
            self.agent(run, "planner", Role::Planner, &prompt, &plan_path)
                .await?
        };
        plan.validate()?;
        self.threads.open_lots(&run.id, &plan);

        let bar_path = run.dir.join("bar.json");
        let bar: Option<Bar> = if mode == Mode::Light {
            None
        } else {
            self.say(
                run,
                Phase::Calibrating,
                0,
                "Teacher fixe la barre de validation",
            );
            let prompt = prompts::teacher_bar(
                &run.task,
                &self.rel(&plan_path),
                &self.rel(&bar_path),
                e2e::configured(&self.root),
            );
            let bar: Bar = self
                .agent(run, "teacher-bar", Role::Teacher, &prompt, &bar_path)
                .await?;
            self.say(
                run,
                Phase::Calibrating,
                0,
                format!(
                    "Barre : {} critère(s), risque {}. Te déranger quand : {}",
                    bar.criteria.len(),
                    or(&bar.risk, "medium"),
                    bar.notify_user_when
                ),
            );
            Some(bar)
        };
        let high = bar
            .as_ref()
            .is_some_and(|b| b.risk.eq_ignore_ascii_case("high"));
        run.high_risk
            .store(high, std::sync::atomic::Ordering::Relaxed);
        // A risky run gets the deep review, unless the week is short of allowance.
        let deep = mode == Mode::Deep || (high && !self.eco);

        let waves = plan.effective_waves();
        self.say(
            run,
            Phase::Working,
            0,
            format!("{} lots en {} vague(s)", plan.lots.len(), waves.len()),
        );
        let mut reports: BTreeMap<String, PathBuf> = BTreeMap::new();
        for (i, wave) in waves.iter().enumerate() {
            self.say(
                run,
                Phase::Working,
                0,
                format!("Vague {} : {}", i + 1, wave.join(", ")),
            );
            let jobs: Vec<(String, Vec<Fix>)> =
                wave.iter().map(|id| (id.clone(), vec![])).collect();
            reports.extend(self.workers(run, &plan, &jobs, 0).await?);
        }

        for round in 0..self.max_rounds {
            m.rounds = round + 1;
            self.say(
                run,
                Phase::Reviewing,
                round,
                format!(
                    "Revue{}, tour {}",
                    if deep { " renforcée" } else { "" },
                    round + 1
                ),
            );
            let e2e_note = self.e2e_gate(run, round).await;
            let list: Vec<String> = reports.values().map(|p| self.rel(p)).collect();
            let previous: Vec<String> = (0..round)
                .map(|r| self.rel(&run.dir.join(format!("review-{r}.json"))))
                .collect();
            let review = self
                .review(
                    run,
                    &plan,
                    &self.rel(&plan_path),
                    round,
                    &list,
                    &previous,
                    e2e_note.as_deref(),
                    deep,
                )
                .await?;
            m.count(&review.fixes);
            let old: Vec<String> = review
                .fixes
                .iter()
                .filter(|f| f.pre_existing())
                .map(|f| format!("{} : {}", f.r#where, f.problem))
                .collect();
            if !old.is_empty() {
                self.threads.event(
                    &run.id,
                    &format!(
                        "Déjà là avant ce run, non bloquant :\n- {}",
                        old.join("\n- ")
                    ),
                );
            }

            let fixes: Vec<Fix> = if review.passed() {
                let Some(bar_file) = bar.as_ref().map(|_| self.rel(&bar_path)) else {
                    let summary = or(&review.summary, "Revue PASS.").to_string();
                    return Ok(Outcome {
                        review,
                        gate: Gate {
                            ready: true,
                            met: vec![],
                            unmet: vec![],
                            summary_for_user: summary,
                        },
                    });
                };
                self.say(
                    run,
                    Phase::Gating,
                    round,
                    "Teacher vérifie la barre avant de te solliciter",
                );
                let review_path = run.dir.join(format!("review-{round}.json"));
                let gate_path = run.dir.join(format!("gate-{round}.json"));
                let prompt = prompts::teacher_gate(
                    &run.task,
                    &bar_file,
                    &self.rel(&review_path),
                    e2e_note.as_deref(),
                    &self.rel(&gate_path),
                );
                let mut gate: Gate = self
                    .agent(
                        run,
                        &format!("teacher-gate-{round}"),
                        Role::Teacher,
                        &prompt,
                        &gate_path,
                    )
                    .await?;
                if gate.ready {
                    if high {
                        gate.summary_for_user
                            .push_str("\nRisque élevé : relis le diff toi-même avant de valider.");
                    }
                    return Ok(Outcome { review, gate });
                }
                gate.unmet
                    .into_iter()
                    .map(|u| Fix {
                        lot: u.lot,
                        r#where: u.criterion,
                        problem: u.problem,
                        fix: u.fix,
                        severity: "important".into(),
                    })
                    .collect()
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
                let lot = if plan.lot(&f.lot).is_some() {
                    f.lot.clone()
                } else {
                    plan.lots[0].id.clone()
                };
                by_lot.entry(lot).or_default().push(f);
            }
            if by_lot.is_empty() {
                bail!("renvoi sans correction exploitable");
            }
            self.say(
                run,
                Phase::Fixing,
                round,
                format!(
                    "Corrections : {}",
                    by_lot.keys().cloned().collect::<Vec<_>>().join(", ")
                ),
            );
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
        self.say(
            run,
            Phase::Reviewing,
            round,
            "Tests e2e : suite complète (harnes-e2e)",
        );
        let report = e2e::run(&self.root).await;
        let _ = std::fs::write(
            run.dir.join(format!("e2e-{round}.json")),
            serde_json::to_string_pretty(&report).unwrap_or_default(),
        );
        let note = report.describe();
        self.say(run, Phase::Reviewing, round, note.clone());
        Some(note)
    }

    /// Runs one worker per job in parallel; a failed worker becomes a "blocked" report.
    async fn workers(
        &self,
        run: &Run,
        plan: &Plan,
        jobs: &[(String, Vec<Fix>)],
        round: u32,
    ) -> Result<Vec<(String, PathBuf)>> {
        let futs = jobs.iter().map(|(id, fixes)| async move {
            let lot = plan.lot(id).ok_or_else(|| anyhow!("unknown lot {id}"))?;
            let path = run.dir.join("reports").join(format!("{id}-r{round}.json"));
            let key = format!("worker-{id}-r{round}");
            // Per-lot worktree: the worker edits an isolated branch. The report
            // stays in the run directory (absolute, outside the worktree).
            let isolation = match self.worktrees.as_ref().filter(|w| w.active()) {
                Some(w) => match w.create(&run.id, id) {
                    Ok(wt) => {
                        if let Err(e) = w.prepare_for_agent(&wt) {
                            self.event(
                                run,
                                harnes_telemetry::Event::new(&run.id, "", "worktree.failed")
                                    .lot(id.clone())
                                    .status("error")
                                    .summary(format!("{e:#}")),
                            );
                            return Err(e);
                        }
                        self.event(
                            run,
                            harnes_telemetry::Event::new(&run.id, "", "worktree.created")
                                .lot(id.clone())
                                .path(wt.path.display().to_string())
                                .summary(format!("branche {}", wt.branch)),
                        );
                        Some((w.clone(), wt))
                    }
                    Err(e) => {
                        self.event(
                            run,
                            harnes_telemetry::Event::new(&run.id, "", "worktree.failed")
                                .lot(id.clone())
                                .status("error")
                                .summary(format!("{e:#}")),
                        );
                        return Err(e);
                    }
                },
                None => None,
            };
            let out_arg = if isolation.is_some() {
                path.display().to_string().replace('\\', "/")
            } else {
                self.rel(&path)
            };
            let prompt = prompts::worker(
                plan,
                lot,
                &out_arg,
                fixes,
                &crate::threading::lot_thread(&run.id, id),
            );
            let cwd = isolation.as_ref().map(|(_, wt)| wt.path.as_path());
            let task = harnes_free::Task {
                kind: harnes_free::Kind::Worker,
                round,
                light: run.light,
                files: &lot.files,
                isolated: isolation.is_some(),
                high_risk: run.high_risk.load(std::sync::atomic::Ordering::Relaxed),
            };
            let result = self
                .agent_routed::<Report>(run, &key, Role::Worker, &prompt, &path, cwd, &task)
                .await;
            if let Some((w, wt)) = &isolation {
                match &result {
                    Ok(_) => {
                        let msg = format!("harnes: run {} lot {id} (tour {round})", run.id);
                        if let Err(e) = w.finish(wt, &msg) {
                            self.event(
                                run,
                                harnes_telemetry::Event::new(&run.id, "", "worktree.conflict")
                                    .lot(id.clone())
                                    .status("error")
                                    .summary(format!("{e:#}")),
                            );
                            return Err(e);
                        }
                        self.event(
                            run,
                            harnes_telemetry::Event::new(&run.id, "", "worktree.merged")
                                .lot(id.clone())
                                .summary(format!("branche {} fusionnée", wt.branch)),
                        );
                    }
                    Err(e) => {
                        // Keep the result for inspection, but do not merge a
                        // cancelled or broken worker's partial edits.
                        self.event(
                            run,
                            harnes_telemetry::Event::new(&run.id, "", "worktree.kept")
                                .lot(id.clone())
                                .status("error")
                                .summary(format!("{e:#}")),
                        );
                    }
                }
            }
            if let Err(e) = result {
                if *self.cancel.borrow() {
                    return Err(e);
                }
                let r = Report {
                    lot: id.clone(),
                    status: "blocked".into(),
                    files: vec![],
                    tests: String::new(),
                    notes: format!("{e:#}"),
                    e2e: None,
                    resolved: vec![],
                };
                std::fs::write(&path, serde_json::to_string_pretty(&r)?)?;
            }
            Ok::<_, anyhow::Error>((id.clone(), path))
        });
        join_all(futs).await.into_iter().collect()
    }

    /// Starts one agent and waits until it has written a valid `out` file.
    /// The Planner alone gets Opus; every other role gets Sonnet high.
    pub(crate) async fn agent<T: DeserializeOwned>(
        &self,
        run: &Run,
        name: &str,
        role: Role,
        prompt: &str,
        out: &Path,
    ) -> Result<T> {
        let tune = Some(budget::tune(role));
        self.agent_tuned(run, name, role, prompt, out, tune).await
    }

    pub(crate) async fn agent_tuned<T: DeserializeOwned>(
        &self,
        run: &Run,
        name: &str,
        role: Role,
        prompt: &str,
        out: &Path,
        tune: Option<Tune>,
    ) -> Result<T> {
        self.agent_tuned_at(run, name, role, prompt, out, tune, None, None)
            .await
    }

    /// One agent, on Claude (`free` is None) or on a free OpenCode model.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn agent_tuned_at<T: DeserializeOwned>(
        &self,
        run: &Run,
        name: &str,
        role: Role,
        prompt: &str,
        out: &Path,
        tune: Option<Tune>,
        cwd: Option<&Path>,
        free: Option<&str>,
    ) -> Result<T> {
        if let Ok(Some(done)) = read_json::<T>(out) {
            return Ok(done); // resumed run: this step is already done
        }
        let key = format!("{}-{name}", run.id);
        let settings = run.dir.join("agents").join(format!("{name}.settings.json"));
        std::fs::write(
            &settings,
            serde_json::to_string_pretty(&self.hooks.settings_for_root(&key, Some(&self.root)))?,
        )?;
        let mut rx = self.hooks.registry.register(&key);
        // The always-on context is paid by every agent at every start.
        if let Ok(spec) = crate::harness::HarnessSpec::load(&self.root) {
            let used = crate::harness::always_on_tokens(&self.root, role.agent());
            if used > spec.budgets.always_on_tokens {
                self.event(
                    run,
                    harnes_telemetry::Event::new(&run.id, "", "budget.exceeded")
                        .role(role.agent())
                        .status("warn")
                        .summary(format!(
                            "{name} charge ~{used} tokens avant de commencer (budget {})",
                            spec.budgets.always_on_tokens
                        )),
                );
            }
        }
        // Each role gets its method packs (OS) named in its first prompt.
        let prompt = format!(
            "{prompt}{}",
            harnes_os::role_note(&harnes_os::Library::open_default(), role.agent())
        );
        self.event(
            run,
            harnes_telemetry::Event::new(&run.id, "", "agent.started")
                .role(role.agent())
                .status("running")
                .summary(format!("{name} démarre"))
                .detail(serde_json::json!({"cwd": cwd.map(|c| c.display().to_string()), "free_model": free, "model": free.map(String::from).or(tune.map(|t| t.model.to_string()))})),
        );
        let spawned = match free {
            Some(model) => {
                // The prompt goes in a file: it can hold any character, and it
                // never travels on a command line.
                let file = run.dir.join("agents").join(format!("{name}.prompt.md"));
                std::fs::write(&file, &prompt)
                    .map_err(anyhow::Error::from)
                    .and_then(|_| {
                        self.host
                            .spawn_free(&key, role, &file, &settings, model, cwd)
                    })
            }
            None => self
                .host
                .spawn_in(&key, role, &prompt, &settings, tune, cwd),
        };
        let pane = match spawned {
            Ok(p) => p,
            Err(e) => {
                self.event(
                    run,
                    harnes_telemetry::Event::new(&run.id, "", "agent.failed")
                        .role(role.agent())
                        .status("error")
                        .summary(format!("{name} : {e:#}")),
                );
                return Err(e);
            }
        };
        // OpenCode reports nothing through hooks, so its output file is the
        // only signal; its time is also shorter: a free try that has not
        // answered in a quarter of an hour is not worth waiting for.
        let hooks = free.is_none() && self.host.has_hooks(role);
        if !hooks {
            self.hooks.registry.set(&key, AgentState::Working, false);
        }
        let limit = if free.is_some() {
            self.agent_timeout.min(FREE_TIMEOUT)
        } else {
            self.agent_timeout
        };
        let result = self
            .wait_output(run, name, &mut rx, out, hooks, limit)
            .await;
        self.host.close(&pane);
        self.hooks.registry.forget(&key);
        match &result {
            Ok(_) => self.event(
                run,
                harnes_telemetry::Event::new(&run.id, "", "agent.completed")
                    .role(role.agent())
                    .status("ok")
                    .summary(format!("{name} a écrit son rapport")),
            ),
            Err(e) => self.event(
                run,
                harnes_telemetry::Event::new(&run.id, "", "agent.failed")
                    .role(role.agent())
                    .status("error")
                    .summary(format!("{name} : {e:#}")),
            ),
        }
        result
    }

    async fn wait_output<T: DeserializeOwned>(
        &self,
        run: &Run,
        name: &str,
        rx: &mut watch::Receiver<crate::hooks::Turn>,
        out: &Path,
        hooks: bool,
        limit: Duration,
    ) -> Result<T> {
        let mut cancel = self.cancel.clone();
        let deadline = tokio::time::Instant::now() + limit;
        // Heartbeat: every hook event is a sign of life. An agent that is neither
        // waiting for the user nor sending any is stuck (a hung tool, a dead
        // session): quarantine it instead of waiting out the whole timeout.
        // Providers without hooks send nothing, so they are only timed out.
        let stall = (limit / 3).min(Duration::from_secs(20 * 60));
        let mut last_beat = tokio::time::Instant::now();
        let mut seen_stops = 0;
        let mut warned_blocked = false;
        loop {
            tokio::select! {
                changed = rx.changed() => { changed.map_err(|_| anyhow!("{name} : canal d'état fermé"))?; last_beat = tokio::time::Instant::now(); }
                _ = cancel.changed() => {}
                // Providers without hooks: the output file is the only signal.
                _ = tokio::time::sleep(Duration::from_secs(2)) => {
                    if let Ok(Some(v)) = read_json::<T>(out) {
                        return Ok(v);
                    }
                    if hooks && rx.borrow().state == AgentState::Working && last_beat.elapsed() > stall {
                        bail!("{name} est figé : aucun signe de vie depuis {} min, il est arrêté", stall.as_secs() / 60);
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
                    self.say(
                        run,
                        Phase::Working,
                        0,
                        format!("{name} attend une réponse dans son pane"),
                    );
                }
                AgentState::Working => warned_blocked = false,
                _ => {}
            }
            if turn.stops > seen_stops {
                seen_stops = turn.stops;
                match read_json::<T>(out) {
                    Ok(Some(v)) => return Ok(v),
                    Ok(None) => self.say(
                        run,
                        Phase::Working,
                        0,
                        format!(
                            "{name} a fini son tour sans écrire {} : réponds-lui dans son pane",
                            self.rel(out)
                        ),
                    ),
                    Err(e) => self.say(run, Phase::Working, 0, format!("{name} : {e:#}")),
                }
            }
        }
    }
}

/// How long a free-model attempt may take before the step moves to Claude.
pub(crate) const FREE_TIMEOUT: Duration = Duration::from_secs(15 * 60);

/// `s`, or `fallback` when it is blank.
fn or<'a>(s: &'a str, fallback: &'a str) -> &'a str {
    if s.trim().is_empty() {
        fallback
    } else {
        s
    }
}
