//! Moves a session to another provider without losing its context.

use crate::{
    crew_ctl,
    host::agent_env,
    state::{AgentView, Project, Shared},
};
use harnes_crew::{context, providers::Providers, AgentState};
use harnes_pty::SpawnSpec;
use serde_json::json;
use std::sync::Arc;
use tauri::{Emitter, State};

/// Recent thread messages of the project, as context for the next provider.
fn threads_digest(p: &Project) -> String {
    let Some(t) = &p.threads else {
        return String::new();
    };
    t.recent(0, 30)
        .unwrap_or_default()
        .iter()
        .map(|m| {
            format!(
                "- [{}] {} : {}",
                m.thread,
                m.author,
                m.body.lines().next().unwrap_or("")
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Transcript of the current session, whatever its provider.
fn transcript(st: &Shared, p: &Project, a: &AgentView) -> String {
    if a.provider == "claude" {
        if let Some(path) = st
            .hooks
            .registry
            .session_of(&a.key)
            .and_then(|s| context::claude_transcript(&s))
        {
            if let Ok(md) = context::export_claude(&path) {
                return md;
            }
        }
    }
    // Other providers: chain the previous hand-off so nothing is lost.
    a.context
        .as_ref()
        .and_then(|rel| std::fs::read_to_string(p.root.join(rel)).ok())
        .unwrap_or_default()
}

/// Starts `key` on `provider` with a first prompt. Returns the pane id.
pub fn spawn_on(
    st: &Shared,
    p: &Arc<Project>,
    key: &str,
    title: &str,
    provider: &str,
    prompt: String,
    handoff: Option<String>,
) -> Result<String, String> {
    let agent = if key == p.orch_key() {
        Some("orchestrator")
    } else {
        None
    };
    if provider == "claude" {
        let id = crew_ctl::spawn_claude(st, p, key, title, agent, None, Some(prompt))?;
        if let Some(a) = st.agents.write().get_mut(key) {
            a.context = handoff;
        }
        return Ok(id);
    }
    let providers = Providers::load(&p.root).map_err(|e| format!("{e:#}"))?;
    let full = match agent {
        Some(a) => format!("Lis et applique les instructions de `.claude/agents/{a}.md` (ignore l'en-tête YAML) : c'est ton rôle. {prompt}"),
        None => prompt,
    };
    let launch = providers
        .launch_session(provider, &full, std::path::Path::new(""))
        .map_err(|e| format!("{e:#}"))?;
    let mut env = agent_env(p, key);
    if agent.is_some() {
        env.push(("HARNES_ROLE".into(), "orchestrator".into()));
        env.push(("HARNES_API".into(), st.hooks.api_url()));
    }
    let info = st
        .panes
        .spawn(SpawnSpec {
            title: title.into(),
            program: launch.program,
            args: launch.args,
            cwd: Some(p.root.clone()),
            env,
            cols: 120,
            rows: 32,
        })
        .map_err(|e| e.to_string())?;
    st.tag_pane(&info.id, &p.id);
    let role = agent.unwrap_or(provider).to_string();
    st.agents.write().insert(
        key.into(),
        AgentView {
            key: key.into(),
            project: p.id.clone(),
            pane: Some(info.id.clone()),
            pid: info.pid,
            role: role.clone(),
            provider: provider.into(),
            state: AgentState::Working,
            context: handoff,
        },
    );
    let _ = st.app.emit(
        "pane-open",
        json!({ "info": &info, "key": key, "role": role, "provider": provider, "project": p.id }),
    );
    Ok(info.id)
}

#[tauri::command]
pub fn providers_list(st: State<'_, Shared>) -> Vec<String> {
    st.active()
        .and_then(|p| Providers::load(&p.root).ok())
        .map(|p| p.names())
        .unwrap_or_else(|| vec!["claude".into()])
}

/// Hands the session in pane `id` over to `provider`, keeping its context.
#[tauri::command]
pub fn pane_switch_provider(
    st: State<'_, Shared>,
    id: String,
    provider: String,
) -> Result<String, String> {
    let st = st.inner();
    let p = st.project_of_pane(&id).ok_or("pane sans projet")?;
    let agent = st
        .agents
        .read()
        .values()
        .find(|a| a.pane.as_deref() == Some(id.as_str()))
        .cloned()
        .ok_or(
            "seules les sessions d'agent (orchestrateur, Claude…) peuvent changer de fournisseur",
        )?;
    if agent.key != p.orch_key() && !agent.key.starts_with("claude-") {
        return Err("les agents d'un run sont gérés par le moteur : change leur fournisseur dans .harnes/providers.json".into());
    }
    if agent.provider == provider {
        return Ok(id);
    }
    let md = transcript(st, &p, &agent);
    let handoff = context::write_handoff(
        &p.root,
        &agent.key,
        &agent.provider,
        &provider,
        &md,
        &threads_digest(&p),
    )
    .map_err(|e| format!("{e:#}"))?;
    let title = st
        .panes
        .get(&id)
        .map(|x| x.info().title)
        .unwrap_or_else(|_| agent.key.clone());
    let _ = st.panes.close(&id);
    st.pane_project.write().remove(&id);
    let _ = st.app.emit("pane-closed", &id);
    spawn_on(
        st,
        &p,
        &agent.key,
        &title,
        &provider,
        context::resume_prompt(&handoff),
        Some(handoff),
    )
}
