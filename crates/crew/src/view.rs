//! Typed run views (`harnes.view.v1`).
//!
//! The cockpit never renders model-generated HTML. It asks for a schema:
//! plan, reviews, gate, metrics, a compact trace, usage, findings, memory
//! candidates and decisions. Each field comes from a file or an event, so the
//! same view can be audited later.

use crate::{
    plan::{read_json, Gate, Plan, Review},
    review::Metrics,
};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub const SCHEMA: &str = "harnes.view.v1";

#[derive(Debug, Clone, Serialize)]
pub struct TraceItem {
    pub id: String,
    pub at: u64,
    pub kind: String,
    pub role: String,
    pub lot: Option<String>,
    pub tool: Option<String>,
    pub path: Option<String>,
    pub command: Option<String>,
    pub status: Option<String>,
    pub summary: String,
    pub evidence: Vec<String>,
}

impl From<harnes_telemetry::Event> for TraceItem {
    fn from(e: harnes_telemetry::Event) -> Self {
        Self {
            id: e.id,
            at: e.at,
            kind: e.kind,
            role: e.role,
            lot: e.lot,
            tool: e.tool,
            path: e.path,
            command: e.command,
            status: e.status,
            summary: e.summary,
            evidence: e.evidence,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct UsageView {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub cost_usd: f64,
    pub requests: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Decision {
    pub at: u64,
    /// gate | review | memory | guard
    pub kind: String,
    #[serde(default)]
    pub id: String,
    /// approve | reject | allow | deny | note
    pub action: String,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub by: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemoryItem {
    pub id: i64,
    pub title: String,
    pub status: String,
    pub kind: String,
    pub confidence: f64,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunView {
    pub schema: String,
    pub run: String,
    pub task: String,
    pub phase: String,
    pub started_at: u64,
    pub plan: Option<Plan>,
    pub reviews: Vec<Review>,
    pub gate: Option<Gate>,
    pub metrics: Option<Metrics>,
    pub trace: Vec<TraceItem>,
    pub findings: Vec<harnes_guards::Finding>,
    pub usage: UsageView,
    pub decisions: Vec<Decision>,
    pub memory: Vec<MemoryItem>,
    pub files: Vec<String>,
}

pub fn build(
    root: &Path,
    run: &str,
    telemetry: Option<&harnes_telemetry::Telemetry>,
    memory: Option<&harnes_memory::Memory>,
) -> RunView {
    let dir = root.join(".harnes").join("runs").join(run);
    let plan = read_json::<Plan>(&dir.join("plan.json")).ok().flatten();
    let mut reviews = Vec::new();
    for i in 0..32 {
        match read_json::<Review>(&dir.join(format!("review-{i}.json"))) {
            Ok(Some(r)) => reviews.push(r),
            _ => break,
        }
    }
    let gate = latest_json::<Gate>(&dir, "gate-");
    let metrics = read_json::<Metrics>(&dir.join("metrics.json"))
        .ok()
        .flatten();
    let task = std::fs::read_to_string(dir.join("task.txt"))
        .unwrap_or_default()
        .trim()
        .to_string();

    let events = telemetry
        .and_then(|t| t.recent(run, 500).ok())
        .unwrap_or_default();
    let summary = telemetry
        .and_then(|t| t.summary(run).ok())
        .unwrap_or_default();
    let findings = harnes_guards::scan(&events);
    let trace: Vec<TraceItem> = events.into_iter().map(TraceItem::from).collect();
    let usage = UsageView {
        input_tokens: summary.input_tokens,
        output_tokens: summary.output_tokens,
        cache_read_tokens: summary.cache_read_tokens,
        cache_write_tokens: summary.cache_write_tokens,
        cost_usd: summary.cost_usd,
        requests: summary.requests,
    };
    let phase = trace
        .iter()
        .find(|t| t.kind == "phase.changed")
        .map(|t| t.status.clone().unwrap_or_default())
        .unwrap_or_else(|| {
            if metrics.as_ref().is_some_and(|m| m.passed) {
                "done".into()
            } else {
                "unknown".into()
            }
        });
    let started_at = trace.iter().map(|t| t.at).min().unwrap_or(0);
    let decisions = read_decisions(&dir);
    let memory = memory
        .and_then(|m| m.candidates("all", Some(run), 50).ok())
        .unwrap_or_default()
        .into_iter()
        .map(|c| MemoryItem {
            id: c.id,
            title: c.title,
            status: c.status,
            kind: c.kind,
            confidence: c.confidence,
            evidence: c.evidence,
        })
        .collect();
    let mut files: Vec<String> = summary.files;
    for r in &reviews {
        for c in &r.commits {
            for f in &c.files {
                files.push(f.clone());
            }
        }
    }
    files.sort();
    files.dedup();

    RunView {
        schema: SCHEMA.into(),
        run: run.into(),
        task,
        phase,
        started_at,
        plan,
        reviews,
        gate,
        metrics,
        trace,
        findings,
        usage,
        decisions,
        memory,
        files,
    }
}

fn latest_json<T: for<'de> Deserialize<'de>>(dir: &Path, prefix: &str) -> Option<T> {
    let mut found: Option<(u32, PathBuf)> = None;
    for e in std::fs::read_dir(dir).ok()?.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        let Some(n) = name
            .strip_prefix(prefix)
            .and_then(|s| s.strip_suffix(".json"))
            .and_then(|s| s.parse::<u32>().ok())
        else {
            continue;
        };
        if found.as_ref().is_none_or(|(m, _)| n > *m) {
            found = Some((n, e.path()));
        }
    }
    read_json::<T>(&found?.1).ok().flatten()
}

fn read_decisions(dir: &Path) -> Vec<Decision> {
    let Ok(text) = std::fs::read_to_string(dir.join("decisions.jsonl")) else {
        return vec![];
    };
    text.lines()
        .filter_map(|l| serde_json::from_str::<Decision>(l).ok())
        .collect()
}

/// Appends a human decision to the run. Never rewrites previous decisions.
pub fn record_decision(root: &Path, run: &str, d: &Decision) -> anyhow::Result<()> {
    use std::io::Write;
    let dir = root.join(".harnes").join("runs").join(run);
    std::fs::create_dir_all(&dir)?;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(dir.join("decisions.jsonl"))?;
    writeln!(f, "{}", serde_json::to_string(d)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_a_typed_view_from_files() {
        let root = std::env::temp_dir().join(format!("harnes-view-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join(".harnes/runs/9");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("task.txt"), "ship it").unwrap();
        std::fs::write(
            dir.join("plan.json"),
            r#"{"lots":[{"id":"L1","title":"a","instructions":"x"}]}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("review-0.json"),
            r#"{"verdict":"PASS","commits":[{"lot":"L1","message":"m","files":["a.rs"]}]}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("gate-0.json"),
            r#"{"ready":true,"summary_for_user":"ok"}"#,
        )
        .unwrap();
        let t = harnes_telemetry::Telemetry::open(&root).unwrap();
        t.append(
            harnes_telemetry::Event::new("9", "p", "phase.changed")
                .status("done")
                .summary("fini"),
        )
        .unwrap();
        t.append(
            harnes_telemetry::Event::new("9", "worker", "model.usage")
                .status("message_start")
                .usage(100, 0, 0, 0)
                .summary("start"),
        )
        .unwrap();
        t.append(
            harnes_telemetry::Event::new("9", "worker", "model.usage")
                .status("message_delta")
                .usage(0, 42, 0, 0)
                .summary("delta"),
        )
        .unwrap();
        let v = build(&root, "9", Some(&t), None);
        assert_eq!(v.schema, "harnes.view.v1");
        assert_eq!(v.task, "ship it");
        assert_eq!(v.files, vec!["a.rs"]);
        assert!(v.gate.is_some_and(|g| g.ready));
        assert_eq!(v.phase, "done");
        assert_eq!(
            (
                v.usage.requests,
                v.usage.input_tokens,
                v.usage.output_tokens
            ),
            (1, 100, 42)
        );
        let _ = std::fs::remove_dir_all(&root);
    }
}
