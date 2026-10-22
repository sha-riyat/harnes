//! The review step, and what a run measures about it. A normal review is one
//! Sonnet high reviewer; a deep one has several look independently, then a
//! Sonnet high verifier keeps only what they agree on or what it confirms.

use crate::{
    budget::{self, Mode},
    engine::{AgentHost, Engine, Run},
    plan::{Fix, Plan, Resolution, Review},
    prompts, Role,
};
use anyhow::{bail, Result};
use futures_util::future::join_all;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};

/// What a run leaves in `metrics.json`: rounds to pass, findings, how fixes ended.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metrics {
    pub mode: Mode,
    pub eco: bool,
    pub rounds: u32,
    pub passed: bool,
    pub important: u32,
    pub nits: u32,
    pub pre_existing: u32,
    /// fixed / wrong / intentional / irrelevant, as the workers answered.
    pub resolved: BTreeMap<String, u32>,
}

impl Metrics {
    pub fn new(mode: Mode, eco: bool) -> Self {
        Self {
            mode,
            eco,
            rounds: 0,
            passed: false,
            important: 0,
            nits: 0,
            pre_existing: 0,
            resolved: BTreeMap::new(),
        }
    }

    pub fn count(&mut self, fixes: &[Fix]) {
        for f in fixes {
            match (f.blocks(), f.pre_existing()) {
                (true, _) => self.important += 1,
                (false, true) => self.pre_existing += 1,
                _ => self.nits += 1,
            }
        }
    }

    pub fn resolve(&mut self, answers: &[Resolution]) {
        for r in answers {
            *self
                .resolved
                .entry(r.outcome.trim().to_lowercase())
                .or_default() += 1;
        }
    }
}

/// Every run of a project, summed: what the review loop costs and how it lands.
#[derive(Debug, Default, Serialize)]
pub struct History {
    pub runs: u32,
    pub passed: u32,
    /// Runs that passed on their first review.
    pub first_pass: u32,
    pub mean_rounds: f64,
    pub important: u32,
    pub nits: u32,
    pub pre_existing: u32,
    pub resolved: BTreeMap<String, u32>,
}

pub fn history(root: &Path) -> History {
    let mut h = History::default();
    let Ok(dirs) = std::fs::read_dir(root.join(".harnes").join("runs")) else {
        return h;
    };
    let mut rounds = 0;
    for m in dirs.flatten().filter_map(|d| {
        crate::plan::read_json::<Metrics>(&d.path().join("metrics.json"))
            .ok()
            .flatten()
    }) {
        h.runs += 1;
        h.passed += m.passed as u32;
        h.first_pass += (m.passed && m.rounds == 1) as u32;
        rounds += m.rounds;
        (h.important, h.nits, h.pre_existing) = (
            h.important + m.important,
            h.nits + m.nits,
            h.pre_existing + m.pre_existing,
        );
        for (k, v) in m.resolved {
            *h.resolved.entry(k).or_default() += v;
        }
    }
    if h.runs > 0 {
        h.mean_rounds = rounds as f64 / h.runs as f64;
    }
    h
}

impl<H: AgentHost> Engine<H> {
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn review(
        &self,
        run: &Run,
        plan: &Plan,
        plan_path: &str,
        round: u32,
        reports: &[String],
        previous: &[String],
        e2e: Option<&str>,
        deep: bool,
    ) -> Result<Review> {
        let path = run.dir.join(format!("review-{round}.json"));
        let notes = prompts::threads_note(&run.id, plan);
        let prompt = |out: &str| {
            prompts::reviewer(&run.task, plan_path, reports, out, round, e2e, previous) + &notes
        };
        if !deep {
            return self
                .agent(
                    run,
                    &format!("reviewer-{round}"),
                    Role::Reviewer,
                    &prompt(&self.rel(&path)),
                    &path,
                )
                .await;
        }
        let finders = (0..budget::FINDERS).map(|k| {
            let out = run.dir.join(format!("review-{round}-f{k}.json"));
            let p = prompt(&self.rel(&out));
            async move {
                self.agent_tuned::<Review>(
                    run,
                    &format!("reviewer-{round}-f{k}"),
                    Role::Reviewer,
                    &p,
                    &out,
                    Some(budget::tune(Role::Reviewer)),
                )
                .await
                .map(|_| self.rel(&out))
            }
        });
        let found: Vec<String> = join_all(finders)
            .await
            .into_iter()
            .filter_map(Result::ok)
            .collect();
        if found.is_empty() {
            bail!("revue renforcée : aucun relecteur n'a rendu de verdict");
        }
        let prompt = prompts::verifier(&run.task, &found, &self.rel(&path)) + &notes;
        // The verifier follows the same policy as every reviewer: Sonnet high.
        let tune = Some(budget::tune(Role::Reviewer));
        self.agent_tuned(
            run,
            &format!("reviewer-{round}-verify"),
            Role::Reviewer,
            &prompt,
            &path,
            tune,
        )
        .await
    }
}
