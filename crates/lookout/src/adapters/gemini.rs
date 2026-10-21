//! Gemini CLI: chat logs `~/.gemini/tmp/<project>/chats/session-*.jsonl`.
//! No status is recorded, so it is inferred from write activity.

use super::{home, mtime_ms, Adapter};
use crate::model::{now_ms, Session, Status};
use anyhow::Result;

pub struct GeminiCli;

const WORKING_WINDOW_MS: u64 = 15_000;
const FINISHED_AFTER_MS: u64 = 30 * 60 * 1000;

impl Adapter for GeminiCli {
    fn id(&self) -> &'static str {
        "gemini-cli"
    }

    fn poll(&self) -> Result<Option<Vec<Session>>> {
        let root = home().join(".gemini").join("tmp");
        let Ok(projects) = std::fs::read_dir(&root) else {
            return Ok(None);
        };
        let mut out = vec![];
        for proj in projects.flatten() {
            let cwd = std::fs::read_to_string(proj.path().join(".project_root"))
                .unwrap_or_default()
                .trim()
                .to_string();
            for e in std::fs::read_dir(proj.path().join("chats"))
                .into_iter()
                .flatten()
                .flatten()
            {
                let p = e.path();
                let Some(name) = p.file_stem().and_then(|n| n.to_str()) else {
                    continue;
                };
                if !name.starts_with("session-") {
                    continue;
                }
                let age = now_ms().saturating_sub(mtime_ms(&p));
                if age > FINISHED_AFTER_MS * 4 {
                    continue;
                }
                let dir_name = proj.file_name().to_string_lossy().into_owned();
                let mut s = Session::new(
                    self.id(),
                    "gemini",
                    name,
                    if cwd.is_empty() { &dir_name } else { &cwd },
                );
                s.name = format!("gemini {}", s.project);
                s.status = if age < WORKING_WINDOW_MS {
                    Status::Working
                } else if age > FINISHED_AFTER_MS {
                    Status::Finished
                } else {
                    Status::Idle
                };
                s.status_since = mtime_ms(&p);
                out.push(s);
            }
        }
        Ok(Some(out))
    }
}
