//! MCP tools over review-gated project memory.
//!
//! Agents may propose candidates and read approved records. Approving is a
//! human action (UI, or an explicit `HARNES_MEMORY_REVIEW=1` policy); the MCP
//! surface refuses it by default so an agent cannot promote its own lesson.

use crate::tools::{str_arg, usize_arg};
use anyhow::{anyhow, Result};
use harnes_memory::Memory;
use serde_json::{json, Value};

pub fn definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "memory_search",
            "description": "Search approved project memory (past corrections, conventions, gotchas). Only human-approved records are returned.",
            "inputSchema": {"type": "object", "properties": {
                "query": {"type": "string"},
                "limit": {"type": "integer", "default": 8}
            }, "required": ["query"]}
        }),
        json!({
            "name": "memory_context",
            "description": "Compact approved-memory block to apply before planning or editing. Use it to avoid repeating a known mistake.",
            "inputSchema": {"type": "object", "properties": {
                "query": {"type": "string"},
                "limit": {"type": "integer", "default": 5}
            }, "required": ["query"]}
        }),
        json!({
            "name": "memory_list",
            "description": "List the most recent approved memory records with provenance.",
            "inputSchema": {"type": "object", "properties": {"limit": {"type": "integer", "default": 20}}}
        }),
        json!({
            "name": "memory_candidates",
            "description": "List memory candidates awaiting human review.",
            "inputSchema": {"type": "object", "properties": {
                "status": {"type": "string", "enum": ["candidate", "approved", "rejected", "all"], "default": "candidate"},
                "run": {"type": "string"},
                "limit": {"type": "integer", "default": 20}
            }}
        }),
        json!({
            "name": "memory_propose",
            "description": "Propose a lesson for human review. Keep it short, actionable and backed by evidence (file, command, event). It is not visible to future agents until approved.",
            "inputSchema": {"type": "object", "properties": {
                "title": {"type": "string"},
                "body": {"type": "string"},
                "applicability": {"type": "string"},
                "kind": {"type": "string", "enum": ["workflow", "correction", "debugging_pattern", "gotcha", "convention"], "default": "gotcha"},
                "tags": {"type": "array", "items": {"type": "string"}},
                "evidence": {"type": "array", "items": {"type": "string"}},
                "confidence": {"type": "number", "default": 0.5}
            }, "required": ["title", "body"]}
        }),
        json!({
            "name": "memory_review",
            "description": "Approve or reject a candidate. Disabled for agents by default; a human reviews candidates in the harnes cockpit.",
            "inputSchema": {"type": "object", "properties": {
                "id": {"type": "integer"},
                "decision": {"type": "string", "enum": ["approve", "reject"]},
                "reason": {"type": "string"},
                "title": {"type": "string"},
                "body": {"type": "string"}
            }, "required": ["id", "decision"]}
        }),
    ]
}

fn tags(args: &Value) -> Vec<String> {
    args.get("tags")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect()
        })
        .unwrap_or_default()
}

fn strings(args: &Value, key: &str) -> Vec<String> {
    args.get(key)
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(String::from)
                .collect::<Vec<String>>()
        })
        .unwrap_or_default()
}

pub fn call(memory: &Memory, name: &str, args: &Value) -> Result<Option<String>> {
    let out = match name {
        "memory_search" => {
            let hits = memory.search(str_arg(args, "query")?, usize_arg(args, "limit", 8))?;
            if hits.is_empty() {
                "no approved memory matches".to_string()
            } else {
                hits.iter()
                    .map(|r| {
                        format!(
                            "#{} [{}] {}\n{}\n  quand : {}\n  provenance : {}",
                            r.id, r.kind, r.title, r.body, r.applicability, r.provenance
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n\n")
            }
        }
        "memory_context" => {
            let s = memory.context(str_arg(args, "query")?, usize_arg(args, "limit", 5))?;
            if s.trim().is_empty() {
                "no approved memory matches".into()
            } else {
                s
            }
        }
        "memory_list" => {
            let records = memory.records(usize_arg(args, "limit", 20))?;
            if records.is_empty() {
                "memory is empty (candidates need human approval)".into()
            } else {
                serde_json::to_string_pretty(&records)?
            }
        }
        "memory_candidates" => {
            let status = args
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("candidate");
            let run = args.get("run").and_then(Value::as_str);
            serde_json::to_string_pretty(&memory.candidates(
                status,
                run,
                usize_arg(args, "limit", 20),
            )?)?
        }
        "memory_propose" => {
            if std::env::var("HARNES_MEMORY_WRITE").as_deref() == Ok("0") {
                return Ok(Some("memory proposals are disabled for this agent".into()));
            }
            let run = std::env::var("HARNES_RUN").unwrap_or_else(|_| "manual".into());
            let c = memory.create_candidate(
                &run,
                args.get("kind").and_then(Value::as_str).unwrap_or("gotcha"),
                str_arg(args, "title")?,
                str_arg(args, "body")?,
                args.get("applicability")
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                &tags(args),
                &strings(args, "evidence"),
                "agent",
                args.get("confidence")
                    .and_then(Value::as_f64)
                    .unwrap_or(0.5),
            )?;
            format!(
                "candidate #{} en attente de revue humaine : {}",
                c.id, c.title
            )
        }
        "memory_review" => {
            if std::env::var("HARNES_MEMORY_REVIEW").as_deref() != Ok("1") {
                return Err(anyhow!("approbation humaine requise : utilise le cockpit harnes (ou HARNES_MEMORY_REVIEW=1 pour une politique explicite)"));
            }
            let id = args
                .get("id")
                .and_then(Value::as_i64)
                .ok_or_else(|| anyhow!("missing argument: id"))?;
            let decision = str_arg(args, "decision")?;
            let reason = args.get("reason").and_then(Value::as_str).unwrap_or("");
            let edit = match (
                args.get("title").and_then(Value::as_str),
                args.get("body").and_then(Value::as_str),
            ) {
                (Some(t), Some(b)) => Some((t.to_string(), b.to_string())),
                _ => None,
            };
            serde_json::to_string_pretty(&memory.review(id, decision, reason, edit)?)?
        }
        _ => return Ok(None),
    };
    Ok(Some(out))
}
