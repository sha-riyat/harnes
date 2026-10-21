//! MCP tool definitions and dispatch.

use crate::{exec, store::Store, summary::digest};
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::{path::PathBuf, time::Duration};

pub struct Ctx {
    pub store: Store,
    pub root: PathBuf,
    pub threads: Option<harnes_threads::Threads>,
    pub os: harnes_os::Library,
    pub memory: Option<harnes_memory::Memory>,
}

/// All tools offered by this server: context sandbox + threads (+ crew control for the orchestrator).
pub fn all_definitions(ctx: &Ctx) -> Value {
    let mut v = definitions().as_array().cloned().unwrap_or_default();
    if ctx.threads.is_some() {
        let orchestrator = std::env::var("HARNES_ROLE")
            .map(|r| r == "orchestrator")
            .unwrap_or(false);
        v.extend(crate::threads::definitions(orchestrator));
    }
    v.extend(crate::os::definitions());
    if ctx.memory.is_some() {
        v.extend(crate::memory::definitions());
    }
    Value::Array(v)
}

pub fn definitions() -> Value {
    json!([
        {
            "name": "ctx_execute",
            "description": "Run code (shell, powershell, cmd, python, node) in the project directory. Small output is returned as-is; large output (tests, builds, logs) is stored and indexed, and you get a digest with head, errors, tail and an id. Prefer this over Bash for anything that may print a lot.",
            "inputSchema": {"type": "object", "properties": {
                "language": {"type": "string", "enum": ["shell", "powershell", "cmd", "python", "node"]},
                "code": {"type": "string"},
                "intent": {"type": "string", "description": "What you are looking for in the output; matching lines are surfaced."},
                "timeout_ms": {"type": "integer", "default": 120000}
            }, "required": ["language", "code"]}
        },
        {
            "name": "ctx_execute_file",
            "description": "Read a file through the context sandbox. Large files are indexed and summarised instead of pasted.",
            "inputSchema": {"type": "object", "properties": {
                "path": {"type": "string"},
                "intent": {"type": "string"}
            }, "required": ["path"]}
        },
        {
            "name": "ctx_index",
            "description": "Store and index text (docs, API responses, notes) so it can be searched later without staying in context.",
            "inputSchema": {"type": "object", "properties": {
                "content": {"type": "string"},
                "source": {"type": "string", "description": "Label, e.g. a URL or file name."}
            }, "required": ["content", "source"]}
        },
        {
            "name": "ctx_search",
            "description": "Full-text search (BM25) across everything stored by ctx tools and by the harnes proxy.",
            "inputSchema": {"type": "object", "properties": {
                "query": {"type": "string"},
                "limit": {"type": "integer", "default": 10}
            }, "required": ["query"]}
        },
        {
            "name": "ctx_retrieve",
            "description": "Read lines of a stored item by id (from a digest or a '[harnes: … elided]' marker).",
            "inputSchema": {"type": "object", "properties": {
                "id": {"type": "string"},
                "offset": {"type": "integer", "default": 1},
                "limit": {"type": "integer", "default": 200}
            }, "required": ["id"]}
        },
        {
            "name": "ctx_stats",
            "description": "How much content the sandbox has kept out of context.",
            "inputSchema": {"type": "object", "properties": {}}
        }
    ])
}

pub(crate) fn str_arg<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("missing argument: {key}"))
}

pub(crate) fn usize_arg(args: &Value, key: &str, default: usize) -> usize {
    args.get(key)
        .and_then(Value::as_u64)
        .map(|v| v as usize)
        .unwrap_or(default)
}

pub fn call(ctx: &Ctx, name: &str, args: &Value) -> Result<String> {
    if let Some(t) = &ctx.threads {
        if let Some(out) = crate::threads::call(t, name, args)? {
            return Ok(out);
        }
    }
    if let Some(out) = crate::os::call(&ctx.os, name, args)? {
        return Ok(out);
    }
    if let Some(memory) = &ctx.memory {
        if let Some(out) = crate::memory::call(memory, name, args)? {
            return Ok(out);
        }
    }
    match name {
        "ctx_execute" => {
            let lang = str_arg(args, "language")?;
            let code = str_arg(args, "code")?;
            let timeout = Duration::from_millis(usize_arg(args, "timeout_ms", 120_000) as u64);
            let out = exec::run(lang, code, &ctx.root, timeout)?;
            let mut header = format!(
                "exit={} time={}ms",
                out.code.map_or("?".into(), |c| c.to_string()),
                out.millis
            );
            if out.timed_out {
                header.push_str(" TIMED OUT");
            }
            let source = format!(
                "{lang}: {}",
                code.lines()
                    .next()
                    .unwrap_or("")
                    .chars()
                    .take(80)
                    .collect::<String>()
            );
            let body = digest(
                &ctx.store,
                &source,
                &out.combined(),
                args.get("intent").and_then(Value::as_str),
            )?;
            Ok(format!("{header}\n{body}"))
        }
        "ctx_execute_file" => {
            let path = ctx.root.join(str_arg(args, "path")?);
            let content =
                std::fs::read_to_string(&path).map_err(|e| anyhow!("{}: {e}", path.display()))?;
            digest(
                &ctx.store,
                &path.display().to_string(),
                &content,
                args.get("intent").and_then(Value::as_str),
            )
        }
        "ctx_index" => {
            let id = ctx
                .store
                .put(str_arg(args, "source")?, str_arg(args, "content")?)?;
            Ok(format!("indexed as id={id}"))
        }
        "ctx_search" => {
            let hits = ctx
                .store
                .search(str_arg(args, "query")?, usize_arg(args, "limit", 10))?;
            if hits.is_empty() {
                return Ok("no matches".into());
            }
            Ok(hits
                .iter()
                .map(|h| {
                    format!(
                        "[{} line {}] {}\n  {}",
                        h.id,
                        h.line,
                        h.source,
                        h.snippet.replace('\n', " ")
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"))
        }
        "ctx_retrieve" => {
            let id = str_arg(args, "id")?;
            let offset = usize_arg(args, "offset", 1).max(1);
            let limit = usize_arg(args, "limit", 200).clamp(1, 2000);
            match ctx.store.get(id, offset, limit)? {
                Some((text, total)) => {
                    let end = (offset + limit - 1).min(total);
                    ctx.store.add_returned(text.len());
                    Ok(format!("[{id}] lines {offset}-{end} of {total}\n{text}"))
                }
                None => Ok(format!("unknown id {id}")),
            }
        }
        "ctx_stats" => Ok(serde_json::to_string_pretty(&ctx.store.stats()?)?),
        other => Err(anyhow!("unknown tool {other}")),
    }
}
