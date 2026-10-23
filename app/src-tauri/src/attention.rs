//! Everything that needs the user, surfaced in the UI instead of terminals:
//! permission requests (hook), questions and menus read from agent screens,
//! and the orchestrator conversation.

use crate::state::Shared;
use harnes_crew::{
    context,
    permissions::{Ask, Decision},
    screen,
};
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::collections::HashMap;
use tauri::{Emitter, State};

/// Last prompt seen on each agent pane, to emit only changes, and whether
/// it was answered: the menu stays on screen a moment after the keys go in,
/// and must not come back as a new card.
#[derive(Default)]
pub struct Prompts(Mutex<HashMap<String, (Option<screen::ScreenPrompt>, bool)>>);

/// Records a permission request or answer next to the agent's run, so the
/// trace and the findings agree with what the user saw in the UI.
fn record_ask(st: &Shared, ask: &Ask, kind: &str, status: &str, summary: String) {
    let Some(p) = st.project_of_key(&ask.agent) else {
        return;
    };
    let role = st
        .agents
        .read()
        .get(&ask.agent)
        .map(|a| a.role.clone())
        .unwrap_or_default();
    let run = p.run_slot.lock().clone().unwrap_or_default();
    let _ = p.telemetry.append(
        harnes_telemetry::Event::new(&run, &p.id, kind)
            .role(role)
            .tool(ask.tool.clone())
            .status(status)
            .summary(summary)
            .detail(ask.input.clone())
            .evidence(vec![ask.id.clone()]),
    );
}

/// Forwards new permission requests to the UI.
pub fn install(st: &Shared) {
    let app = st.app.clone();
    let shared = st.clone();
    st.hooks
        .permissions
        .set_sink(std::sync::Arc::new(move |ask| {
            record_ask(
                &shared,
                ask,
                "approval.requested",
                "pending",
                ask.summary.clone(),
            );
            let _ = app.emit("permission-request", ask);
        }));
    // Typed questions from agents (`ask_user`).
    let (app, shared) = (st.app.clone(), st.clone());
    st.hooks.asks.set_sink(std::sync::Arc::new(move |pending| {
        record_question(
            &shared,
            &pending.agent,
            "question.asked",
            "pending",
            &pending.card.question,
            &pending.id,
        );
        let _ = app.emit("ask-request", pending);
    }));
}

/// Records a typed question or its answer next to the agent's run. Only what
/// the agent asked is kept; a free-text answer is not stored.
fn record_question(st: &Shared, agent: &str, kind: &str, status: &str, summary: &str, id: &str) {
    let Some(p) = st.project_of_key(agent) else {
        return;
    };
    let role = st
        .agents
        .read()
        .get(agent)
        .map(|a| a.role.clone())
        .unwrap_or_default();
    let run = p.run_slot.lock().clone().unwrap_or_default();
    let _ = p.telemetry.append(
        harnes_telemetry::Event::new(&run, &p.id, kind)
            .role(role)
            .tool("ask_user")
            .status(status)
            .summary(summary.to_string())
            .evidence(vec![id.to_string()]),
    );
}

/// Typed questions waiting for the user.
#[tauri::command]
pub fn asks_pending(st: State<'_, Shared>) -> Value {
    json!(st.hooks.asks.pending())
}

#[tauri::command]
pub fn ask_answer(
    st: State<'_, Shared>,
    id: String,
    choice: Option<String>,
    text: Option<String>,
) -> Result<(), String> {
    let pending = st.hooks.asks.pending().into_iter().find(|p| p.id == id);
    st.hooks.asks.answer(
        &id,
        harnes_crew::ask::Answer {
            choice: choice.clone(),
            text: text.clone(),
        },
    )?;
    if let Some(p) = pending {
        let what = match (&choice, &text) {
            (Some(c), _) => format!("réponse : {c}"),
            (_, Some(t)) => format!("réponse libre ({} caractères)", t.chars().count()),
            _ => "réponse".into(),
        };
        record_question(&st, &p.agent, "question.answered", "ok", &what, &id);
    }
    Ok(())
}

#[tauri::command]
pub fn ask_decline(st: State<'_, Shared>, id: String) -> Result<(), String> {
    let pending = st.hooks.asks.pending().into_iter().find(|p| p.id == id);
    if !st.hooks.asks.decline(&id) {
        return Err("question expirée".into());
    }
    if let Some(p) = pending {
        record_question(
            &st,
            &p.agent,
            "question.declined",
            "declined",
            "l'utilisateur n'a pas répondu",
            &id,
        );
    }
    Ok(())
}

/// Runs on every fresh frame of an agent pane (from the frame pump).
pub fn on_frame(st: &Shared, prompts: &Prompts, pane: &str, text: &str) {
    let Some((key, project)) = st
        .agents
        .read()
        .values()
        .find(|a| a.pane.as_deref() == Some(pane))
        .map(|a| (a.key.clone(), a.project.clone()))
    else {
        return;
    };
    let found = screen::detect(text);
    let mut seen = prompts.0.lock();
    if seen.get(pane).map(|(p, _)| p) == Some(&found) {
        return;
    }
    seen.insert(pane.to_string(), (found.clone(), false));
    drop(seen);
    let _ = st.app.emit(
        "screen-prompt",
        json!({ "pane": pane, "agent": key, "project": project, "prompt": found }),
    );
}

#[tauri::command]
pub fn permissions_pending(st: State<'_, Shared>) -> Value {
    json!(st.hooks.permissions.pending())
}

#[tauri::command]
pub fn permission_answer(
    st: State<'_, Shared>,
    id: String,
    decision: Decision,
) -> Result<(), String> {
    let ask = st
        .hooks
        .permissions
        .pending()
        .into_iter()
        .find(|a| a.id == id);
    if st.hooks.permissions.answer(&id, decision) {
        if let Some(ask) = ask {
            let (kind, status, label) = match decision {
                Decision::Allow => ("approval.granted", "allowed", "accordée"),
                Decision::Always => ("approval.granted", "allowed", "toujours autorisée"),
                Decision::Deny => ("approval.denied", "denied", "refusée"),
            };
            record_ask(
                &st,
                &ask,
                kind,
                status,
                format!("{} — {label}", ask.summary),
            );
        }
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
        .filter_map(|(pane, (p, answered))| {
            let p = p.as_ref().filter(|_| !answered)?;
            let a = agents
                .values()
                .find(|a| a.pane.as_deref() == Some(pane.as_str()))?;
            Some(json!({ "pane": pane, "agent": a.key, "project": a.project, "prompt": p }))
        })
        .collect();
    json!(list)
}

/// Sends the chosen answer's keys to the agent's terminal.
#[tauri::command]
pub fn screen_answer(
    st: State<'_, Shared>,
    prompts: State<'_, Prompts>,
    pane: String,
    keys: String,
) -> Result<(), String> {
    // One key per write, a beat apart: terminal UIs (Ink) read a burst of
    // escape sequences as one unknown key.
    let mut tokens = vec![];
    let mut rest = keys.as_str();
    while let Some(c) = rest.chars().next() {
        let n = if rest.starts_with("\u{1b}[") {
            3.min(rest.len())
        } else {
            c.len_utf8()
        };
        tokens.push(rest[..n].to_string());
        rest = &rest[n..];
    }
    let (panes, target) = (st.panes.clone(), pane.clone());
    std::thread::spawn(move || {
        for t in tokens {
            let _ = panes.write(&target, t.as_bytes());
            std::thread::sleep(std::time::Duration::from_millis(60));
        }
    });
    if let Some(slot) = prompts.0.lock().get_mut(&pane) {
        slot.1 = true;
    }
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
