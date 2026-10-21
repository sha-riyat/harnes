//! Claude Code: the per-process registry `~/.claude/sessions/<pid>.json`.

use super::{alive, home, Adapter};
use crate::model::{Session, Status, WaitingReason};
use anyhow::Result;
use serde_json::Value;

pub struct ClaudeCode;

/// Maps registry `status` / `state` / `waitingFor` to our statuses.
pub fn map(
    status: &str,
    state: Option<&str>,
    waiting_for: Option<&str>,
) -> (Status, Option<WaitingReason>) {
    match state {
        Some("blocked") => return (Status::NeedsYou, Some(WaitingReason::Other)),
        Some("done" | "stopped") => return (Status::Finished, None),
        Some("failed") => return (Status::Failed, None),
        _ => {}
    }
    match status {
        "busy" | "shell" => (Status::Working, None),
        "idle" => (Status::Idle, None),
        "waiting" => {
            let reason = match waiting_for.unwrap_or("") {
                w if w.contains("permission") => WaitingReason::Permission,
                w if w.contains("input") || w.contains("question") => WaitingReason::Question,
                _ => WaitingReason::Other,
            };
            (Status::NeedsYou, Some(reason))
        }
        _ => (Status::Unknown, None),
    }
}

impl Adapter for ClaudeCode {
    fn id(&self) -> &'static str {
        "claude-code"
    }

    fn poll(&self) -> Result<Option<Vec<Session>>> {
        let dir = home().join(".claude").join("sessions");
        let Ok(rd) = std::fs::read_dir(&dir) else {
            return Ok(None);
        };
        let mut out = vec![];
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|x| x.to_str()) != Some("json") {
                continue;
            }
            let Ok(v) = std::fs::read_to_string(&p)
                .map_err(anyhow::Error::from)
                .and_then(|s| Ok(serde_json::from_str::<Value>(&s)?))
            else {
                continue;
            };
            let kind = v["kind"].as_str().unwrap_or("");
            if !matches!(kind, "interactive" | "bg") {
                continue;
            }
            let pid = v["pid"].as_u64().map(|p| p as u32);
            if pid.is_some_and(|p| !alive(p)) {
                continue; // stale registry entry
            }
            let sid = v["sessionId"].as_str().unwrap_or_default();
            let mut s = Session::new(self.id(), "claude", sid, v["cwd"].as_str().unwrap_or(""));
            s.name = v["name"].as_str().unwrap_or("claude").to_string();
            let (status, reason) = map(
                v["status"].as_str().unwrap_or(""),
                v["state"].as_str(),
                v["waitingFor"].as_str(),
            );
            s.status = status;
            s.waiting_reason = reason;
            s.waiting_text = v["waitingFor"].as_str().map(String::from);
            s.started_at = v["startedAt"].as_u64().unwrap_or(0);
            s.status_since = v["statusUpdatedAt"].as_u64().unwrap_or(s.started_at);
            s.pid = pid;
            out.push(s);
        }
        Ok(Some(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_statuses() {
        assert_eq!(map("busy", None, None).0, Status::Working);
        assert_eq!(
            map("waiting", None, Some("permission prompt")),
            (Status::NeedsYou, Some(WaitingReason::Permission))
        );
        assert_eq!(map("idle", Some("done"), None).0, Status::Finished);
        assert_eq!(map("weird", None, None).0, Status::Unknown);
    }
}
