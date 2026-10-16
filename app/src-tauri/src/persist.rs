//! Saves the workspace continuously and restores it at startup: every open
//! project, its terminals and Claude sessions (`--resume`), its unfinished run,
//! and which project was active.

use crate::{commands::shell_spec, crew_ctl, projects, state::Shared, switch};
use harnes_pty::SpawnSpec;
use serde::{Deserialize, Serialize};
use std::{path::PathBuf, time::Duration};
use tauri::{Emitter, Manager};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct PaneRecord {
    /// shell | claude | orchestrator
    pub kind: String,
    pub title: String,
    pub cwd: String,
    #[serde(default)]
    pub key: Option<String>,
    /// Claude Code session id, for `claude --resume`.
    #[serde(default)]
    pub session: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    /// Last hand-off file, used to restore non-Claude sessions with their context.
    #[serde(default)]
    pub context: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ProjectRecord {
    pub root: String,
    pub panes: Vec<PaneRecord>,
    /// (run id, task) of an unfinished crew run.
    pub crew: Option<(String, String)>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SessionFile {
    pub version: u32,
    #[serde(default)]
    pub projects: Vec<ProjectRecord>,
    #[serde(default)]
    pub active: Option<String>,
    // v1 (single project), migrated on load.
    #[serde(default, skip_serializing)]
    pub project: Option<String>,
    #[serde(default, skip_serializing)]
    pub panes: Vec<PaneRecord>,
    #[serde(default, skip_serializing)]
    pub crew: Option<(String, String)>,
}

impl SessionFile {
    pub fn migrate(mut self) -> Self {
        if self.version < 2 {
            if let Some(root) = self.project.take() {
                self.projects = vec![ProjectRecord { root: root.clone(), panes: std::mem::take(&mut self.panes), crew: self.crew.take() }];
                self.active = Some(root);
            }
            self.version = 2;
        }
        self
    }
}

fn path(st: &Shared) -> Option<PathBuf> {
    st.app.path().app_data_dir().ok().map(|d| d.join("session.json"))
}

/// Current workspace. Crew agents are not recorded: the engine resumes them from the run directory.
pub fn snapshot(st: &Shared) -> SessionFile {
    let agents = st.agents.read();
    let owners = st.pane_project.read();
    let all_panes = st.panes.list();
    let projects = st
        .projects
        .read()
        .iter()
        .map(|p| {
            let panes = all_panes
                .iter()
                .filter(|x| owners.get(&x.id) == Some(&p.id))
                .filter_map(|x| match agents.values().find(|a| a.pane.as_deref() == Some(x.id.as_str())) {
                    Some(a) if a.key == p.orch_key() || a.key.starts_with("claude-") => Some(PaneRecord {
                        kind: if a.key == p.orch_key() { "orchestrator".into() } else { "claude".into() },
                        title: x.title.clone(),
                        cwd: x.cwd.clone(),
                        key: Some(a.key.clone()),
                        session: st.hooks.registry.session_of(&a.key),
                        provider: Some(a.provider.clone()),
                        context: a.context.clone(),
                    }),
                    Some(_) => None, // crew agent
                    None => Some(PaneRecord { kind: "shell".into(), title: x.title.clone(), cwd: x.cwd.clone(), ..Default::default() }),
                })
                .collect();
            ProjectRecord { root: p.root.display().to_string(), panes, crew: p.crew.task.lock().clone() }
        })
        .collect();
    SessionFile { version: 2, projects, active: st.active().map(|p| p.root.display().to_string()), ..Default::default() }
}

fn write(path: &PathBuf, f: &SessionFile) {
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let tmp = path.with_extension("json.tmp");
    if std::fs::write(&tmp, serde_json::to_string_pretty(f).unwrap_or_default()).is_ok() {
        let _ = std::fs::rename(&tmp, path);
    }
}

/// Writes the session file whenever it changes (checked every 3 s).
pub fn start_autosave(st: Shared) {
    tauri::async_runtime::spawn(async move {
        let Some(p) = path(&st) else { return };
        let mut last = snapshot(&st);
        loop {
            tokio::time::sleep(Duration::from_secs(3)).await;
            let now = snapshot(&st);
            if now != last {
                write(&p, &now);
                last = now;
            }
        }
    });
}

pub fn save_now(st: &Shared) {
    if let Some(p) = path(st) {
        write(&p, &snapshot(st));
    }
}

/// Restores every project of the last session. Best-effort per piece.
pub async fn restore(st: Shared) {
    let Some(p) = path(&st) else { return };
    let Ok(raw) = std::fs::read_to_string(&p) else { return };
    let Ok(file) = serde_json::from_str::<SessionFile>(&raw).map(SessionFile::migrate) else { return };
    let (mut opened, mut panes, mut runs, mut errors) = (0, 0, 0, vec![]);
    for rec in &file.projects {
        let project = match projects::open(&st, &rec.root, false, false).await {
            Ok((id, _)) => st.project(&id).expect("just opened"),
            Err(e) => {
                errors.push(e);
                continue;
            }
        };
        opened += 1;
        for pr in &rec.panes {
            let provider = pr.provider.clone().unwrap_or_else(|| "claude".into());
            let ok = match pr.kind.as_str() {
                // Non-Claude sessions cannot be resumed natively: restart them from their last hand-off.
                "orchestrator" | "claude" if provider != "claude" => {
                    let key = pr.key.clone().unwrap_or_else(|| project.orch_key());
                    let prompt = pr.context.as_deref().map(harnes_crew::context::resume_prompt).unwrap_or_else(|| "Reprends le travail en cours du projet.".into());
                    switch::spawn_on(&st, &project, &key, &pr.title, &provider, prompt, pr.context.clone()).is_ok()
                }
                "orchestrator" => crew_ctl::spawn_claude(&st, &project, &project.orch_key(), "orchestrateur", Some("orchestrator"), pr.session.as_deref(), None).is_ok(),
                "claude" => {
                    let key = pr.key.clone().unwrap_or_else(|| format!("claude-{}", crate::commands::short_id()));
                    crew_ctl::spawn_claude(&st, &project, &key, &pr.title, None, pr.session.as_deref(), None).is_ok()
                }
                _ => {
                    let cwd = Some(PathBuf::from(&pr.cwd)).filter(|d| d.is_dir()).unwrap_or_else(|| project.root.clone());
                    match st.panes.spawn(SpawnSpec { cwd: Some(cwd), ..shell_spec(&pr.title) }) {
                        Ok(info) => {
                            st.tag_pane(&info.id, &project.id);
                            let _ = st.app.emit("pane-open", serde_json::json!({ "info": info, "project": project.id }));
                            true
                        }
                        Err(_) => false,
                    }
                }
            };
            panes += ok as usize;
        }
        if let Some((run, task)) = &rec.crew {
            runs += crew_ctl::start_crew(&st, &project, task.clone(), Some(run.clone())).is_ok() as usize;
        }
    }
    if let Some(active) = &file.active {
        let id = crate::state::project_id(std::path::Path::new(active));
        if st.project(&id).is_some() {
            projects::set_active(&st, &id);
        }
    }
    let _ = st.app.emit("projects-changed", ());
    let _ = st.app.emit("restore", serde_json::json!({ "projects": opened, "panes": panes, "runs": runs, "errors": errors }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrates_v1_and_roundtrips_v2() {
        let v1: SessionFile = serde_json::from_str(r#"{"version":1,"project":"C:/p","panes":[{"kind":"shell","title":"s","cwd":"."}],"crew":["42","task"]}"#).unwrap();
        let v2 = v1.migrate();
        assert_eq!(v2.projects.len(), 1);
        assert_eq!(v2.projects[0].panes.len(), 1);
        assert_eq!(v2.projects[0].crew, Some(("42".into(), "task".into())));
        assert_eq!(v2.active.as_deref(), Some("C:/p"));
        let back: SessionFile = serde_json::from_str(&serde_json::to_string(&v2).unwrap()).unwrap();
        assert_eq!(back.migrate(), v2);
    }
}
