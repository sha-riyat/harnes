//! Agent state from Claude Code hooks.
//!
//! Each agent is launched with `--settings <file>` whose hooks POST the hook
//! payload to this local server. That gives exact lifecycle events (turn
//! start, tool use, permission prompt, turn end) instead of screen scraping.

use crate::AgentState;
use anyhow::Result;
use axum::{
    body::Bytes,
    extract::{Path, State},
    routing::post,
    Router,
};
use parking_lot::Mutex;
use serde_json::{json, Value};
use std::path::{Path as FsPath, PathBuf};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::watch;

/// Latest state plus a counter of finished turns (Stop events).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Turn {
    pub state: AgentState,
    pub stops: u64,
}

pub type StateSink = Arc<dyn Fn(&str, AgentState) + Send + Sync>;

/// Asked for a PreToolUse decision; returns a Claude-compatible JSON object.
pub type GuardFn = Arc<dyn Fn(&str, &Value) -> Value + Send + Sync>;

struct HookState {
    registry: Arc<Registry>,
    guard: Arc<Mutex<Option<GuardFn>>>,
}

pub struct Registry {
    agents: Mutex<HashMap<String, watch::Sender<Turn>>>,
    /// Claude Code session id per agent key (from SessionStart), for `--resume`.
    sessions: Mutex<HashMap<String, String>>,
    sink: StateSink,
}

impl Registry {
    pub fn register(&self, key: &str) -> watch::Receiver<Turn> {
        let (tx, rx) = watch::channel(Turn {
            state: AgentState::Starting,
            stops: 0,
        });
        self.agents.lock().insert(key.to_string(), tx);
        (self.sink)(key, AgentState::Starting);
        rx
    }

    pub fn subscribe(&self, key: &str) -> Option<watch::Receiver<Turn>> {
        self.agents.lock().get(key).map(|tx| tx.subscribe())
    }

    pub fn forget(&self, key: &str) {
        self.agents.lock().remove(key);
    }

    pub fn set(&self, key: &str, state: AgentState, stopped: bool) {
        let agents = self.agents.lock();
        let Some(tx) = agents.get(key) else { return };
        tx.send_modify(|t| {
            t.state = state;
            if stopped {
                t.stops += 1;
            }
        });
        drop(agents);
        (self.sink)(key, state);
    }

    pub fn session_of(&self, key: &str) -> Option<String> {
        self.sessions.lock().get(key).cloned()
    }

    pub fn apply_hook(&self, key: &str, payload: &Value) {
        let event = payload["hook_event_name"].as_str().unwrap_or("");
        if let Some(sid) = payload["session_id"].as_str().filter(|s| !s.is_empty()) {
            self.sessions
                .lock()
                .insert(key.to_string(), sid.to_string());
        }
        // Subagent events come from inside the same session; ignore them.
        if payload
            .get("agent_id")
            .and_then(Value::as_str)
            .is_some_and(|s| !s.is_empty())
        {
            return;
        }
        match event {
            "SessionStart" => self.set(key, AgentState::Idle, false),
            "UserPromptSubmit" | "PreToolUse" | "PostToolUse" => {
                self.set(key, AgentState::Working, false)
            }
            "Notification" => self.set(key, AgentState::Blocked, false),
            "Stop" => self.set(key, AgentState::Done, true),
            "SessionEnd" => self.set(key, AgentState::Exited, false),
            _ => {}
        }
    }
}

