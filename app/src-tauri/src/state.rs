//! Application state: several projects open at once, one of them active in the UI.

use harnes_crew::{hooks::HookServer, AgentState, CrewEvent};
use harnes_ctx::Store;
use harnes_proxy::ProxyHandle;
use harnes_pty::{PaneEvent, PaneManager};
use parking_lot::{Mutex, RwLock};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};
use tauri::{ipc::Channel, ipc::InvokeResponseBody, AppHandle, Emitter};

#[derive(Debug, Clone, Serialize)]
pub struct AgentView {
    pub key: String,
    pub project: String,
    pub pane: Option<String>,
    pub pid: Option<u32>,
    pub role: String,
    pub provider: String,
    pub state: AgentState,
    /// Last context hand-off file (project-relative), if the session changed provider.
    pub context: Option<String>,
}

/// Crew run state of one project.
pub struct CrewState {
    pub cancel: Mutex<Option<tokio::sync::watch::Sender<bool>>>,
    pub log: Mutex<Vec<CrewEvent>>,
    pub tick: tokio::sync::watch::Sender<u64>,
    /// (run id, task) of the run in progress, persisted for resume.
    pub task: Mutex<Option<(String, String)>>,
}

impl Default for CrewState {
    fn default() -> Self {
        Self {
            cancel: Mutex::new(None),
            log: Mutex::new(Vec::new()),
            tick: tokio::sync::watch::channel(0).0,
            task: Mutex::new(None),
        }
    }
}

impl CrewState {
    pub fn running(&self) -> bool {
        self.cancel.lock().is_some()
    }
}

pub struct Project {
    /// Short stable id derived from the path.
    pub id: String,
    pub name: String,
    pub root: PathBuf,
    pub store: Arc<Store>,
    pub proxy: ProxyHandle,
    pub threads: Option<Arc<harnes_threads::Threads>>,
    pub crew: CrewState,
    pub telemetry: Arc<harnes_telemetry::Telemetry>,
    pub memory: Arc<harnes_memory::Memory>,
    pub guards: Arc<RwLock<harnes_guards::Guard>>,
    pub worktrees: harnes_worktree::Worktrees,
    /// Run id currently producing proxy usage events.
    pub run_slot: Arc<Mutex<Option<String>>>,
}

impl Project {
    /// Key of this project's orchestrator agent.
    pub fn orch_key(&self) -> String {
        format!("orch-{}", self.id)
    }
}

/// Stable short id for a project path (case-insensitive on Windows).
pub fn project_id(root: &std::path::Path) -> String {
    let norm = root
        .display()
        .to_string()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_lowercase();
    let mut h: u64 = 0xcbf29ce484222325;
    for b in norm.bytes() {
        h = (h ^ b as u64).wrapping_mul(0x100000001b3);
    }
    format!("{:08x}", h as u32)
}

pub struct AppState {
    pub app: AppHandle,
    pub panes: Arc<PaneManager>,
    pub hooks: Arc<HookServer>,
    pub projects: RwLock<Vec<Arc<Project>>>,
    pub active: RwLock<Option<String>>,
    /// pane id -> project id, for every pane (agents and terminals).
    pub pane_project: RwLock<HashMap<String, String>>,
    pub agents: RwLock<HashMap<String, AgentView>>,
    pub dirty: Arc<Mutex<HashSet<String>>>,
    pub frames: Mutex<Option<Channel<InvokeResponseBody>>>,
    pub lookout: Mutex<harnes_lookout::Collector>,
}

pub type Shared = Arc<AppState>;

