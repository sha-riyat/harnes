//! Status files for any other agent or script: one small JSON per session in
//! `~/.harnes/sessions/` (and `~/.agent-lookout/sessions/`, same format):
//! `{"agent": "my-bot", "status": "working|idle|finished|failed|waiting", "name"?, "cwd"?, "since"?, "pid"?}`

use super::{alive, home, Adapter};
use crate::model::{Session, Status, WaitingReason};
use anyhow::Result;
use serde_json::Value;

pub struct StatusFiles;

const MAX_FILES: usize = 200;
const MAX_BYTES: u64 = 16 * 1024;

impl Adapter for StatusFiles {
    fn id(&self) -> &'static str {
        "status-files"
    }

    fn poll(&self) -> Result<Option<Vec<Session>>> {
        let dirs = [
            home().join(".harnes").join("sessions"),
            home().join(".agent-lookout").join("sessions"),
        ];
        if !dirs.iter().any(|d| d.exists()) {
            return Ok(None);
        }
        let mut out = vec![];
        for dir in dirs {
            for e in std::fs::read_dir(&dir)
                .into_iter()
                .flatten()
                .flatten()
                .take(MAX_FILES)
            {
                let p = e.path();
                if p.extension().and_then(|x| x.to_str()) != Some("json")
                    || p.metadata().map(|m| m.len()).unwrap_or(0) > MAX_BYTES
                {
                    continue;
                }
                let Ok(v) = std::fs::read_to_string(&p)
                    .map_err(anyhow::Error::from)
                    .and_then(|s| Ok(serde_json::from_str::<Value>(&s)?))
                else {
                    continue;
                };
                let (Some(agent), Some(status)) = (v["agent"].as_str(), v["status"].as_str())
                else {
                    continue;
                };
                let pid = v["pid"].as_u64().map(|p| p as u32);
                if pid.is_some_and(|p| !alive(p)) {
                    continue;
                }
                let stem = p.file_stem().and_then(|n| n.to_str()).unwrap_or(agent);
                let mut s = Session::new(self.id(), "other", stem, v["cwd"].as_str().unwrap_or(""));
                s.name = v["name"].as_str().unwrap_or(agent).to_string();
                (s.status, s.waiting_reason) = match status {
                    "working" => (Status::Working, None),
                    "idle" => (Status::Idle, None),
                    "finished" => (Status::Finished, None),
                    "failed" => (Status::Failed, None),
                    "waiting" => (Status::NeedsYou, Some(WaitingReason::Other)),
                    _ => (Status::Unknown, None),
                };
                s.status_since = v["since"].as_u64().unwrap_or(0);
                s.pid = pid;
                out.push(s);
            }
        }
        Ok(Some(out))
    }
}