/// Handles `POST /api/<cmd>` from the orchestrator's MCP tools. Async because
/// `status` may wait for the next crew change.
pub type ControlFn = Arc<
    dyn Fn(String, Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = Value> + Send>>
        + Send
        + Sync,
>;

pub struct HookServer {
    pub port: u16,
    pub registry: Arc<Registry>,
    /// Permission requests waiting for the user (answered from the UI).
    pub permissions: Arc<crate::permissions::Broker>,
    control: Arc<std::sync::OnceLock<ControlFn>>,
    guard: Arc<Mutex<Option<GuardFn>>>,
}

impl HookServer {
    pub async fn start(sink: StateSink) -> Result<Self> {
        let registry = Arc::new(Registry {
            agents: Mutex::new(HashMap::new()),
            sessions: Mutex::new(HashMap::new()),
            sink,
        });
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let port = listener.local_addr()?.port();
        let control: Arc<std::sync::OnceLock<ControlFn>> = Arc::default();
        let guard: Arc<Mutex<Option<GuardFn>>> = Arc::default();
        let ctl = control.clone();
        let permissions: Arc<crate::permissions::Broker> = Arc::default();
        let (perm, reg2) = (permissions.clone(), registry.clone());
        let state = Arc::new(HookState {
            registry: registry.clone(),
            guard: guard.clone(),
        });
        let app = Router::new()
            .route(
                "/permission/{key}",
                post(move |Path(key): Path<String>, body: Bytes| {
                    let (perm, reg) = (perm.clone(), reg2.clone());
                    async move {
                        let v: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
                        reg.set(&key, AgentState::Blocked, false);
                        let out = perm.request(&key, v).await;
                        reg.set(&key, AgentState::Working, false);
                        out
                    }
                }),
            )
            .route("/hook/{key}", post(receive))
            .with_state(state)
            .route(
                "/api/{cmd}",
                post(move |Path(cmd): Path<String>, body: Bytes| {
                    let ctl = ctl.clone();
                    async move {
                        let v: Value = serde_json::from_slice(&body).unwrap_or(Value::Null);
                        match ctl.get() {
                            Some(f) => f(cmd, v).await.to_string(),
                            None => json!({"error": "harnes pas prêt"}).to_string(),
                        }
                    }
                }),
            );
        tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });
        Ok(Self {
            port,
            registry,
            permissions,
            control,
            guard,
        })
    }

    /// Installs the crew control handler (once).
    pub fn set_control(&self, f: ControlFn) {
        let _ = self.control.set(f);
    }

    /// Installs the runtime guard consulted on PreToolUse.
    pub fn set_guard(&self, f: GuardFn) {
        *self.guard.lock() = Some(f);
    }

    pub fn api_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// `--settings` JSON for an agent: lifecycle hooks report to this server.
    /// The native hook (the cockpit's `__hook` command or `harnes-hook`) makes
    /// the PreToolUse decision from `.harnes/guards.json`; it reports the event
    /// first so the trace still sees it. Without a native hook we keep curl.
    pub fn settings_for(&self, key: &str) -> Value {
        self.settings_for_root(key, None)
    }

    pub fn settings_for_root(&self, key: &str, root: Option<&FsPath>) -> Value {
        let url = format!("{}/hook/{key}", self.api_url());
        let native = native_hook().map(|bin| native_hook_command(&bin, &url, root));
        let cmd = native.unwrap_or_else(|| {
            // Double quotes only: the command runs under Git Bash or cmd.exe.
            format!(
                "curl -s -m 3 -X POST -H \"Content-Type: application/json\" --data-binary @- {url}"
            )
        });
        let hook = json!([{ "hooks": [{ "type": "command", "command": cmd, "timeout": 5 }] }]);
        let mut hooks = serde_json::Map::new();
        for ev in [
            "SessionStart",
            "UserPromptSubmit",
            "PreToolUse",
            "PostToolUse",
            "Notification",
            "Stop",
            "SessionEnd",
        ] {
            hooks.insert(ev.into(), hook.clone());
        }
        // Blocking: the hook's stdout is the user's decision taken in the harnes UI.
        let ask = format!(
            "curl -s -m 590 -X POST -H \"Content-Type: application/json\" --data-binary @- {}/permission/{key}",
            self.api_url()
        );
        hooks.insert(
            "PermissionRequest".into(),
            json!([{ "hooks": [{ "type": "command", "command": ask, "timeout": 600 }] }]),
        );
        json!({ "hooks": hooks })
    }
}

