//! Run views, trace, review-gated memory, eval and harness commands.

use crate::state::{Project, Shared};
use harnes_crew::view::{self, Decision};
use serde_json::{json, Value};
use tauri::State;

fn e(err: impl std::fmt::Display) -> String {
    format!("{err:#}")
}

fn run_id(run: &str) -> Result<&str, String> {
    if !run.is_empty()
        && run
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        Ok(run)
    } else {
        Err(format!("identifiant de run invalide : {run}"))
    }
}

fn latest_run(p: &Project) -> Option<String> {
    std::fs::read_dir(p.root.join(".harnes").join("runs"))
        .ok()?
        .flatten()
        .filter_map(|d| d.file_name().to_string_lossy().parse::<u64>().ok())
        .max()
        .map(|n| n.to_string())
}

/// `harnes.view.v1`: the whole run as typed cards, never generated HTML.
#[tauri::command]
pub fn run_view(st: State<'_, Shared>, run: Option<String>) -> Result<Value, String> {
    let p = st.active_or_err()?;
    let run = run
        .or_else(|| latest_run(&p))
        .ok_or("aucun run dans ce projet")?;
    let run = run_id(&run)?;
    let v = view::build(&p.root, run, Some(&p.telemetry), Some(&p.memory));
    serde_json::to_value(v).map_err(e)
}

#[tauri::command]
pub fn trace_events(
    st: State<'_, Shared>,
    run: Option<String>,
    kind: Option<String>,
    role: Option<String>,
    path: Option<String>,
    limit: Option<usize>,
) -> Result<Value, String> {
    let p = st.active_or_err()?;
    let events = p
        .telemetry
        .query(&harnes_telemetry::Filter {
            run,
            kind,
            role,
            path,
            since: None,
            limit: limit.unwrap_or(300),
        })
        .map_err(e)?;
    serde_json::to_value(events).map_err(e)
}

#[tauri::command]
pub fn trace_runs(st: State<'_, Shared>) -> Result<Value, String> {
    let p = st.active_or_err()?;
    serde_json::to_value(p.telemetry.runs().map_err(e)?).map_err(e)
}

#[tauri::command]
pub fn trace_summary(st: State<'_, Shared>, run: String) -> Result<Value, String> {
    let p = st.active_or_err()?;
    serde_json::to_value(p.telemetry.summary(&run).map_err(e)?).map_err(e)
}

/// Records a human decision (gate, review, memory) next to the run.
#[tauri::command]
pub fn run_decide(
    st: State<'_, Shared>,
    run: String,
    kind: String,
    id: String,
    action: String,
    reason: String,
) -> Result<Value, String> {
    let p = st.active_or_err()?;
    let run = run_id(&run)?.to_string();
    let decision = Decision {
        at: harnes_telemetry::now_ms(),
        kind: kind.clone(),
        id: id.clone(),
        action: action.clone(),
        reason: reason.clone(),
        by: "user".into(),
    };
    // Promote memory only after the review succeeded. Otherwise an invalid or
    // already-reviewed candidate would leave an `approved` decision in the
    // audit log while no record exists.
    if kind == "memory" {
        if action != "approve" && action != "reject" {
            return Err("décision mémoire attendue : approve | reject".into());
        }
        let mid = id.parse::<i64>().map_err(e)?;
        let c = p.memory.review(mid, &action, &reason, None).map_err(e)?;
        view::record_decision(&p.root, &run, &decision).map_err(e)?;
        let _ = p.telemetry.append(
            harnes_telemetry::Event::new(&run, &p.id, format!("{kind}.decided"))
                .role("user")
                .status(action.clone())
                .summary(reason.clone())
                .detail(json!({"id": id}))
                .evidence(vec![id.clone()]),
        );
        return serde_json::to_value(c).map_err(e);
    }
    view::record_decision(&p.root, &run, &decision).map_err(e)?;
    let ev = harnes_telemetry::Event::new(&run, &p.id, format!("{kind}.decided"))
        .role("user")
        .status(action.clone())
        .summary(reason.clone())
        .detail(json!({"id": id}))
        .evidence(vec![id.clone()]);
    let _ = p.telemetry.append(ev);
    Ok(json!({"ok": true}))
}

#[tauri::command]
pub fn memory_candidates(
    st: State<'_, Shared>,
    status: Option<String>,
    run: Option<String>,
    limit: Option<usize>,
) -> Result<Value, String> {
    let p = st.active_or_err()?;
    let list = p
        .memory
        .candidates(
            status.as_deref().unwrap_or("candidate"),
            run.as_deref(),
            limit.unwrap_or(50),
        )
        .map_err(e)?;
    serde_json::to_value(list).map_err(e)
}

