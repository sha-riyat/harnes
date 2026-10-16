//! How a crew run uses threads: one `run-<id>` thread for the whole run and
//! one `lot-<id>-<lot>` thread per lot. The engine posts progress, lot
//! briefs and review fixes; agents ask and answer each other there.

use crate::plan::{Fix, Plan};
use harnes_threads::Threads;

pub fn run_thread(run: &str) -> String {
    format!("run-{run}")
}

pub fn lot_thread(run: &str, lot: &str) -> String {
    format!("lot-{run}-{lot}")
}

/// Wraps an optional thread store; every call is best-effort (threads never block a run).
#[derive(Clone, Default)]
pub struct RunThreads(pub Option<std::sync::Arc<Threads>>);

impl RunThreads {
    pub fn open_run(&self, run: &str, task: &str) {
        if let Some(t) = &self.0 {
            let title: String = task.chars().take(80).collect();
            if t.ensure(&run_thread(run), &title, "run", Some(run), "harnes").is_ok() {
                let _ = t.post(&run_thread(run), "user", "message", task);
            }
        }
    }

    pub fn event(&self, run: &str, body: &str) {
        if let Some(t) = &self.0 {
            let _ = t.post(&run_thread(run), "harnes", "event", body);
        }
    }

    /// One thread per lot, seeded with the lot brief.
    pub fn open_lots(&self, run: &str, plan: &Plan) {
        let Some(t) = &self.0 else { return };
        for lot in &plan.lots {
            let id = lot_thread(run, &lot.id);
            if t.ensure(&id, &format!("{} · {}", lot.id, lot.title), "lot", Some(run), "planner").is_ok() {
                let brief = format!(
                    "Lot {} ({}). Fichiers : {}\n{}\nAcceptation : {}",
                    lot.id,
                    lot.kind,
                    lot.files.join(", "),
                    lot.instructions,
                    lot.acceptance
                );
                let _ = t.post(&id, "planner", "handoff", &brief);
            }
        }
    }

    pub fn fixes(&self, run: &str, author: &str, fixes: &[(String, Vec<Fix>)]) {
        let Some(t) = &self.0 else { return };
        for (lot, list) in fixes {
            let body = list.iter().map(|f| format!("- {} : {} -> {}", f.r#where, f.problem, f.fix)).collect::<Vec<_>>().join("\n");
            let _ = t.post(&lot_thread(run, lot), author, "message", &format!("Corrections demandées :\n{body}"));
        }
    }

    /// Closes every thread of the run with a summary; history stays readable.
    pub fn close_run(&self, run: &str, summary: &str) {
        let Some(t) = &self.0 else { return };
        if let Ok(list) = t.list("open", Some(run), 200) {
            for th in list {
                let s = if th.kind == "run" { summary } else { "lot terminé" };
                let _ = t.close(&th.id, "teacher", Some(s));
            }
        }
    }
}
