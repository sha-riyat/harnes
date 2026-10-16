//! Provider-independent session context.
//!
//! When a session moves to another provider (Claude -> Codex, …), harnes
//! turns the old session's transcript into a hand-off file the new agent
//! reads first, so no context is lost.

use anyhow::Result;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// Keeps the most recent part of a transcript.
const MAX_CHARS: usize = 60_000;
const TOOL_RESULT_CHARS: usize = 400;

fn home() -> PathBuf {
    std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(PathBuf::from).unwrap_or_default()
}

/// Finds `~/.claude/projects/*/<session>.jsonl`.
pub fn claude_transcript(session: &str) -> Option<PathBuf> {
    let dir = home().join(".claude").join("projects");
    std::fs::read_dir(dir).ok()?.flatten().map(|d| d.path().join(format!("{session}.jsonl"))).find(|p| p.exists())
}

fn clip(s: &str, max: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(max).collect::<String>())
    }
}

/// One line per meaningful entry: user prompts, assistant text, tool calls and short results.
pub fn export_claude(path: &Path) -> Result<String> {
    let mut out = Vec::new();
    for line in std::fs::read_to_string(path)?.lines() {
        let Ok(v) = serde_json::from_str::<Value>(line) else { continue };
        if v["isSidechain"].as_bool() == Some(true) {
            continue; // sub-agent internals
        }
        let role = v["type"].as_str().unwrap_or("");
        if role != "user" && role != "assistant" {
            continue;
        }
        let content = &v["message"]["content"];
        if let Some(text) = content.as_str() {
            out.push(format!("**{}** : {}", if role == "user" { "Utilisateur" } else { "Agent" }, text.trim()));
            continue;
        }
        for b in content.as_array().into_iter().flatten() {
            match b["type"].as_str() {
                Some("text") => out.push(format!("**{}** : {}", if role == "user" { "Utilisateur" } else { "Agent" }, b["text"].as_str().unwrap_or("").trim())),
                Some("tool_use") => out.push(format!("> outil `{}` {}", b["name"].as_str().unwrap_or("?"), clip(&b["input"].to_string(), 200))),
                Some("tool_result") => {
                    let text = match &b["content"] {
                        Value::String(s) => s.clone(),
                        Value::Array(parts) => parts.iter().filter_map(|p| p["text"].as_str()).collect::<Vec<_>>().join(" "),
                        _ => String::new(),
                    };
                    if !text.trim().is_empty() {
                        out.push(format!("> résultat : {}", clip(&text, TOOL_RESULT_CHARS)));
                    }
                }
                _ => {}
            }
        }
    }
    Ok(tail(out))
}

/// One item of a conversation, for the UI (orchestrator chat).
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct ConvItem {
    /// user | assistant | tool
    pub role: String,
    pub text: String,
    /// Tool name for `role = tool`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    /// ISO timestamp from the transcript.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
}

/// Turns a Claude Code transcript into chat items: user prompts, assistant
/// text and one short line per tool call (results and thinking omitted).
pub fn conversation_claude(path: &Path, limit: usize) -> Result<Vec<ConvItem>> {
    let mut items = Vec::new();
    for line in std::fs::read_to_string(path)?.lines() {
        let Ok(v) = serde_json::from_str::<Value>(line) else { continue };
        if v["isSidechain"].as_bool() == Some(true) {
            continue;
        }
        let role = v["type"].as_str().unwrap_or("");
        if role != "user" && role != "assistant" {
            continue;
        }
        let at = v["timestamp"].as_str().map(String::from);
        let content = &v["message"]["content"];
        if let Some(text) = content.as_str() {
            // Skip injected command/system wrappers.
            if !text.trim_start().starts_with('<') && !text.trim().is_empty() {
                items.push(ConvItem { role: role.into(), text: text.trim().into(), tool: None, at });
            }
            continue;
        }
        for b in content.as_array().into_iter().flatten() {
            match b["type"].as_str() {
                Some("text") => {
                    let t = b["text"].as_str().unwrap_or("").trim();
                    if !t.is_empty() && !t.starts_with('<') {
                        items.push(ConvItem { role: role.into(), text: t.into(), tool: None, at: at.clone() });
                    }
                }
                Some("tool_use") => {
                    let name = b["name"].as_str().unwrap_or("?").replace("mcp__harnes-ctx__", "").replace("mcp__harnes-e2e__", "");
                    let input = &b["input"];
                    let hint = ["task", "body", "title", "thread", "command", "file_path", "pattern", "description", "prompt"]
                        .iter()
                        .find_map(|k| input[*k].as_str())
                        .unwrap_or("");
                    items.push(ConvItem { role: "tool".into(), text: clip(hint, 140), tool: Some(name), at: at.clone() });
                }
                _ => {}
            }
        }
    }
    let skip = items.len().saturating_sub(limit);
    Ok(items.into_iter().skip(skip).collect())
}

