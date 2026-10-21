//! OpenAI Codex CLI: rollout files `~/.codex/sessions/YYYY/MM/DD/rollout-*.jsonl`
//! (today and yesterday). Codex records no approval waits.

use super::{home, mtime_ms, tail_lines, Adapter};
use crate::model::{now_ms, Session, Status, Tokens};
use anyhow::Result;
use serde_json::Value;
use std::path::{Path, PathBuf};

pub struct Codex;

/// A session idle this long is considered finished.
const FINISHED_AFTER_MS: u64 = 30 * 60 * 1000;

fn codex_home() -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".codex"))
}

fn day_dirs(root: &Path) -> Vec<PathBuf> {
    // Two most recent YYYY/MM/DD folders, whatever the timezone.
    let mut days = vec![];
    for y in std::fs::read_dir(root).into_iter().flatten().flatten() {
        for m in std::fs::read_dir(y.path()).into_iter().flatten().flatten() {
            for d in std::fs::read_dir(m.path()).into_iter().flatten().flatten() {
                days.push(d.path());
            }
        }
    }
    days.sort();
    days.into_iter().rev().take(2).collect()
}

/// Reads one rollout: meta from the first line, status from the tail.
pub fn read_rollout(path: &Path) -> Option<Session> {
    use std::io::BufRead;
    let mut first = String::new();
    std::io::BufReader::new(std::fs::File::open(path).ok()?)
        .read_line(&mut first)
        .ok()?;
    let meta: Value = serde_json::from_str(&first).ok()?;
    let p = &meta["payload"];
    if meta["type"] != "session_meta" {
        return None;
    }
    let id = p["id"].as_str().or(p["session_id"].as_str())?;
    let mut s = Session::new("codex", "codex", id, p["cwd"].as_str().unwrap_or(""));
    s.name = format!("codex {}", &id[..id.len().min(8)]);
    let mut status = Status::Idle;
    for line in tail_lines(path, 256 * 1024) {
        let Ok(v) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if v["type"] != "event_msg" {
            continue;
        }
        match v["payload"]["type"].as_str() {
            Some("task_started") => status = Status::Working,
            Some("task_complete") => status = Status::Idle,
            Some("token_count") => {
                let u = &v["payload"]["info"]["last_token_usage"];
                if u.is_object() {
                    s.tokens = Some(Tokens {
                        input: u["input_tokens"].as_u64().unwrap_or(0),
                        cached: u["cached_input_tokens"].as_u64().unwrap_or(0),
                        output: u["output_tokens"].as_u64().unwrap_or(0),
                    });
                }
            }
            _ => {}
        }
    }
    let modified = mtime_ms(path);
    if now_ms().saturating_sub(modified) > FINISHED_AFTER_MS {
        status = Status::Finished;
    }
    s.status = status;
    s.status_since = modified;
    s.started_at = modified;
    Some(s)
}

impl Adapter for Codex {
    fn id(&self) -> &'static str {
        "codex"
    }

    fn poll(&self) -> Result<Option<Vec<Session>>> {
        let root = codex_home().join("sessions");
        if !root.exists() {
            return Ok(None);
        }
        let mut out = vec![];
        for day in day_dirs(&root) {
            for e in std::fs::read_dir(day).into_iter().flatten().flatten() {
                let p = e.path();
                if p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("rollout-") && n.ends_with(".jsonl"))
                {
                    if let Some(s) = read_rollout(&p) {
                        out.push(s);
                    }
                }
            }
        }
        Ok(Some(out))
    }
}