fn quote(s: &str) -> String {
    format!("\"{}\"", s.replace('"', "\\\""))
}

/// The native guard executable. In the cockpit it is the cockpit itself
/// (`harnes __hook`), so release packaging needs no sidecar. The standalone
/// `harnes-hook` is still supported for headless use and tests.
fn native_hook() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("HARNES_HOOK_BIN").map(PathBuf::from) {
        if p.is_file() {
            return Some(p);
        }
    }
    let exe = std::env::current_exe().ok()?;
    let stem = exe.file_stem()?.to_string_lossy();
    if stem == "harnes" || stem == "harnes-hook" {
        return Some(exe);
    }
    let name = if cfg!(windows) {
        "harnes-hook.exe"
    } else {
        "harnes-hook"
    };
    exe.ancestors()
        .skip(1)
        .take(3)
        .map(|d| d.join(name))
        .find(|p| p.is_file())
}

fn native_hook_command(bin: &FsPath, url: &str, root: Option<&FsPath>) -> String {
    let mut cmd = quote(&bin.display().to_string());
    if bin.file_stem().is_some_and(|s| s == "harnes") {
        cmd.push_str(" __hook");
    }
    cmd.push_str(" --forward ");
    cmd.push_str(&quote(url));
    if let Some(root) = root {
        cmd.push_str(" --root ");
        cmd.push_str(&quote(&root.display().to_string()));
    }
    cmd
}

async fn receive(
    State(state): State<Arc<HookState>>,
    Path(key): Path<String>,
    body: Bytes,
) -> axum::Json<Value> {
    let Ok(v) = serde_json::from_slice::<Value>(&body) else {
        return axum::Json(json!({}));
    };
    state.registry.apply_hook(&key, &v);
    if let Some(guard) = state.guard.lock().as_ref() {
        let out = guard(&key, &v);
        if v.get("hook_event_name").and_then(Value::as_str) == Some("PreToolUse") {
            return axum::Json(out);
        }
    }
    axum::Json(json!({}))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn hook_posts_drive_state() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let s2 = seen.clone();
        let srv = HookServer::start(Arc::new(move |k: &str, st| {
            s2.lock().push((k.to_string(), st))
        }))
        .await
        .unwrap();
        let mut rx = srv.registry.register("planner-1");
        let url = format!("http://127.0.0.1:{}/hook/planner-1", srv.port);
        let client = reqwest_like_post(url);
        client(json!({"hook_event_name":"UserPromptSubmit","session_id":"abc-123"})).await;
        client(json!({"hook_event_name":"Stop"})).await;
        rx.changed().await.unwrap();
        let t = *rx.borrow();
        assert_eq!(t.stops, 1);
        assert_eq!(
            srv.registry.session_of("planner-1").as_deref(),
            Some("abc-123")
        );
        assert_eq!(t.state, AgentState::Done);
        assert!(seen.lock().iter().any(|(_, s)| *s == AgentState::Working));
        assert!(
            srv.settings_for("x")["hooks"]["Stop"][0]["hooks"][0]["command"]
                .as_str()
                .unwrap()
                .contains("/hook/x")
        );
    }

    /// Tiny HTTP POST without extra dependencies.
    fn reqwest_like_post(
        url: String,
    ) -> impl Fn(Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()>>> {
        move |body: Value| {
            let url = url.clone();
            Box::pin(async move {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                let rest = url.trim_start_matches("http://");
                let (host, path) = rest.split_once('/').unwrap();
                let mut s = tokio::net::TcpStream::connect(host).await.unwrap();
                let b = body.to_string();
                let req = format!("POST /{path} HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{b}", b.len());
                s.write_all(req.as_bytes()).await.unwrap();
                let mut out = Vec::new();
                s.read_to_end(&mut out).await.unwrap();
            })
        }
    }
}
