//! Repeated runs and the rule for keeping a change.
//!
//! One run of a suite says little: a flaky check passes one time in two.
//! Running it `k` times gives pass@1 (the average) and pass^k (all `k` pass),
//! and names the flaky tasks. `decide` is Harness2's discipline in one rule:
//! a change to the harness is kept only if it wins, or ties and is simpler,
//! and never if a task that always passed now fails. `record` writes each
//! verdict, with the harness fingerprint and the numbers behind it, to an
//! experience library that later tuning reads before it repeats a try.

use crate::{run_suite, Suite};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};

pub const MAX_K: u32 = 10;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct KResult {
    pub suite: String,
    pub k: u32,
    /// Times each task passed out of `k`.
    pub passes: BTreeMap<String, u32>,
    /// Average pass rate over every run of every task.
    pub pass_at_1: f64,
    /// Share of tasks that passed all `k` runs.
    pub pass_all_k: f64,
    /// Tasks that passed sometimes but not always.
    pub flaky: Vec<String>,
    pub duration_ms: u64,
}

/// Runs `suite` `k` times (1 to 10) and summarises it.
pub fn run_k(root: &Path, suite: &Suite, k: u32) -> KResult {
    let k = k.clamp(1, MAX_K);
    let mut passes: BTreeMap<String, u32> = suite.tasks.iter().map(|t| (t.id.clone(), 0)).collect();
    let mut ms = 0;
    for _ in 0..k {
        let r = run_suite(root, suite);
        ms += r.duration_ms;
        for t in r.tasks.iter().filter(|t| t.passed) {
            *passes.entry(t.id.clone()).or_default() += 1;
        }
    }
    summarise(&suite.name, k, passes, ms)
}