/// Codex rollout: user / assistant messages.
pub fn export_codex(path: &Path) -> Result<String> {
    let mut out = Vec::new();
    for line in std::fs::read_to_string(path)?.lines() {
        let Ok(v) = serde_json::from_str::<Value>(line) else { continue };
        let p = &v["payload"];
        if v["type"] != "response_item" || p["type"] != "message" {
            continue;
        }
        let who = if p["role"] == "user" { "Utilisateur" } else { "Agent" };
        for c in p["content"].as_array().into_iter().flatten() {
            if let Some(t) = c["text"].as_str() {
                out.push(format!("**{who}** : {}", t.trim()));
            }
        }
    }
    Ok(tail(out))
}

fn tail(lines: Vec<String>) -> String {
    let mut kept = Vec::new();
    let mut size = 0;
    for l in lines.iter().rev() {
        size += l.len() + 1;
        if size > MAX_CHARS {
            kept.push("_(début de la session omis)_".to_string());
            break;
        }
        kept.push(l.clone());
    }
    kept.reverse();
    kept.join("\n")
}

/// Writes `.harnes/context/<key>-<ts>.md` and returns its project-relative path.
pub fn write_handoff(root: &Path, key: &str, from: &str, to: &str, transcript: &str, threads: &str) -> Result<String> {
    let dir = root.join(".harnes").join("context");
    std::fs::create_dir_all(&dir)?;
    let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let name = format!("{key}-{ts}.md");
    let body = format!(
        "# Passation de session\n\nSession `{key}` reprise par **{to}** (avant : {from}).\n\
         Lis ce fichier en entier : c'est tout le contexte de la session précédente. Continue exactement là où elle s'est arrêtée, \
         sans refaire ce qui est déjà fait.\n\n## Fils de discussion récents\n\n{}\n\n## Transcription de la session précédente\n\n{}\n",
        if threads.trim().is_empty() { "_(aucun)_" } else { threads },
        if transcript.trim().is_empty() { "_(transcription indisponible)_" } else { transcript }
    );
    std::fs::write(dir.join(&name), body)?;
    Ok(format!(".harnes/context/{name}"))
}

pub fn resume_prompt(handoff: &str) -> String {
    format!("Reprends cette session : lis d'abord `{handoff}` (contexte complet de la session précédente, y compris ses derniers échanges), puis continue le travail en cours.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exports_claude_transcript_and_writes_handoff() {
        let dir = std::env::temp_dir().join(format!("harnes-ctx-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let t = dir.join("s.jsonl");
        std::fs::write(
            &t,
            [
                r#"{"type":"user","message":{"role":"user","content":"Ajoute une page contact"}}"#,
                r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"thinking","thinking":"secret"},{"type":"tool_use","name":"Write","input":{"file_path":"contact.html"}}]}}"#,
                r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","content":"File created"}]}}"#,
                r#"{"type":"assistant","isSidechain":true,"message":{"role":"assistant","content":"sub-agent noise"}}"#,
                r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Page créée."}]}}"#,
            ]
            .join("\n"),
        )
        .unwrap();
        let md = export_claude(&t).unwrap();
        assert!(md.contains("Ajoute une page contact") && md.contains("outil `Write`") && md.contains("File created") && md.contains("Page créée."));
        assert!(!md.contains("secret") && !md.contains("sub-agent noise"));
        let conv = conversation_claude(&t, 50).unwrap();
        assert_eq!(conv.iter().map(|c| c.role.as_str()).collect::<Vec<_>>(), ["user", "tool", "assistant"]);
        assert_eq!(conv[1].tool.as_deref(), Some("Write"));
        assert_eq!(conv[1].text, "contact.html");
        let rel = write_handoff(&dir, "claude-1", "claude", "codex", &md, "").unwrap();
        assert!(std::fs::read_to_string(dir.join(&rel)).unwrap().contains("reprise par **codex**"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
