//! Crew control per project (UI and orchestrator), Claude panes, thread commands.

use crate::{
    host::{agent_env, PaneHost},
    state::{AgentView, Project, Shared},
};
use harnes_crew::{
    budget::Mode, engine::Engine, threading::RunThreads, AgentState, CrewEvent, Phase,
};
use harnes_pty::SpawnSpec;
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use tauri::{Emitter, State};

/// Starts (or resumes) a crew run in a project, in the background.
pub fn start_crew(
    st: &Shared,
    p: &Arc<Project>,
    task: String,
    resume: Option<String>,
    mode: Mode,
) -> Result<(), String> {
    if task.trim().is_empty() {
        return Err("tâche vide".into());
    }
    if let Some(why) = crate::budget::blocked(st) {
        return Err(why);
    }
    // Nothing starts in a folder whose own config redirects the API or runs code.
    harnes_guards::trust::check(&p.root)?;
    let mut slot = p.crew.cancel.lock();
    if slot.is_some() {
        return Err("un run est déjà en cours dans ce projet".into());
    }
    let providers =
        harnes_crew::providers::Providers::load(&p.root).map_err(|e| format!("{e:#}"))?;
    let (tx, rx) = tokio::sync::watch::channel(false);
    *slot = Some(tx);
    p.crew.log.lock().clear();
    let resuming = resume.is_some();
    let run_id = resume.unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
            .to_string()
    });
    // The mode is kept with the run, so a resumed run keeps its shape.
    let mode_file = harnes_crew::setup::run_dir(&p.root, &run_id).join("mode.json");
    let mode = match std::fs::read_to_string(&mode_file)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
    {
        Some(saved) if resuming => saved,
        _ => {
            let _ = std::fs::create_dir_all(mode_file.parent().unwrap());
            let _ = std::fs::write(&mode_file, serde_json::to_string(&mode).unwrap_or_default());
            mode
        }
    };
    *p.crew.task.lock() = Some((run_id.clone(), task.clone()));
    *p.run_slot.lock() = Some(run_id.clone());
    let (shared, project) = (st.clone(), p.clone());
    let engine = Engine {
        host: Arc::new(PaneHost(shared.clone(), providers, project.clone())),
        hooks: shared.hooks.clone(),
        root: project.root.clone(),
        project: project.id.clone(),
        emit: {
            let (s, pr) = (shared.clone(), project.clone());
            Arc::new(move |ev: CrewEvent| {
                pr.crew.log.lock().push(ev.clone());
                pr.crew.tick.send_modify(|n| *n += 1);
                let mut v = serde_json::to_value(&ev).unwrap_or_default();
                v["project"] = json!(pr.id);
                let _ = s.app.emit("crew", v);
            })
        },
        cancel: rx,
        max_rounds: 3,
        agent_timeout: Duration::from_secs(60 * 60),
        run_e2e: true,
        threads: RunThreads(project.threads.clone()),
        mode,
        eco: crate::budget::pace(st).is_some_and(|p| p.eco()),
        telemetry: Some(project.telemetry.clone()),
        memory: Some(project.memory.clone()),
        worktrees: Some(project.worktrees.clone()),
    };
    tauri::async_runtime::spawn(async move {
        let _ = engine.run_resumable(&task, Some(run_id)).await;
        *project.crew.cancel.lock() = None;
        *project.crew.task.lock() = None;
        *project.run_slot.lock() = None;
        project.crew.tick.send_modify(|n| *n += 1);
        let _ = shared.app.emit("projects-changed", ());
    });
    let _ = st.app.emit("projects-changed", ());
    Ok(())
}