pub fn summarise(suite: &str, k: u32, passes: BTreeMap<String, u32>, duration_ms: u64) -> KResult {
    let tasks = passes.len().max(1) as f64;
    let total: u32 = passes.values().sum();
    KResult {
        suite: suite.into(),
        k,
        pass_at_1: total as f64 / (tasks * k as f64),
        pass_all_k: passes.values().filter(|&&p| p == k).count() as f64 / tasks,
        flaky: passes
            .iter()
            .filter(|(_, &p)| p > 0 && p < k)
            .map(|(id, _)| id.clone())
            .collect(),
        passes,
        duration_ms,
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Decision {
    pub keep: bool,
    pub delta: f64,
    pub reason: String,
}

/// Keep a change only if it wins, or ties and is simpler; never if it breaks
/// a task that always passed. Both results must be of the same suite and `k`.
pub fn decide(baseline: &KResult, candidate: &KResult, simpler: bool) -> Decision {
    let delta = candidate.pass_at_1 - baseline.pass_at_1;
    if baseline.suite != candidate.suite || baseline.k != candidate.k {
        return Decision {
            keep: false,
            delta,
            reason: "résultats non comparables : même suite et même k requis".into(),
        };
    }
    if let Some((id, _)) = baseline.passes.iter().find(|(id, &p)| {
        p == baseline.k && candidate.passes.get(*id).copied().unwrap_or(0) < candidate.k
    }) {
        return Decision {
            keep: false,
            delta,
            reason: format!("régression : `{id}` passait à tous les tours et ne passe plus"),
        };
    }
    if delta > 1e-9 {
        Decision {
            keep: true,
            delta,
            reason: format!("meilleur de {:.0} points", delta * 100.0),
        }
    } else if delta > -1e-9 && simpler {
        Decision {
            keep: true,
            delta,
            reason: "à égalité et plus simple".into(),
        }
    } else if delta > -1e-9 {
        Decision {
            keep: false,
            delta,
            reason: "à égalité sans être plus simple : rien à gagner".into(),
        }
    } else {
        Decision {
            keep: false,
            delta,
            reason: format!("moins bon de {:.0} points", -delta * 100.0),
        }
    }
}

/// One tried change and what came of it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub at: u64,
    /// The component varied: `system_prompt`, `skills`, `budgets`…
    pub component: String,
    pub variant: String,
    pub suite: String,
    pub model: String,
    /// `harness::fingerprint` before and after.
    pub harness_before: String,
    pub harness_after: String,
    pub baseline: KResult,
    pub candidate: KResult,
    pub keep: bool,
    pub reason: String,
}

fn library_path(root: &Path) -> std::path::PathBuf {
    root.join(".harnes")
        .join("experiments")
        .join("library.jsonl")
}

/// Appends one verdict to the library (append-only: history is not rewritten).
pub fn record(root: &Path, entry: &Entry) -> Result<()> {
    use std::io::Write;
    let path = library_path(root);
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(f, "{}", serde_json::to_string(entry)?)?;
    Ok(())
}

/// Earlier verdicts, newest first, optionally for one component.
pub fn library(root: &Path, component: Option<&str>, limit: usize) -> Vec<Entry> {
    let text = std::fs::read_to_string(library_path(root)).unwrap_or_default();
    let mut all: Vec<Entry> = text
        .lines()
        .filter_map(|l| serde_json::from_str(l).ok())
        .filter(|e: &Entry| component.map_or(true, |c| e.component == c))
        .collect();
    all.reverse();
    all.truncate(limit);
    all
}

/// Whether this exact change was already tried and rejected on this model, so
/// a sweep does not pay for it twice.
pub fn already_rejected(root: &Path, component: &str, variant: &str, model: &str) -> Option<Entry> {
    library(root, Some(component), 200)
        .into_iter()
        .find(|e| e.variant == variant && e.model == model && !e.keep)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(suite: &str, kk: u32, passes: &[(&str, u32)]) -> KResult {
        summarise(
            suite,
            kk,
            passes.iter().map(|(a, b)| (a.to_string(), *b)).collect(),
            10,
        )
    }

    #[test]
    fn pass_at_1_pass_all_k_and_flaky_tasks() {
        let r = k("s", 4, &[("a", 4), ("b", 2), ("c", 0)]);
        assert!((r.pass_at_1 - 0.5).abs() < 1e-9);
        assert!((r.pass_all_k - 1.0 / 3.0).abs() < 1e-9);
        assert_eq!(r.flaky, ["b"]);
    }

    #[test]
    fn a_change_is_kept_only_if_it_wins_or_ties_simpler_and_breaks_nothing() {
        let base = k("s", 3, &[("a", 3), ("b", 1)]);
        assert!(
            decide(&base, &k("s", 3, &[("a", 3), ("b", 3)]), false).keep,
            "wins"
        );
        assert!(
            decide(&base, &k("s", 3, &[("a", 3), ("b", 1)]), true).keep,
            "ties, simpler"
        );
        assert!(
            !decide(&base, &k("s", 3, &[("a", 3), ("b", 1)]), false).keep,
            "ties, not simpler"
        );
        assert!(
            !decide(&base, &k("s", 3, &[("a", 3), ("b", 0)]), true).keep,
            "worse"
        );
        // Better on average, but `a` always passed before and no longer does.
        let d = decide(&base, &k("s", 3, &[("a", 2), ("b", 3)]), true);
        assert!(!d.keep && d.reason.contains("`a`"), "{d:?}");
        assert!(
            !decide(&base, &k("other", 3, &[("a", 3)]), true).keep,
            "different suite"
        );
    }

    #[test]
    fn runs_a_real_suite_k_times_and_remembers_verdicts() {
        let root = std::env::temp_dir().join(format!("harnes-passk-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let suite: Suite = serde_yaml::from_str(&format!(
            "name: tiny\ntasks:\n  - id: ok\n    title: ok\n    prompt: p\n    checks:\n      - name: true\n        command: \"{cmd}\"\n        expect_exit: 0\n",
            cmd = if cfg!(windows) { "exit 0" } else { "true" }
        ))
        .unwrap();
        let r = run_k(&root, &suite, 2);
        assert_eq!((r.k, r.passes["ok"], r.pass_all_k), (2, 2, 1.0));
        let entry = Entry {
            at: 1,
            component: "budgets".into(),
            variant: "context-tight".into(),
            suite: "tiny".into(),
            model: "sonnet".into(),
            harness_before: "aa".into(),
            harness_after: "bb".into(),
            baseline: r.clone(),
            candidate: k("tiny", 2, &[("ok", 1)]),
            keep: false,
            reason: "moins bon".into(),
        };
        record(&root, &entry).unwrap();
        assert!(already_rejected(&root, "budgets", "context-tight", "sonnet").is_some());
        assert!(already_rejected(&root, "budgets", "context-tight", "opus").is_none());
        assert_eq!(library(&root, None, 10).len(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }
}
