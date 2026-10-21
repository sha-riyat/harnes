//! MCP tools over the OS pack library (method packs for agents).

use anyhow::{anyhow, Result};
use harnes_os::Library;
use serde_json::{json, Value};

pub fn definitions() -> Vec<Value> {
    vec![
        json!({
            "name": "os_list",
            "description": "List the installed OS packs (method packs: doctrine, oracle, skills) with their purpose and the crew roles that use them.",
            "inputSchema": {"type": "object", "properties": {}}
        }),
        json!({
            "name": "os_brief",
            "description": "Load an OS pack before acting with it: doctrine (OS.md), oracle (non-negotiable defaults and refusals), first principles and its skills. Follow the oracle.",
            "inputSchema": {"type": "object", "properties": {"id": {"type": "string", "description": "Pack id, e.g. builder-os"}}, "required": ["id"]}
        }),
        json!({
            "name": "os_route",
            "description": "Find which OS pack and skill fit a request (phrases are in English, e.g. 'review this diff', 'good enough to ship').",
            "inputSchema": {"type": "object", "properties": {
                "text": {"type": "string"},
                "limit": {"type": "integer", "default": 5}
            }, "required": ["text"]}
        }),
        json!({
            "name": "os_read",
            "description": "Read one file of an OS pack: skills/<name>/SKILL.md, playbooks/*.md, workflows/*.yaml, knowledge/…, evals/rubric.yaml.",
            "inputSchema": {"type": "object", "properties": {
                "id": {"type": "string"},
                "path": {"type": "string"}
            }, "required": ["id", "path"]}
        }),
    ]
}

fn arg<'a>(args: &'a Value, key: &str) -> Result<&'a str> {
    args.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("missing argument: {key}"))
}

/// `None` when the tool is not an OS tool.
pub fn call(lib: &Library, name: &str, args: &Value) -> Result<Option<String>> {
    Ok(Some(match name {
        "os_list" => {
            let packs = lib.list();
            if packs.is_empty() {
                return Ok(Some(format!(
                    "no OS packs installed in {}",
                    lib.dir.display()
                )));
            }
            packs
                .iter()
                .map(|p| {
                    let roles = if p.roles.is_empty() {
                        String::new()
                    } else {
                        format!(" [{}]", p.roles.join(", "))
                    };
                    format!("{}{roles}: {}", p.id, p.purpose)
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
        "os_brief" => lib.brief(arg(args, "id")?)?,
        "os_route" => {
            let limit = args.get("limit").and_then(Value::as_u64).unwrap_or(5) as usize;
            let hits = lib.route(arg(args, "text")?, limit.clamp(1, 20));
            if hits.is_empty() {
                "no pack matches; call os_list".into()
            } else {
                hits.iter()
                    .map(|h| format!("{:.2} {} · {} -> {}", h.score, h.os, h.intent, h.route))
                    .collect::<Vec<_>>()
                    .join("\n")
            }
        }
        "os_read" => lib.read(arg(args, "id")?, arg(args, "path")?)?,
        _ => return Ok(None),
    }))
}
