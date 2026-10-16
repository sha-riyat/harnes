//! MCP tools for agent-to-agent threads and, for the orchestrator, crew control.
//!
//! Author identity comes from `HARNES_AGENT` (set by harnes for each pane);
//! sub-agents sign with `as` (e.g. "scout"), shown as `parent/scout`.
//! Crew tools call the harnes app through `HARNES_API` (local HTTP).

use anyhow::{anyhow, bail, Result};
use harnes_threads::Threads;
use serde_json::{json, Value};
use std::io::{Read, Write};

pub fn definitions(orchestrator: bool) -> Vec<Value> {
    let mut v = vec![
        json!({"name": "thread_list", "description": "List threads (agent conversations) of this project, most active first.",
            "inputSchema": {"type": "object", "properties": {"status": {"type": "string", "enum": ["open", "closed", "all"], "default": "open"}, "run": {"type": "string"}}}}),
        json!({"name": "thread_read", "description": "Read a thread's messages (all, or after message id `since`). Read the thread of your lot before starting and before finishing.",
            "inputSchema": {"type": "object", "properties": {"thread": {"type": "string"}, "since": {"type": "integer", "default": 0}}, "required": ["thread"]}}),
        json!({"name": "thread_post", "description": "Post a message to a thread: questions to another agent, decisions, hand-offs, results. Sub-agents set `as` (e.g. \"scout\").",
            "inputSchema": {"type": "object", "properties": {"thread": {"type": "string"}, "body": {"type": "string"}, "as": {"type": "string"}}, "required": ["thread", "body"]}}),
        json!({"name": "thread_create", "description": "Open a new thread (e.g. a question to the planner, a design discussion).",
            "inputSchema": {"type": "object", "properties": {"title": {"type": "string"}, "kind": {"type": "string", "enum": ["question", "chat", "review", "lot", "run"], "default": "question"}, "run": {"type": "string"}, "body": {"type": "string"}}, "required": ["title"]}}),
        json!({"name": "thread_close", "description": "Close a thread with a one-line summary. History stays readable; posting again reopens it.",
            "inputSchema": {"type": "object", "properties": {"thread": {"type": "string"}, "summary": {"type": "string"}}, "required": ["thread"]}}),
    ];
    if orchestrator {
        v.push(json!({"name": "crew_start", "description": "Start a crew run (Planner -> Teacher -> Workers -> Reviewer -> Teacher gate) for a task.",
            "inputSchema": {"type": "object", "properties": {"task": {"type": "string"}}, "required": ["task"]}}));
        v.push(json!({"name": "crew_status", "description": "Current crew run: phase, agents and their state, last events. With `wait_secs`, waits for the next change first.",
            "inputSchema": {"type": "object", "properties": {"wait_secs": {"type": "integer", "default": 0}}}}));
        v.push(json!({"name": "crew_cancel", "description": "Stop the current crew run.", "inputSchema": {"type": "object", "properties": {}}}));
    }
    v
}

pub fn author(args: &Value) -> String {
    let me = std::env::var("HARNES_AGENT").unwrap_or_else(|_| "agent".into());
    match args.get("as").and_then(Value::as_str).filter(|s| !s.trim().is_empty()) {
        Some(sub) => format!("{me}/{sub}"),
        None => me,
    }
}

fn s<'a>(a: &'a Value, k: &str) -> Result<&'a str> {
    a.get(k).and_then(Value::as_str).ok_or_else(|| anyhow!("argument manquant : {k}"))
}

fn fmt_msgs(msgs: &[harnes_threads::Message]) -> String {
    if msgs.is_empty() {
        return "aucun nouveau message".into();
    }
    msgs.iter().map(|m| format!("#{} [{}] {} : {}", m.id, m.kind, m.author, m.body)).collect::<Vec<_>>().join("\n")
}

pub fn call(t: &Threads, name: &str, a: &Value) -> Result<Option<String>> {
    Ok(Some(match name {
        "thread_list" => {
            let list = t.list(a.get("status").and_then(Value::as_str).unwrap_or("open"), a.get("run").and_then(Value::as_str), 50)?;
            if list.is_empty() {
                "aucun thread".into()
            } else {
                list.iter()
                    .map(|th| format!("{} [{}|{}] {} — {} msg, dernier : {}", th.id, th.kind, th.status, th.title, th.messages, th.last_author.as_deref().unwrap_or("-")))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
        "thread_read" => {
            let id = s(a, "thread")?;
            let th = t.get(id)?.ok_or_else(|| anyhow!("thread inconnu : {id}"))?;
            let msgs = t.read(id, a.get("since").and_then(Value::as_i64).unwrap_or(0), 200)?;
            format!("{} — {} ({})\n{}", th.id, th.title, th.status, fmt_msgs(&msgs))
        }
        "thread_post" => {
            let m = t.post(s(a, "thread")?, &author(a), "message", s(a, "body")?)?;
            format!("posté (#{})", m.id)
        }
        "thread_create" => {
            let me = author(a);
            let th = t.create(s(a, "title")?, a.get("kind").and_then(Value::as_str).unwrap_or("question"), a.get("run").and_then(Value::as_str), &me)?;
            if let Some(body) = a.get("body").and_then(Value::as_str) {
                t.post(&th.id, &me, "message", body)?;
            }
            format!("thread créé : {}", th.id)
        }
        "thread_close" => {
            t.close(s(a, "thread")?, &author(a), a.get("summary").and_then(Value::as_str))?;
            "fil clos".into()
        }
        "crew_start" => api("start", &json!({"task": s(a, "task")?}))?,
        "crew_status" => api("status", &json!({"wait_secs": a.get("wait_secs").and_then(Value::as_u64).unwrap_or(0).min(600)}))?,
        "crew_cancel" => api("cancel", &json!({}))?,
        _ => return Ok(None),
    }))
}

/// Minimal blocking HTTP POST to the harnes app (`HARNES_API=http://127.0.0.1:PORT`).
fn api(cmd: &str, body: &Value) -> Result<String> {
    let base = std::env::var("HARNES_API").map_err(|_| anyhow!("HARNES_API absent : outil réservé à l'orchestrateur lancé par harnes"))?;
    let host = base.trim_start_matches("http://").trim_end_matches('/');
    // Tell harnes which project this orchestrator belongs to.
    let mut body = body.clone();
    if let Ok(project) = std::env::var("HARNES_PROJECT") {
        body["project"] = json!(project);
    }
    let mut stream = std::net::TcpStream::connect(host)?;
    stream.set_read_timeout(Some(std::time::Duration::from_secs(660)))?;
    let b = body.to_string();
    write!(stream, "POST /api/{cmd} HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{b}", b.len())?;
    let mut raw = String::new();
    stream.read_to_string(&mut raw)?;
    let (head, payload) = raw.split_once("\r\n\r\n").unwrap_or((&raw, ""));
    if !head.starts_with("HTTP/1.1 200") {
        bail!("harnes a répondu : {}", head.lines().next().unwrap_or("?"));
    }
    Ok(payload.to_string())
}
