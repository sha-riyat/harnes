//! Several projects open at once (like Herdr workspaces): each has its own
//! proxy, context store, threads and crew run. One is active in the UI.

use crate::state::{ctx_bin, project_id, CrewState, Project, Shared};
use harnes_crew::setup;
use harnes_ctx::Store;
use harnes_proxy::{ProxyConfig, DEFAULT_UPSTREAM};
use serde_json::{json, Value};
use std::{path::PathBuf, sync::Arc};
use tauri::{Emitter, State};

/// Opens (or re-activates) a project. Returns its id.
pub async fn open(st: &Shared, path: &str, force: bool, activate: bool) -> Result<(String, setup::SetupReport), String> {
    let root = PathBuf::from(path.trim());
    if !root.is_dir() {
        return Err(format!("dossier introuvable : {}", root.display()));
    }
    let id = project_id(&root);
    if st.project(&id).is_some() {
        if activate {
            set_active(st, &id);
        }
        return Ok((id, setup::SetupReport::default()));
    }
    let report = setup::equip(&root, &ctx_bin(), force).map_err(|e| format!("{e:#}"))?;
    let store = Arc::new(Store::open(&root.join(".harnes").join("ctx.db")).map_err(|e| format!("{e:#}"))?);
    let proxy = harnes_proxy::start(ProxyConfig { port: 0, upstream: DEFAULT_UPSTREAM.into(), compress: true, store: Some(store.clone()) })
        .await
        .map_err(|e| format!("{e:#}"))?;
    let threads = harnes_threads::Threads::open(&root.join(".harnes").join("threads.db")).ok().map(Arc::new);
    let name = root.file_name().and_then(|n| n.to_str()).unwrap_or("projet").to_string();
    let project = Arc::new(Project { id: id.clone(), name, root, store, proxy, threads, crew: CrewState::default() });
    st.projects.write().push(project);
    if activate || st.active.read().is_none() {
        set_active(st, &id);
    }
    let _ = st.app.emit("projects-changed", ());
    Ok((id, report))
}

pub fn set_active(st: &Shared, id: &str) {
    *st.active.write() = Some(id.to_string());
    let _ = st.app.emit("project-active", id);
}

/// Summary used by the sidebar.
pub fn summary(st: &Shared, p: &Project) -> Value {
    let agents = st.agents.read();
    let mine: Vec<_> = agents.values().filter(|a| a.project == p.id).collect();
    let panes = st.pane_project.read().values().filter(|v| **v == p.id).count();
    json!({
        "id": p.id,
        "name": p.name,
        "root": p.root.display().to_string(),
        "active": st.active.read().as_deref() == Some(p.id.as_str()),
        "running": p.crew.running(),
        "phase": p.crew.log.lock().last().map(|e| e.phase),
        "panes": panes,
        "working": mine.iter().filter(|a| matches!(a.state, harnes_crew::AgentState::Working | harnes_crew::AgentState::Starting)).count(),
        "waiting": mine.iter().filter(|a| a.state == harnes_crew::AgentState::Blocked).count(),
        "proxy_port": p.proxy.port,
    })
}

#[tauri::command]
pub async fn open_project(st: State<'_, Shared>, path: String, force: bool) -> Result<Value, String> {
    let (id, report) = open(st.inner(), &path, force, true).await?;
    Ok(json!({ "id": id, "written": report.written, "kept": report.kept }))
}

#[tauri::command]
pub fn projects_list(st: State<'_, Shared>) -> Vec<Value> {
    st.projects.read().iter().map(|p| summary(st.inner(), p)).collect()
}

#[tauri::command]
pub fn project_activate(st: State<'_, Shared>, id: String) -> Result<(), String> {
    st.project(&id).ok_or("projet inconnu")?;
    set_active(st.inner(), &id);
    Ok(())
}

/// Closes a project: cancels its run, closes its panes, stops its proxy.
#[tauri::command]
pub fn project_close(st: State<'_, Shared>, id: String) -> Result<(), String> {
    let p = st.project(&id).ok_or("projet inconnu")?;
    if let Some(tx) = p.crew.cancel.lock().as_ref() {
        let _ = tx.send(true);
    }
    let panes: Vec<String> = st.pane_project.read().iter().filter(|(_, v)| **v == id).map(|(k, _)| k.clone()).collect();
    for pane in &panes {
        let _ = st.panes.close(pane);
        let _ = st.app.emit("pane-closed", pane);
    }
    st.pane_project.write().retain(|_, v| *v != id);
    st.agents.write().retain(|_, a| a.project != id);
    p.proxy.stop();
    st.projects.write().retain(|x| x.id != id);
    if st.active.read().as_deref() == Some(id.as_str()) {
        let next = st.projects.read().first().map(|p| p.id.clone());
        *st.active.write() = next.clone();
        let _ = st.app.emit("project-active", next);
    }
    let _ = st.app.emit("projects-changed", ());
    Ok(())
}

#[tauri::command]
pub fn app_info(st: State<'_, Shared>) -> Value {
    let p = st.active();
    let bin = ctx_bin();
    json!({
        "project": p.as_ref().map(|p| p.root.display().to_string()),
        "project_id": p.as_ref().map(|p| p.id.clone()),
        "proxy_port": p.as_ref().map(|p| p.proxy.port),
        "hook_port": st.hooks.port,
        "ctx_bin": bin.display().to_string(),
        "ctx_bin_found": bin.exists(),
        "crew_running": p.as_ref().is_some_and(|p| p.crew.running()),
    })
}

#[tauri::command]
pub fn stats(st: State<'_, Shared>) -> Value {
    match st.active() {
        Some(p) => json!({ "proxy": p.proxy.snapshot(), "ctx": p.store.stats().ok(), "runs": harnes_crew::review::history(&p.root) }),
        None => json!({ "proxy": null, "ctx": null }),
    }
}
