//! Everything that needs the user, surfaced in the UI instead of terminals:
//! permission requests (hook), questions and menus read from agent screens,
//! and the orchestrator conversation.

use crate::state::Shared;
use harnes_crew::{context, permissions::Decision, screen};
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::collections::HashMap;
use tauri::{Emitter, State};

/// Last prompt seen on each agent pane, to emit only changes.
#[derive(Default)]
pub struct Prompts(Mutex<HashMap<String, Option<screen::ScreenPrompt>>>);

/// Forwards new permission requests to the UI.
pub fn install(st: &Shared) {
    let app = st.app.clone();
    st.hooks.permissions.set_sink(std::sync::Arc::new(move |ask| {
        let _ = app.emit("permission-request", ask);
    }));
}

/// Runs on every fresh frame of an agent pane (from the frame pump).
pub fn on_frame(st: &Shared, prompts: &Prompts, pane: &str, text: &str) {
    let Some((key, project)) = st.agents.read().values().find(|a| a.pane.as_deref() == Some(pane)).map(|a| (a.key.clone(), a.project.clone())) else {
        return;
    };
    let found = screen::detect(text);
    let mut seen = prompts.0.lock();
    if seen.get(pane) == Some(&found) {
        return;
    }
    seen.insert(pane.to_string(), found.clone());
    drop(seen);
    let _ = st.app.emit("screen-prompt", json!({ "pane": pane, "agent": key, "project": project, "prompt": found }));
}

#[tauri::command]
pub fn permissions_pending(st: State<'_, Shared>) -> Value {
    json!(st.hooks.permissions.pending())
}

#[tauri::command]
pub fn permission_answer(st: State<'_, Shared>, id: String, decision: Decision) -> Result<(), String> {
    if st.hooks.permissions.answer(&id, decision) {
        Ok(())
    } else {
        Err("demande expirée : réponds dans le terminal de l'agent".into())
    }
}

/// Questions currently visible on agent screens.
#[tauri::command]
pub fn screen_prompts(st: State<'_, Shared>, prompts: State<'_, Prompts>) -> Value {
    let agents = st.agents.read();
    let list: Vec<Value> = prompts
        .0
        .lock()
        .iter()
        .filter_map(|(pane, p)| {
            let p = p.as_ref()?;
            let a = agents.values().find(|a| a.pane.as_deref() == Some(pane.as_str()))?;
            Some(json!({ "pane": pane, "agent": a.key, "project": a.project, "prompt": p }))
        })
        .collect();
    json!(list)
}

/// Sends the chosen answer's keys to the agent's terminal.
#[tauri::command]
pub fn screen_answer(st: State<'_, Shared>, prompts: State<'_, Prompts>, pane: String, keys: String) -> Result<(), String> {
    st.panes.write(&pane, keys.as_bytes()).map_err(|e| e.to_string())?;
    prompts.0.lock().insert(pane, None);
    Ok(())
}

/// The orchestrator conversation of a project, read from its Claude transcript.
#[tauri::command]
pub fn conversation(st: State<'_, Shared>, project: Option<String>) -> Result<Value, String> {
    let p = st.pick(project.as_deref())?;
    let key = p.orch_key();
    let agent = st.agents.read().get(&key).cloned();
    let items = st
        .hooks
        .registry
        .session_of(&key)
        .and_then(|s| context::claude_transcript(&s))
        .and_then(|path| context::conversation_claude(&path, 300).ok())
        .unwrap_or_default();
    Ok(json!({
        "started": agent.is_some(),
        "state": agent.as_ref().map(|a| a.state),
        "pane": agent.and_then(|a| a.pane),
        "items": items,
    }))
}