#[tauri::command]
pub fn memory_records(st: State<'_, Shared>, limit: Option<usize>) -> Result<Value, String> {
    let p = st.active_or_err()?;
    serde_json::to_value(p.memory.records(limit.unwrap_or(100)).map_err(e)?).map_err(e)
}

#[tauri::command]
pub fn memory_stats(st: State<'_, Shared>) -> Result<Value, String> {
    let p = st.active_or_err()?;
    serde_json::to_value(p.memory.stats().map_err(e)?).map_err(e)
}

#[tauri::command]
pub fn memory_review(
    st: State<'_, Shared>,
    id: i64,
    decision: String,
    reason: Option<String>,
    title: Option<String>,
    body: Option<String>,
) -> Result<Value, String> {
    let p = st.active_or_err()?;
    let edit = match (title, body) {
        (Some(t), Some(b)) => Some((t, b)),
        _ => None,
    };
    let c = p
        .memory
        .review(id, &decision, reason.as_deref().unwrap_or(""), edit)
        .map_err(e)?;
    let _ = view::record_decision(
        &p.root,
        run_id(&c.run).unwrap_or("manual"),
        &Decision {
            at: harnes_telemetry::now_ms(),
            kind: "memory".into(),
            id: c.id.to_string(),
            action: decision.clone(),
            reason: reason.unwrap_or_default(),
            by: "user".into(),
        },
    );
    let _ = p.telemetry.append(
        harnes_telemetry::Event::new(&c.run, &p.id, "memory.reviewed")
            .role("user")
            .status(decision.clone())
            .summary(format!("#{} {}", c.id, c.title))
            .evidence(c.evidence.clone()),
    );
    serde_json::to_value(c).map_err(e)
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn memory_propose(
    st: State<'_, Shared>,
    title: String,
    body: String,
    kind: Option<String>,
    applicability: Option<String>,
    tags: Option<Vec<String>>,
    evidence: Option<Vec<String>>,
    confidence: Option<f64>,
) -> Result<Value, String> {
    let p = st.active_or_err()?;
    let run = p.run_slot.lock().clone().unwrap_or_else(|| "manual".into());
    let c = p
        .memory
        .create_candidate(
            &run,
            kind.as_deref().unwrap_or("gotcha"),
            &title,
            &body,
            applicability.as_deref().unwrap_or(""),
            &tags.unwrap_or_default(),
            &evidence.unwrap_or_default(),
            "user",
            confidence.unwrap_or(0.7),
        )
        .map_err(e)?;
    serde_json::to_value(c).map_err(e)
}

#[tauri::command]
pub fn harness_get(st: State<'_, Shared>) -> Result<Value, String> {
    let p = st.active_or_err()?;
    let spec = harnes_crew::harness::HarnessSpec::ensure(&p.root, "claude").map_err(e)?;
    Ok(
        json!({"spec": spec, "components": spec.flat(), "path": harnes_crew::harness::path(&p.root).display().to_string()}),
    )
}

#[tauri::command]
pub fn harness_save(st: State<'_, Shared>, spec: Value) -> Result<Value, String> {
    let p = st.active_or_err()?;
    let spec: harnes_crew::harness::HarnessSpec = serde_json::from_value(spec).map_err(e)?;
    spec.save(&p.root).map_err(e)?;
    let _ = p.telemetry.append(
        harnes_telemetry::Event::new("", &p.id, "harness.updated")
            .role("user")
            .summary("HarnessSpec enregistré"),
    );
    Ok(json!({"ok": true, "spec": spec}))
}

#[tauri::command]
pub fn guards_get(st: State<'_, Shared>) -> Result<Value, String> {
    let p = st.active_or_err()?;
    let text = std::fs::read_to_string(p.root.join(".harnes").join("guards.json")).map_err(e)?;
    serde_json::from_str(&text).map_err(e)
}

#[tauri::command]
pub fn guards_save(st: State<'_, Shared>, guards: Value) -> Result<Value, String> {
    let p = st.active_or_err()?;
    let parsed = harnes_guards::Guard::from_value(guards.clone()).map_err(e)?;
    std::fs::write(
        p.root.join(".harnes").join("guards.json"),
        serde_json::to_string_pretty(&guards).map_err(e)? + "\n",
    )
    .map_err(e)?;
    *p.guards.write() = parsed;
    let _ = p.telemetry.append(
        harnes_telemetry::Event::new("", &p.id, "guards.updated")
            .role("user")
            .summary("règles de garde enregistrées"),
    );
    Ok(json!({"ok": true}))
}

#[tauri::command]
pub fn guards_reload(st: State<'_, Shared>) -> Result<Value, String> {
    let p = st.active_or_err()?;
    let guard = harnes_guards::Guard::load(&p.root).map_err(e)?;
    *p.guards.write() = guard;
    Ok(json!({"ok": true}))
}

#[tauri::command]
pub fn eval_list(st: State<'_, Shared>) -> Result<Value, String> {
    let p = st.active_or_err()?;
    let _ = harnes_eval::ensure_defaults(&p.root);
    serde_json::to_value(harnes_eval::list(&p.root).map_err(e)?).map_err(e)
}

#[tauri::command]
pub async fn eval_run(st: State<'_, Shared>, suite: Option<String>) -> Result<Value, String> {
    let p = st.active_or_err()?;
    let root = p.root.clone();
    let name = suite;
    tauri::async_runtime::spawn_blocking(move || -> Result<Value, String> {
        let _ = harnes_eval::ensure_defaults(&root);
        let suite = match name {
            Some(n) => harnes_eval::load(&root, &n).map_err(e)?,
            None => harnes_eval::list(&root)
                .map_err(e)?
                .into_iter()
                .next()
                .ok_or("aucune suite d'évaluation")?,
        };
        harnes_eval::validate(&suite).map_err(e)?;
        let result = harnes_eval::run_suite(&root, &suite);
        let path = harnes_eval::save_result(&root, &result).map_err(e)?;
        let ledger = harnes_eval::ledger(&root, &result).map_err(e)?;
        Ok(json!({"result": result, "path": path.display().to_string(), "ledger": ledger}))
    })
    .await
    .map_err(e)?
}

#[tauri::command]
pub fn tune_plan(
    st: State<'_, Shared>,
    component: Option<String>,
    suite: Option<String>,
    model: Option<String>,
) -> Result<Value, String> {
    let p = st.active_or_err()?;
    let plan = harnes_eval::tune_plan(
        component.as_deref().unwrap_or("worker"),
        suite.as_deref().unwrap_or("smoke"),
        model.as_deref().unwrap_or("sonnet"),
        crate::budget::pace(st.inner()).is_some_and(|p| p.eco()),
    );
    let dir = p.root.join(".harnes").join("experiments");
    std::fs::create_dir_all(&dir).map_err(e)?;
    let path = dir.join(format!("tune-{}.json", plan.created_at));
    std::fs::write(&path, serde_json::to_string_pretty(&plan).map_err(e)?).map_err(e)?;
    let _ = p.telemetry.append(
        harnes_telemetry::Event::new("", &p.id, "tune.planned")
            .role("user")
            .summary(format!(
                "{} variantes pour {}",
                plan.variants.len(),
                plan.component
            )),
    );
    Ok(json!({"plan": plan, "path": path.display().to_string()}))
}

#[tauri::command]
pub fn worktree_list(st: State<'_, Shared>) -> Result<Value, String> {
    let p = st.active_or_err()?;
    serde_json::to_value(p.worktrees.list().map_err(e)?).map_err(e)
}

#[tauri::command]
pub fn worktree_remove(
    st: State<'_, Shared>,
    run: String,
    lot: String,
    force: Option<bool>,
) -> Result<Value, String> {
    let p = st.active_or_err()?;
    if p.crew.running()
        && p.crew
            .task
            .lock()
            .as_ref()
            .is_some_and(|(current, _)| current == &run)
    {
        return Err("run en cours : termine ou annule le run avant de retirer son worktree".into());
    }
    p.worktrees
        .remove(&run, &lot, force.unwrap_or(false))
        .map_err(e)?;
    Ok(json!({"ok": true}))
}

#[tauri::command]
pub fn handoffs(st: State<'_, Shared>) -> Result<Value, String> {
    let p = st.active_or_err()?;
    let dir = p.root.join(".harnes").join("context");
    let mut files = Vec::new();
    for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
        if let Ok(meta) = e.metadata() {
            files.push(json!({
                "name": e.file_name().to_string_lossy(),
                "path": e.path().display().to_string(),
                "modified": meta.modified().ok().and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0),
            }));
        }
    }
    Ok(Value::Array(files))
}