impl AppState {
    pub async fn new(app: AppHandle) -> anyhow::Result<Shared> {
        let dirty: Arc<Mutex<HashSet<String>>> = Arc::default();

        // Hook events update the agent list and are forwarded to the UI.
        let app_h = app.clone();
        let hooks = Arc::new(
            HookServer::start(Arc::new(move |key: &str, st: AgentState| {
                use tauri::Manager;
                if let Some(s) = app_h.try_state::<Shared>() {
                    if let Some(a) = s.agents.write().get_mut(key) {
                        a.state = st;
                    }
                    if let Some(p) = s.project_of_key(key) {
                        let role = s
                            .agents
                            .read()
                            .get(key)
                            .map(|a| a.role.clone())
                            .unwrap_or_default();
                        let run = p.run_slot.lock().clone().unwrap_or_default();
                        let _ = p.telemetry.append(
                            harnes_telemetry::Event::new(&run, &p.id, "agent.state")
                                .role(role)
                                .status(format!("{st:?}").to_lowercase())
                                .summary(format!("{key} → {st:?}")),
                        );
                    }
                }
                let _ = app_h.emit(
                    "agent-state",
                    serde_json::json!({ "key": key, "state": st }),
                );
                // The orchestrator's transcript changes with each of its hook events.
                if let Some(project) = key.strip_prefix("orch-") {
                    let _ = app_h.emit("conversation-changed", project);
                }
            }))
            .await?,
        );

        let (d, app_p) = (dirty.clone(), app.clone());
        let hooks_p = hooks.clone();
        let panes = Arc::new(PaneManager::new(Arc::new(move |ev| match ev {
            PaneEvent::Dirty(id) => {
                d.lock().insert(id);
            }
            PaneEvent::Exited(id, code) => {
                let _ = app_p.emit("pane-exit", serde_json::json!({ "id": id, "code": code }));
                // An agent whose process died will never send SessionEnd.
                if let Some(key) = id_to_key(&app_p, &id) {
                    hooks_p.registry.set(&key, AgentState::Exited, false);
                }
            }
            PaneEvent::Output(..) => {}
        })));

        Ok(Arc::new(Self {
            app,
            panes,
            hooks,
            projects: RwLock::new(Vec::new()),
            active: RwLock::new(None),
            pane_project: RwLock::new(HashMap::new()),
            agents: RwLock::new(HashMap::new()),
            dirty,
            frames: Mutex::new(None),
            lookout: Mutex::new(harnes_lookout::Collector::default()),
        }))
    }

    pub fn project(&self, id: &str) -> Option<Arc<Project>> {
        self.projects.read().iter().find(|p| p.id == id).cloned()
    }

    /// The project shown in the UI.
    pub fn active(&self) -> Option<Arc<Project>> {
        let id = self.active.read().clone()?;
        self.project(&id)
    }

    pub fn active_or_err(&self) -> Result<Arc<Project>, String> {
        self.active()
            .ok_or_else(|| "aucun projet ouvert".to_string())
    }

    /// `id` if given, else the active project.
    pub fn pick(&self, id: Option<&str>) -> Result<Arc<Project>, String> {
        match id {
            Some(i) => self
                .project(i)
                .ok_or_else(|| format!("projet inconnu : {i}")),
            None => self.active_or_err(),
        }
    }

    pub fn project_of_pane(&self, pane: &str) -> Option<Arc<Project>> {
        let id = self.pane_project.read().get(pane).cloned()?;
        self.project(&id)
    }

    /// Project owning an agent key (`orch-<id>` or an entry in `agents`).
    pub fn project_of_key(&self, key: &str) -> Option<Arc<Project>> {
        if let Some(id) = key.strip_prefix("orch-") {
            return self.project(id);
        }
        let id = self.agents.read().get(key).map(|a| a.project.clone())?;
        self.project(&id)
    }

    /// An agent whose pane goes away can no longer read an answer: free its questions.
    pub fn cancel_asks_of_pane(&self, pane: &str) {
        let keys: Vec<String> = self
            .agents
            .read()
            .values()
            .filter(|a| a.pane.as_deref() == Some(pane))
            .map(|a| a.key.clone())
            .collect();
        for key in keys {
            self.hooks.asks.cancel_agent(&key);
        }
    }

    pub fn tag_pane(&self, pane: &str, project: &str) {
        self.pane_project
            .write()
            .insert(pane.to_string(), project.to_string());
    }
}

fn id_to_key(app: &AppHandle, pane: &str) -> Option<String> {
    use tauri::Manager;
    let st = app.try_state::<Shared>()?;
    let agents = st.agents.read();
    agents
        .values()
        .find(|a| a.pane.as_deref() == Some(pane))
        .map(|a| a.key.clone())
}

/// Path of the `harnes-ctx` MCP binary shipped next to the app executable.
pub fn ctx_bin() -> PathBuf {
    let exe = std::env::current_exe().unwrap_or_default();
    let name = if cfg!(windows) {
        "harnes-ctx.exe"
    } else {
        "harnes-ctx"
    };
    exe.parent()
        .map(|d| d.join(name))
        .unwrap_or_else(|| PathBuf::from(name))
}

#[cfg(test)]
mod tests {
    #[test]
    fn project_ids_are_stable_and_case_insensitive() {
        let a = super::project_id(std::path::Path::new("C:\\Work\\App"));
        let b = super::project_id(std::path::Path::new("c:/work/app/"));
        assert_eq!(a, b);
        assert_ne!(a, super::project_id(std::path::Path::new("C:/work/other")));
    }
}
