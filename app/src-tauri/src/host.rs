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
        ("HARNES_PROJECT_ROOT".into(), p.root.display().to_string()),
        ("ANTHROPIC_BASE_URL".into(), p.proxy.url()),
        (
            "HARNES_OS_DIR".into(),
            harnes_os::default_dir().display().to_string(),
        ),
    ]
}

pub struct PaneHost(pub Shared, pub Providers, pub Arc<Project>);

impl AgentHost for PaneHost {
    fn spawn(
        &self,
        key: &str,
        role: Role,
        prompt: &str,
        settings: &Path,
        tune: Option<Tune>,
    ) -> anyhow::Result<String> {
        self.spawn_in(key, role, prompt, settings, tune, None)
    }

    fn spawn_in(
        &self,
        key: &str,
        role: Role,
        prompt: &str,
        settings: &Path,
        tune: Option<Tune>,
        cwd: Option<&Path>,
    ) -> anyhow::Result<String> {
        let (st, p) = (&self.0, &self.2);
        let mut launch = self.1.launch(role, prompt, settings)?;
        // The Planner is the only role on Opus; all others run Sonnet high.
        if let (Some(t), "claude") = (tune, launch.provider.as_str()) {
            let at = launch.args.len().saturating_sub(1); // before the prompt
            launch.args.splice(
                at..at,
                [
                    "--model".into(),
                    t.model.into(),
                    "--effort".into(),
                    t.effort.into(),
                ],
            );
        }
        let dir = cwd
            .map(|c| c.to_path_buf())
            .unwrap_or_else(|| p.root.clone());
        let info = st.panes.spawn(SpawnSpec {
            title: key
                .split_once('-')
                .map(|(_, n)| n)
                .unwrap_or(key)
                .to_string(),
            program: launch.program,
            args: launch.args,
            cwd: Some(dir),
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

    /// A free OpenCode model in a pane, working from a prompt file. It reports
    /// nothing through hooks (the engine polls its output file) and the
    /// project's guards do not run in it: that is why it only gets work that
    /// is verified afterwards, in a worktree or read-only.
    fn spawn_free(
        &self,
        key: &str,
        role: Role,
        prompt_file: &Path,
        _settings: &Path,
        model: &str,
        cwd: Option<&Path>,
    ) -> anyhow::Result<String> {
        let (st, p) = (&self.0, &self.2);
        if !harnes_free::valid_model(model) {
            anyhow::bail!("identifiant de modèle invalide");
        }
        let bin = harnes_free::binary().ok_or_else(|| anyhow::anyhow!("opencode introuvable"))?;
        harnes_free::equip(&p.root, &crate::state::ctx_bin())?;
        let file = prompt_file.display().to_string().replace('\\', "/");
        // Only fixed words and a plain path reach the command line (cmd.exe
        // runs npm shims, and it cannot be trusted with anything else).
        if !file.chars().all(|c| {
            c.is_ascii_alphanumeric() || matches!(c, ' ' | '.' | '_' | '-' | ':' | '/' | '(' | ')')
        }) {
            anyhow::bail!("chemin du prompt non sûr pour une ligne de commande");
        }
        let message = format!(
            "Lis le fichier `{file}` et fais exactement ce qu'il demande : c'est ta mission."
        );
        let shim = bin
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("cmd") || e.eq_ignore_ascii_case("bat"));
        let bin_text = bin.display().to_string();
        if shim && bin_text.contains(' ') {
            anyhow::bail!(
                "opencode est dans un chemin avec espace : lancement via cmd.exe non sûr"
            );
        }
        let (program, mut args) = if shim {
            ("cmd".to_string(), vec!["/C".to_string(), bin_text])
        } else {
            (bin_text, vec![])
        };
        args.extend(["--model".into(), model.into(), "--prompt".into(), message]);
        let dir = cwd
            .map(|c| c.to_path_buf())
            .unwrap_or_else(|| p.root.clone());
        let info = st.panes.spawn(SpawnSpec {
            title: key
                .split_once('-')
                .map(|(_, n)| n)
                .unwrap_or(key)
                .to_string(),
            program,
            args,
            cwd: Some(dir),
            env: agent_env(p, key),
            cols: 120,
            rows: 32,
        })?;
        st.tag_pane(&info.id, &p.id);
        st.agents.write().insert(
            key.into(),
            AgentView {
                key: key.into(),
                project: p.id.clone(),
                pane: Some(info.id.clone()),
                pid: info.pid,
                role: role.agent().into(),
                provider: "opencode".into(),
                state: AgentState::Working,
                context: None,
            },
        );
        let _ = st.app.emit(
            "pane-open",
            serde_json::json!({ "info": &info, "key": key, "role": role.agent(), "provider": "opencode", "model": model, "project": p.id }),
        );
        Ok(info.id)
    }

    fn has_hooks(&self, role: Role) -> bool {
        self.1
            .launch(role, "", Path::new(""))
            .map(|l| l.hooks)
            .unwrap_or(true)
    }

    fn close(&self, pane: &str) {
        let st = &self.0;
        // Keep the pane a moment so the user sees the final message.
        let (shared, pane) = (st.clone(), pane.to_string());
        st.agents
            .write()
            .retain(|_, a| a.pane.as_deref() != Some(pane.as_str()));
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(4)).await;
            let _ = shared.panes.close(&pane);
            shared.pane_project.write().remove(&pane);
            let _ = shared.app.emit("pane-closed", &pane);
        });
    }
}
