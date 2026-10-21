//! Pane commands called by the UI.

use crate::{pump, state::Shared};
use harnes_pty::{PaneInfo, SpawnSpec};
use serde_json::{json, Value};
use tauri::{ipc::Channel, ipc::InvokeResponseBody, Emitter, State};

type Res<T> = Result<T, String>;

fn e(err: impl std::fmt::Display) -> String {
    format!("{err:#}")
}

pub fn shell_spec(title: &str) -> SpawnSpec {
    SpawnSpec {
        title: title.into(),
        program: if cfg!(windows) {
            "powershell.exe".into()
        } else {
            "bash".into()
        },
        args: if cfg!(windows) {
            vec!["-NoLogo".into()]
        } else {
            vec![]
        },
        cols: 120,
        rows: 32,
        ..Default::default()
    }
}

pub fn short_id() -> String {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{:06x}", (n as u64) & 0xffffff)
}

/// New shell or Claude session in a project (default: the active one).
#[tauri::command]
pub fn pane_spawn(
    st: State<'_, Shared>,
    kind: String,
    cols: u16,
    rows: u16,
    project: Option<String>,
) -> Res<PaneInfo> {
    let p = st.pick(project.as_deref())?;
    if kind == "claude" {
        let key = format!("claude-{}", short_id());
        let id = crate::crew_ctl::spawn_claude(st.inner(), &p, &key, "claude", None, None, None)?;
        return st.panes.get(&id).map(|x| x.info()).map_err(e);
    }
    let info = st
        .panes
        .spawn(SpawnSpec {
            cwd: Some(p.root.clone()),
            cols,
            rows,
            ..shell_spec("shell")
        })
        .map_err(e)?;
    st.tag_pane(&info.id, &p.id);
    let _ = st.app.emit("projects-changed", ());
    Ok(info)
}

#[tauri::command]
pub fn pane_write(st: State<'_, Shared>, id: String, data: String) -> Res<()> {
    st.panes.write(&id, data.as_bytes()).map_err(e)
}

#[tauri::command]
pub fn pane_resize(
    st: State<'_, Shared>,
    id: String,
    cols: u16,
    rows: u16,
    cell_w: u32,
    cell_h: u32,
) -> Res<()> {
    st.panes
        .resize(&id, cols, rows, cell_w, cell_h)
        .map_err(e)?;
    st.dirty.lock().insert(id);
    Ok(())
}

#[tauri::command]
pub fn pane_scroll(st: State<'_, Shared>, id: String, delta: isize) -> Res<()> {
    st.panes.get(&id).map_err(e)?.scroll(delta);
    st.dirty.lock().insert(id);
    Ok(())
}

#[tauri::command]
pub fn pane_close(st: State<'_, Shared>, id: String) -> Res<()> {
    st.panes.close(&id).map_err(e)?;
    st.pane_project.write().remove(&id);
    st.agents
        .write()
        .retain(|_, a| a.pane.as_deref() != Some(id.as_str()));
    let _ = st.app.emit("projects-changed", ());
    Ok(())
}

/// Panes with their project and agent identity (key, role, provider, state).
#[tauri::command]
pub fn pane_list(st: State<'_, Shared>) -> Vec<Value> {
    let agents = st.agents.read();
    let owners = st.pane_project.read();
    st.panes
        .list()
        .into_iter()
        .map(|p| {
            let mut v = json!(p);
            v["project"] = json!(owners.get(&p.id));
            if let Some(a) = agents
                .values()
                .find(|a| a.pane.as_deref() == Some(p.id.as_str()))
            {
                v["key"] = json!(a.key);
                v["role"] = json!(a.role);
                v["provider"] = json!(a.provider);
                v["state"] = json!(a.state);
            }
            v
        })
        .collect()
}

/// Registers the frame channel and sends a full frame of every pane.
#[tauri::command]
pub fn frames_subscribe(st: State<'_, Shared>, channel: Channel<InvokeResponseBody>) -> Res<()> {
    for p in st.panes.list() {
        if let Ok(Some(f)) = st.panes.snapshot(&p.id, true) {
            let _ = channel.send(InvokeResponseBody::Raw(pump::message(&p.id, &f)));
        }
    }
    *st.frames.lock() = Some(channel);
    Ok(())
}

#[tauri::command]
pub fn pane_refresh(st: State<'_, Shared>, id: String) -> Res<()> {
    let frame = st.panes.snapshot(&id, true).map_err(e)?;
    if let (Some(f), Some(chan)) = (frame, st.frames.lock().clone()) {
        let _ = chan.send(InvokeResponseBody::Raw(pump::message(&id, &f)));
    }
    Ok(())
}

#[tauri::command]
pub fn crew_start(st: State<'_, Shared>, task: String, project: Option<String>) -> Res<()> {
    let p = st.pick(project.as_deref())?;
    crate::crew_ctl::start_crew(st.inner(), &p, task, None, Default::default())
}

#[tauri::command]
pub fn crew_cancel(st: State<'_, Shared>, project: Option<String>) -> Res<()> {
    let p = st.pick(project.as_deref())?;
    if let Some(tx) = p.crew.cancel.lock().as_ref() {
        let _ = tx.send(true);
    }
    Ok(())
}

#[tauri::command]
pub fn lookout(st: State<'_, Shared>) -> Value {
    let c = st.lookout.lock();
    json!({ "snapshot": c.snapshot(), "events": c.events(50), "history": c.history() })
}