/// Snapshot for `crew_status`: phase, agents, recent events.
pub fn status(st: &Shared, p: &Project) -> Value {
    let log = p.crew.log.lock();
    let agents = st.agents.read();
    let mine: Vec<&AgentView> = agents
        .values()
        .filter(|a| a.project == p.id && a.key != p.orch_key())
        .collect();
    json!({
        "project": p.name,
        "running": p.crew.running(),
        "phase": log.last().map(|e| e.phase).unwrap_or(Phase::Idle),
        "run": log.last().map(|e| e.run.clone()),
        "agents": mine.iter().map(|a| json!({"key": a.key, "role": a.role, "provider": a.provider, "state": a.state})).collect::<Vec<_>>(),
        "blocked": mine.iter().filter(|a| a.state == AgentState::Blocked).map(|a| a.key.clone()).collect::<Vec<_>>(),
        "events": log.iter().rev().take(12).rev().map(|e| json!({"phase": e.phase, "round": e.round, "message": e.message})).collect::<Vec<_>>(),
        "review": log.iter().rev().find_map(|e| e.review.clone()),
        "gate": log.iter().rev().find_map(|e| e.gate.clone()),
    })
}

/// Handler behind `POST /api/<cmd>`; the body carries `project` (from `HARNES_PROJECT`).
pub fn install_control(st: &Shared) {
    let shared = st.clone();
    st.hooks
        .set_control(Arc::new(move |cmd: String, body: Value| {
            let st = shared.clone();
            Box::pin(async move {
                let Ok(p) = st.pick(body["project"].as_str()) else {
                    return json!({"error": "projet inconnu"});
                };
                match cmd.as_str() {
                    "start" => match start_crew(
                        &st,
                        &p,
                        body["task"].as_str().unwrap_or("").to_string(),
                        None,
                        serde_json::from_value(body["mode"].clone()).unwrap_or_default(),
                    ) {
                        Ok(()) => json!({"ok": true, "message": "run lancé"}),
                        Err(e) => json!({"ok": false, "error": e}),
                    },
                    "status" => {
                        let wait = body["wait_secs"].as_u64().unwrap_or(0);
                        if wait > 0 {
                            let mut rx = p.crew.tick.subscribe();
                            rx.borrow_and_update();
                            let _ =
                                tokio::time::timeout(Duration::from_secs(wait), rx.changed()).await;
                        }
                        status(&st, &p)
                    }
                    "cancel" => {
                        if let Some(tx) = p.crew.cancel.lock().as_ref() {
                            let _ = tx.send(true);
                        }
                        json!({"ok": true})
                    }
                    other => json!({"error": format!("commande inconnue : {other}")}),
                }
            })
        }));
}

/// Interactive Claude Code pane with harnes hooks (state + session id).
/// `resume` reopens a previous Claude session; `prompt` is the first message.
pub fn spawn_claude(
    st: &Shared,
    p: &Arc<Project>,
    key: &str,
    title: &str,
    agent: Option<&str>,
    resume: Option<&str>,
    prompt: Option<String>,
) -> Result<String, String> {
    harnes_guards::trust::check(&p.root)?;
    let settings = p
        .root
        .join(".harnes")
        .join("agents")
        .join(format!("{key}.settings.json"));
    std::fs::create_dir_all(settings.parent().unwrap()).map_err(|e| e.to_string())?;
    drop(st.hooks.registry.register(key));
    std::fs::write(
        &settings,
        st.hooks.settings_for_root(key, Some(&p.root)).to_string(),
    )
    .map_err(|e| e.to_string())?;
    let mut env = agent_env(p, key);
    if agent == Some("orchestrator") {
        env.push(("HARNES_ROLE".into(), "orchestrator".into()));
        env.push(("HARNES_API".into(), st.hooks.api_url()));
    }
    let mut args: Vec<String> = vec![];
    if let Some(a) = agent {
        args.extend(["--agent".into(), a.into()]);
    }
    if let Some(r) = resume {
        args.extend(["--resume".into(), r.into()]);
    }
    // Enforce the policy even when a project keeps an older agent definition:
    // only the Planner uses Opus; interactive panes use Sonnet high.
    let session = harnes_crew::budget::SESSION;
    args.extend([
        "--model".into(),
        session.model.into(),
        "--effort".into(),
        session.effort.into(),
        "--permission-mode".into(),
        "auto".into(),
        "--settings".into(),
        settings.display().to_string(),
    ]);
    args.extend(prompt);
    let info = st
        .panes
        .spawn(SpawnSpec {
            title: title.into(),
            program: "claude".into(),
            args,
            cwd: Some(p.root.clone()),
            env,
            cols: 120,
            rows: 32,
        })
        .map_err(|e| e.to_string())?;
    st.tag_pane(&info.id, &p.id);
    let role = agent.unwrap_or("claude").to_string();
    st.agents.write().insert(
        key.into(),
        AgentView {
            key: key.into(),
            project: p.id.clone(),
            pane: Some(info.id.clone()),
            pid: info.pid,
            role: role.clone(),
            provider: "claude".into(),
            state: AgentState::Starting,
            context: None,
        },
    );
    let _ = st.app.emit(
        "pane-open",
        json!({ "info": &info, "key": key, "role": role, "provider": "claude", "project": p.id }),
    );
    let _ = st.app.emit("projects-changed", ());
    Ok(info.id)
}

