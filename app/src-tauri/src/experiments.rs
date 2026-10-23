//! Repeated eval runs, the rule for keeping a change, and the library of what
//! was tried (see `harnes_eval::passk`).

use crate::state::Shared;
use harnes_eval::passk::{self, Entry, KResult};
use serde_json::{json, Value};
use tauri::State;

fn e(err: impl std::fmt::Display) -> String {
    err.to_string()
}

/// Runs a suite `k` times (1 to 10): pass@1, pass^k and the flaky tasks.
#[tauri::command]
pub async fn eval_run_k(
    st: State<'_, Shared>,
    suite: Option<String>,
    k: Option<u32>,
) -> Result<Value, String> {
    let p = st.active_or_err()?;
    let root = p.root.clone();
    tauri::async_runtime::spawn_blocking(move || -> Result<Value, String> {
        let _ = harnes_eval::ensure_defaults(&root);
        let suite = match suite {
            Some(n) => harnes_eval::load(&root, &n).map_err(e)?,
            None => harnes_eval::list(&root)
                .map_err(e)?
                .into_iter()
                .next()
                .ok_or("aucune suite d'évaluation")?,
        };
        harnes_eval::validate(&suite).map_err(e)?;
        serde_json::to_value(passk::run_k(&root, &suite, k.unwrap_or(3))).map_err(e)
    })
    .await
    .map_err(e)?
}

/// Decides whether a tried change is kept, and writes the verdict with the
/// harness fingerprints to the library. `harness_after` is the fingerprint
/// of the harness the candidate ran with; the current one is the "before".
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn experiment_record(
    st: State<'_, Shared>,
    component: String,
    variant: String,
    model: String,
    baseline: KResult,
    candidate: KResult,
    simpler: bool,
    harness_after: Option<String>,
) -> Result<Value, String> {
    let p = st.active_or_err()?;
    let decision = passk::decide(&baseline, &candidate, simpler);
    let entry = Entry {
        at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
        component,
        variant,
        suite: baseline.suite.clone(),
        model,
        harness_before: harnes_crew::harness::fingerprint(&p.root),
        harness_after: harness_after.unwrap_or_default(),
        baseline,
        candidate,
        keep: decision.keep,
        reason: decision.reason.clone(),
    };
    passk::record(&p.root, &entry).map_err(e)?;
    let _ = p.telemetry.append(
        harnes_telemetry::Event::new("", &p.id, "experiment.recorded")
            .role("user")
            .status(if decision.keep { "ok" } else { "rejected" })
            .summary(format!(
                "{} / {} : {}",
                entry.component, entry.variant, decision.reason
            )),
    );
    Ok(json!({"decision": decision, "entry": entry}))
}

/// Earlier verdicts, newest first.
#[tauri::command]
pub fn experiment_library(
    st: State<'_, Shared>,
    component: Option<String>,
    limit: Option<usize>,
) -> Result<Value, String> {
    let p = st.active_or_err()?;
    serde_json::to_value(passk::library(
        &p.root,
        component.as_deref(),
        limit.unwrap_or(50),
    ))
    .map_err(e)
}
