//! Minimal MCP server over stdio (newline-delimited JSON-RPC 2.0).

use crate::tools::{self, Ctx};
use anyhow::Result;
use serde_json::{json, Value};
use std::io::{BufRead, Write};

const PROTOCOL: &str = "2025-06-18";

pub fn handle(ctx: &Ctx, msg: &Value) -> Option<Value> {
    let id = msg.get("id").cloned();
    let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
    // Notifications carry no id and get no response.
    let id = id?;
    let result = match method {
        "initialize" => {
            let version = msg
                .pointer("/params/protocolVersion")
                .and_then(Value::as_str)
                .unwrap_or(PROTOCOL);
            Ok(json!({
                "protocolVersion": version,
                "capabilities": {"tools": {}, "resources": {}, "prompts": {}},
                "serverInfo": {"name": "harnes-ctx", "version": env!("CARGO_PKG_VERSION")},
                "instructions": "Use ctx_execute / ctx_execute_file for commands or files that may produce large output; use ctx_search and ctx_retrieve to read stored content, including content elided by the harnes proxy. OS packs and approved memory are also exposed as MCP resources and prompts."
            }))
        }
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({"tools": tools::all_definitions(ctx)})),
        "tools/call" => {
            let name = msg
                .pointer("/params/name")
                .and_then(Value::as_str)
                .unwrap_or("");
            let args = msg
                .pointer("/params/arguments")
                .cloned()
                .unwrap_or(json!({}));
            Ok(match tools::call(ctx, name, &args) {
                Ok(text) => json!({"content": [{"type": "text", "text": text}]}),
                Err(e) => {
                    json!({"content": [{"type": "text", "text": format!("error: {e:#}")}], "isError": true})
                }
            })
        }
        "resources/list" => Ok(json!({"resources": resources(ctx)})),
        "resources/read" => {
            let uri = msg
                .pointer("/params/uri")
                .and_then(Value::as_str)
                .unwrap_or("");
            match read_resource(ctx, uri) {
                Ok(text) => Ok(
                    json!({"contents": [{"uri": uri, "mimeType": "text/markdown", "text": text}]}),
                ),
                Err(e) => Err(json!({"code": -32602, "message": format!("{e:#}")})),
            }
        }
        "prompts/list" => Ok(json!({"prompts": [
            {"name": "recall", "description": "Approved project memory relevant to a task.", "arguments": [{"name": "query", "required": true}]},
            {"name": "handoff", "description": "Instructions for resuming from a hand-off file.", "arguments": [{"name": "path", "required": true}]},
            {"name": "method", "description": "Load an OS method pack (doctrine + oracle + skills).", "arguments": [{"name": "pack", "required": true}]}
        ]})),
        "prompts/get" => {
            let name = msg
                .pointer("/params/name")
                .and_then(Value::as_str)
                .unwrap_or("");
            let args = msg
                .pointer("/params/arguments")
                .cloned()
                .unwrap_or(json!({}));
            let text = match name {
                "recall" => {
                    let q = args.get("query").and_then(Value::as_str).unwrap_or("");
                    match &ctx.memory {
                        Some(m) => m.context(q, 8).unwrap_or_else(|e| format!("error: {e:#}")),
                        None => "No project memory available.".into(),
                    }
                }
                "handoff" => {
                    let p = args.get("path").and_then(Value::as_str).unwrap_or("");
                    format!("Read `{p}` first and continue exactly where the previous session stopped. Do not redo completed work.")
                }
                "method" => {
                    let p = args.get("pack").and_then(Value::as_str).unwrap_or("");
                    ctx.os.brief(p).unwrap_or_else(|e| format!("error: {e:#}"))
                }
                other => {
                    return Some(
                        json!({"jsonrpc": "2.0", "id": id, "error": {"code": -32602, "message": format!("unknown prompt {other}")}}),
                    )
                }
            };
            Ok(
                json!({"description": name, "messages": [{"role": "user", "content": {"type": "text", "text": text}}]}),
            )
        }
        _ => Err(json!({"code": -32601, "message": format!("method not found: {method}")})),
    };
    Some(match result {
        Ok(r) => json!({"jsonrpc": "2.0", "id": id, "result": r}),
        Err(e) => json!({"jsonrpc": "2.0", "id": id, "error": e}),
    })
}

