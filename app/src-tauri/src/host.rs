//! Runs crew agents in panes of their project.

use crate::state::{AgentView, Project, Shared};
use harnes_crew::{budget::Tune, engine::AgentHost, providers::Providers, AgentState, Role};
use harnes_pty::SpawnSpec;
use std::{path::Path, sync::Arc};
use tauri::Emitter;

/// Environment for every agent pane: the project's proxy, tool search on,
/// identity for threads (`HARNES_AGENT`) and project for crew control.
pub fn agent_env(p: &Project, key: &str) -> Vec<(String, String)> {
    vec![
        ("ENABLE_TOOL_SEARCH".into(), "true".into()),
        ("HARNES_AGENT".into(), key.into()),
        ("HARNES_PROJECT".into(), p.id.clone()),
        ("ANTHROPIC_BASE_URL".into(), p.proxy.url()),
        ("HARNES_OS_DIR".into(), harnes_os::default_dir().display().to_string()),
    ]
}

pub struct PaneHost(pub Shared, pub Providers, pub Arc<Project>);

impl AgentHost for PaneHost {
    fn spawn(&self, key: &str, role: Role, prompt: &str, settings: &Path, tune: Option<Tune>) -> anyhow::Result<String> {
        let (st, p) = (&self.0, &self.2);
        let mut launch = self.1.launch(role, prompt, settings)?;
        // The run's mode or the week's pace can pick another model than the role's own.
        if let (Some(t), "claude") = (tune, launch.provider.as_str()) {
            let at = launch.args.len().saturating_sub(1); // before the prompt
            launch.args.splice(at..at, ["--model".into(), t.model.into(), "--effort".into(), t.effort.into()]);
        }
        let info = st.panes.spawn(SpawnSpec {
            title: key.split_once('-').map(|(_, n)| n).unwrap_or(key).to_string(),
            program: launch.program,
            args: launch.args,
            cwd: Some(p.root.clone()),
            env: agent_env(p, key),
            cols: 120,
            rows: 32,
        })?;
        st.tag_pane(&info.id, &p.id);
        let view = AgentView {
            key: key.into(),
            project: p.id.clone(),
            pane: Some(info.id.clone()),
            pid: info.pid,
            role: role.agent().into(),
            provider: launch.provider.clone(),
            state: AgentState::Starting,
            context: None,
        };
        st.agents.write().insert(key.into(), view);
        let _ = st.app.emit(
            "pane-open",
            serde_json::json!({ "info": &info, "key": key, "role": role.agent(), "provider": launch.provider, "project": p.id }),
        );
        Ok(info.id)
    }

    fn has_hooks(&self, role: Role) -> bool {
        self.1.launch(role, "", Path::new("")).map(|l| l.hooks).unwrap_or(true)
    }

    fn close(&self, pane: &str) {
        let st = &self.0;
        // Keep the pane a moment so the user sees the final message.
        let (shared, pane) = (st.clone(), pane.to_string());
        st.agents.write().retain(|_, a| a.pane.as_deref() != Some(pane.as_str()));
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(4)).await;
            let _ = shared.panes.close(&pane);
            shared.pane_project.write().remove(&pane);
            let _ = shared.app.emit("pane-closed", &pane);
        });
    }
}
