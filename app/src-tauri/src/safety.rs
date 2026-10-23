//! Cockpit commands for what a project is allowed to do: which models may see
//! its code (the profile), and what its own config could run (trust).

use crate::state::Shared;
use harnes_free::{Config, Profile};
use serde_json::{json, Value};
use tauri::State;

type Res<T> = Result<T, String>;

fn e(err: impl std::fmt::Display) -> String {
    err.to_string()
}

/// The profile, what OpenCode offers right now, and which of those models the
/// project allows. `usable` is empty when OpenCode is missing.
#[tauri::command]
pub fn profile_get(st: State<'_, Shared>, project: Option<String>) -> Res<Value> {
    let p = st.pick(project.as_deref())?;
    let cfg = Config::load(&p.root);
    let eco = crate::budget::pace(st.inner()).is_some_and(|x| x.eco());
    let live = harnes_free::live_models();
    Ok(json!({
        "config": cfg,
        "effective": cfg.effective(eco),
        "opencode": harnes_free::binary().is_some(),
        "live": live,
        "usable": harnes_free::usable(&cfg.models),
    }))
}

#[tauri::command]
pub fn profile_set(
    st: State<'_, Shared>,
    project: Option<String>,
    profile: Profile,
    auto_escalate: Option<bool>,
    models: Option<Vec<String>>,
    digest: Option<bool>,
) -> Res<Value> {
    let p = st.pick(project.as_deref())?;
    let mut cfg = Config::load(&p.root);
    cfg.profile = profile;
    if let Some(v) = auto_escalate {
        cfg.auto_escalate = v;
    }
    if let Some(v) = digest {
        cfg.digest = v;
    }
    if let Some(m) = models {
        // Only well-formed OpenCode ids: they end up on a command line.
        if let Some(bad) = m
            .iter()
            .find(|x| !harnes_free::valid_model(x) || !x.starts_with("opencode/"))
        {
            return Err(format!("identifiant de modèle invalide : {bad}"));
        }
        cfg.models = m;
    }
    cfg.save(&p.root).map_err(e)?;
    if profile != Profile::Claude {
        // Free agents get the same MCP tools: declare them to OpenCode.
        harnes_free::equip(&p.root, &crate::state::ctx_bin()).map_err(e)?;
    }
    let _ = p.telemetry.append(
        harnes_telemetry::Event::new("", &p.id, "profile.changed")
            .role("user")
            .summary(format!("profil {:?}", cfg.profile)),
    );
    profile_get(st, Some(p.id.clone()))
}

/// What the project's own config can do, and what is still waiting for a yes.
#[tauri::command]
pub fn trust_report(st: State<'_, Shared>, project: Option<String>) -> Res<Value> {
    let p = st.pick(project.as_deref())?;
    let blocking: Vec<String> = harnes_guards::trust::blocking(&p.root)
        .into_iter()
        .map(|f| f.fingerprint)
        .collect();
    let findings: Vec<Value> = harnes_guards::trust::scan(&p.root)
        .into_iter()
        .map(|f| {
            let blocks = blocking.contains(&f.fingerprint);
            json!({"id": f.id, "severity": f.severity, "file": f.file, "message": f.message, "fingerprint": f.fingerprint, "blocks": blocks})
        })
        .collect();
    Ok(json!({"findings": findings, "blocked": !blocking.is_empty()}))
}

/// Accepts what is blocking now. The yes is tied to the file's content: a
/// later change asks again.
#[tauri::command]
pub fn trust_accept(st: State<'_, Shared>, project: Option<String>) -> Res<Value> {
    let p = st.pick(project.as_deref())?;
    let now: Vec<String> = harnes_guards::trust::blocking(&p.root)
        .into_iter()
        .map(|f| f.fingerprint)
        .collect();
    harnes_guards::trust::accept(&p.root, &now).map_err(e)?;
    let _ = p.telemetry.append(
        harnes_telemetry::Event::new("", &p.id, "trust.accepted")
            .role("user")
            .summary(format!("{} constat(s) acceptés", now.len())),
    );
    trust_report(st, Some(p.id.clone()))
}