/// Sends a message to a project's orchestrator, starting its pane if needed.
#[tauri::command]
pub fn orchestrator_say(
    st: State<'_, Shared>,
    text: String,
    project: Option<String>,
) -> Result<String, String> {
    let p = st.pick(project.as_deref())?;
    let key = p.orch_key();
    let existing = st
        .agents
        .read()
        .get(&key)
        .and_then(|a| a.pane.clone())
        .filter(|x| st.panes.get(x).is_ok());
    if let Some(pane) = existing {
        // Bracketed paste keeps multi-line messages in one prompt, then submit.
        let data = format!(
            "{}{}{}",
            "\u{1b}[200~",
            text.replace("\r\n", "\n"),
            "\u{1b}[201~\r"
        );
        st.panes
            .write(&pane, data.as_bytes())
            .map_err(|e| e.to_string())?;
        return Ok(pane);
    }
    spawn_claude(
        st.inner(),
        &p,
        &key,
        "orchestrateur",
        Some("orchestrator"),
        None,
        Some(text),
    )
}

fn threads_of(st: &Shared, project: Option<&str>) -> Option<Arc<harnes_threads::Threads>> {
    st.pick(project).ok().and_then(|p| p.threads.clone())
}

#[tauri::command]
pub fn threads_list(
    st: State<'_, Shared>,
    status: String,
    project: Option<String>,
) -> Result<Value, String> {
    let Some(t) = threads_of(st.inner(), project.as_deref()) else {
        return Ok(json!([]));
    };
    t.list(&status, None, 200)
        .map(|l| json!(l))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn thread_read(
    st: State<'_, Shared>,
    id: String,
    project: Option<String>,
) -> Result<Value, String> {
    let Some(t) = threads_of(st.inner(), project.as_deref()) else {
        return Ok(json!({"thread": null, "messages": []}));
    };
    let th = t.get(&id).map_err(|e| e.to_string())?;
    let msgs = t.read(&id, 0, 1000).map_err(|e| e.to_string())?;
    Ok(json!({"thread": th, "messages": msgs}))
}

#[tauri::command]
pub fn thread_post(
    st: State<'_, Shared>,
    id: String,
    body: String,
    project: Option<String>,
) -> Result<(), String> {
    let t = threads_of(st.inner(), project.as_deref()).ok_or("aucun projet ouvert")?;
    t.post(&id, "user", "message", &body)
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn threads_recent(
    st: State<'_, Shared>,
    since: i64,
    project: Option<String>,
) -> Result<Value, String> {
    let Some(t) = threads_of(st.inner(), project.as_deref()) else {
        return Ok(json!([]));
    };
    t.recent(since, 40)
        .map(|l| json!(l))
        .map_err(|e| e.to_string())
}
