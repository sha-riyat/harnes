//! Polls every agent source every 2 s and pushes changes to the UI.
//! harnes' own agents are tagged by matching their pane process id.

use crate::state::Shared;
use harnes_crew::AgentState;
use harnes_lookout::model::Status;
use std::time::Duration;
use tauri::Emitter;

fn status_of(s: AgentState) -> Status {
    match s {
        AgentState::Working | AgentState::Starting => Status::Working,
        AgentState::Blocked => Status::NeedsYou,
        AgentState::Idle | AgentState::Done => Status::Idle,
        AgentState::Exited => Status::Finished,
    }
}

pub fn start(st: Shared) {
    tauri::async_runtime::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(1));
        let mut n: u64 = 0;
        let mut versions: std::collections::HashMap<String, (i64, i64)> = Default::default();
        loop {
            tick.tick().await;
            n += 1;
            // Threads: cheap version check every second.
            let projects: Vec<_> = st.projects.read().clone();
            for p in projects {
                if let Some(Ok(v)) = p.threads.as_ref().map(|t| t.version()) {
                    if versions.get(&p.id) != Some(&v) {
                        versions.insert(p.id.clone(), v);
                        let _ = st.app.emit("threads-changed", &p.id);
                    }
                }
            }
            if n % 2 == 1 {
                continue; // agent sources every 2 s
            }
            let st2 = st.clone();
            let changed = tauri::async_runtime::spawn_blocking(move || {
                let agents: Vec<_> = st2.agents.read().values().cloned().collect();
                let mut c = st2.lookout.lock();
                // harnes agents already seen by the Claude Code adapter (same pid) are not duplicated;
                // the others (non-Claude providers) are added as sessions of their own.
                let known: Vec<u32> = c.snapshot().sessions.iter().filter_map(|s| s.pid).collect();
                let extra = agents
                    .iter()
                    .filter(|a| !a.pid.is_some_and(|p| known.contains(&p)))
                    .map(|a| {
                        let mut s =
                            harnes_lookout::model::Session::new("harnes", &a.provider, &a.key, "");
                        s.name = a
                            .key
                            .split_once('-')
                            .map(|(_, n)| n)
                            .unwrap_or(&a.key)
                            .to_string();
                        s.status = status_of(a.state);
                        s.pid = a.pid;
                        s.harnes_key = Some(a.key.clone());
                        s
                    })
                    .collect();
                let events = c.poll(extra);
                !events.is_empty()
            })
            .await
            .unwrap_or(false);
            if changed {
                let _ = st.app.emit("lookout-changed", ());
            }
        }
    });
}