fn resources(ctx: &Ctx) -> Vec<Value> {
    let mut out = ctx
        .os
        .list()
        .into_iter()
        .map(|p| {
            json!({
                "uri": format!("harnes://os/{}", p.id),
                "name": p.id,
                "title": p.title,
                "description": p.purpose,
                "mimeType": "text/markdown"
            })
        })
        .collect::<Vec<_>>();
    if ctx.memory.is_some() {
        out.push(json!({
            "uri": "harnes://memory",
            "name": "approved-memory",
            "title": "Mémoire approuvée",
            "description": "Human-approved project lessons, conventions and gotchas with provenance.",
            "mimeType": "application/json"
        }));
    }
    out
}

fn read_resource(ctx: &Ctx, uri: &str) -> Result<String> {
    if uri == "harnes://memory" {
        let memory = ctx
            .memory
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("mémoire indisponible"))?;
        return Ok(serde_json::to_string_pretty(&memory.records(100)?)?);
    }
    let rest = uri
        .strip_prefix("harnes://os/")
        .ok_or_else(|| anyhow::anyhow!("resource inconnue : {uri}"))?;
    let (id, path) = rest.split_once('/').unwrap_or((rest, ""));
    if path.is_empty() {
        ctx.os.brief(id)
    } else {
        ctx.os.read(id, path)
    }
}

pub fn serve(ctx: &Ctx, input: impl BufRead, mut output: impl Write) -> Result<()> {
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Ok(msg) => handle(ctx, &msg),
            Err(e) => Some(
                json!({"jsonrpc": "2.0", "id": null, "error": {"code": -32700, "message": e.to_string()}}),
            ),
        };
        if let Some(r) = reply {
            writeln!(output, "{r}")?;
            output.flush()?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;

    fn ctx() -> Ctx {
        Ctx {
            store: Store::in_memory().unwrap(),
            root: std::env::temp_dir(),
            threads: Some(harnes_threads::Threads::in_memory().unwrap()),
            os: harnes_os::Library::new(std::env::temp_dir().join("harnes-no-os")),
            memory: None,
        }
    }

    #[test]
    fn lists_tools_and_ignores_notifications() {
        let c = ctx();
        assert!(handle(
            &c,
            &json!({"jsonrpc":"2.0","method":"notifications/initialized"})
        )
        .is_none());
        let r = handle(&c, &json!({"jsonrpc":"2.0","id":1,"method":"tools/list"})).unwrap();
        assert_eq!(r["result"]["tools"].as_array().unwrap().len(), 16);
    }

    #[test]
    fn large_output_is_digested_and_searchable() {
        let c = ctx();
        let big: String = (0..2000)
            .map(|i| format!("line {i} ok\n"))
            .collect::<String>()
            + "error: boom at module_x\n";
        let out = crate::summary::digest(&c.store, "test", &big, Some("module_x")).unwrap();
        assert!(out.len() < 4000, "digest too long: {}", out.len());
        assert!(out.contains("error: boom"));
        let hits = c.store.search("module_x", 5).unwrap();
        assert_eq!(hits.len(), 1);
        let (text, total) = c.store.get(&hits[0].id, 1, 3).unwrap().unwrap();
        assert_eq!(total, 2001);
        assert_eq!(text, "line 0 ok\nline 1 ok\nline 2 ok");
    }
}

#[cfg(test)]
mod thread_tests {
    use super::*;
    use crate::store::Store;

    #[test]
    fn agents_post_and_read_threads() {
        let c = Ctx {
            store: Store::in_memory().unwrap(),
            root: std::env::temp_dir(),
            threads: Some(harnes_threads::Threads::in_memory().unwrap()),
            os: harnes_os::Library::new(std::env::temp_dir().join("harnes-no-os")),
            memory: None,
        };
        let call = |name: &str, args: Value| tools::call(&c, name, &args).unwrap();
        let created = call(
            "thread_create",
            json!({"title": "Question API", "body": "Quel endpoint ?"}),
        );
        let id = created.trim_start_matches("thread créé : ").to_string();
        call(
            "thread_post",
            json!({"thread": id, "body": "Je vérifie", "as": "scout"}),
        );
        let read = call("thread_read", json!({"thread": id}));
        assert!(
            read.contains("Quel endpoint ?") && read.contains("/scout : Je vérifie"),
            "{read}"
        );
        call("thread_close", json!({"thread": id, "summary": "réglé"}));
        assert!(call("thread_list", json!({"status": "closed"})).contains("Question API"));
    }
}
