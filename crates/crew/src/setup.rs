//! Equips a project for harnes: agents, settings, MCP registration, CLAUDE.md import.

use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const PLANNER: &str = include_str!("../../../template/.claude/agents/planner.md");
const WORKER: &str = include_str!("../../../template/.claude/agents/worker.md");
const REVIEWER: &str = include_str!("../../../template/.claude/agents/reviewer.md");
const TEACHER: &str = include_str!("../../../template/.claude/agents/teacher.md");
const SCOUT: &str = include_str!("../../../template/.claude/agents/scout.md");
const ORCHESTRATOR: &str = include_str!("../../../template/.claude/agents/orchestrator.md");
const SETTINGS: &str = include_str!("../../../template/.claude/settings.json");
const HARNESS_MD: &str = include_str!("../../../template/CLAUDE.md");
const IMPORT: &str = "@.claude/HARNESS.md";

#[derive(Debug, Default, serde::Serialize)]
pub struct SetupReport {
    pub written: Vec<String>,
    pub kept: Vec<String>,
}

fn write_if(path: &Path, content: &str, force: bool, rep: &mut SetupReport, root: &Path) -> Result<()> {
    let rel = path.strip_prefix(root).unwrap_or(path).display().to_string();
    if path.exists() && !force {
        rep.kept.push(rel);
        return Ok(());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, content).with_context(|| format!("write {}", path.display()))?;
    rep.written.push(rel);
    Ok(())
}

/// Merges `harnes-ctx` into `.mcp.json`, keeping other servers.
fn register_mcp(root: &Path, ctx_bin: &Path, rep: &mut SetupReport) -> Result<()> {
    let path = root.join(".mcp.json");
    let mut doc: Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| json!({}));
    if !doc.get("mcpServers").is_some_and(Value::is_object) {
        doc["mcpServers"] = json!({});
    }
    doc["mcpServers"]["harnes-ctx"] = json!({
        "type": "stdio",
        "command": ctx_bin.display().to_string(),
        "args": ["--root", "."]
    });
    // harnes-e2e MCP: drive the live app, resolve `act` steps, run specs.
    let e2e_bin = ctx_bin.with_file_name(if cfg!(windows) { "harnes-e2e.exe" } else { "harnes-e2e" });
    doc["mcpServers"]["harnes-e2e"] = json!({
        "type": "stdio",
        "command": e2e_bin.display().to_string(),
        "args": ["mcp", "--root", "."]
    });
    if let Some(m) = doc["mcpServers"].as_object_mut() {
        m.remove("e2e");
    }
    std::fs::write(&path, serde_json::to_string_pretty(&doc)? + "\n")?;
    rep.written.push(".mcp.json".into());
    Ok(())
}

/// Idempotent. `force` overwrites agent and settings files.
pub fn equip(root: &Path, ctx_bin: &Path, force: bool) -> Result<SetupReport> {
    let mut rep = SetupReport::default();
    let agents = root.join(".claude").join("agents");
    write_if(&agents.join("planner.md"), PLANNER, force, &mut rep, root)?;
    write_if(&agents.join("worker.md"), WORKER, force, &mut rep, root)?;
    write_if(&agents.join("reviewer.md"), REVIEWER, force, &mut rep, root)?;
    write_if(&agents.join("teacher.md"), TEACHER, force, &mut rep, root)?;
    write_if(&agents.join("scout.md"), SCOUT, force, &mut rep, root)?;
    write_if(&agents.join("orchestrator.md"), ORCHESTRATOR, force, &mut rep, root)?;
    write_if(&root.join(".claude").join("settings.json"), SETTINGS, force, &mut rep, root)?;
    // HARNESS.md is ours: rewritten each time, with the installed OS packs.
    let harness = format!("{HARNESS_MD}{}", harnes_os::Library::open_default().index_md());
    write_if(&root.join(".claude").join("HARNESS.md"), &harness, true, &mut rep, root)?;
    register_mcp(root, ctx_bin, &mut rep)?;

    let claude_md = root.join("CLAUDE.md");
    let current = std::fs::read_to_string(&claude_md).unwrap_or_default();
    if !current.contains(IMPORT) {
        let sep = if current.is_empty() || current.ends_with('\n') { "" } else { "\n" };
        std::fs::write(&claude_md, format!("{current}{sep}{IMPORT}\n"))?;
        rep.written.push("CLAUDE.md".into());
    }

    // Keep run artefacts out of git.
    let gi = root.join(".harnes").join(".gitignore");
    write_if(&gi, "*\n", false, &mut rep, root)?;
    Ok(rep)
}

/// Directory for one crew run.
pub fn run_dir(root: &Path, run: &str) -> PathBuf {
    root.join(".harnes").join("runs").join(run)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equips_idempotently_and_keeps_other_mcp_servers() {
        let dir = std::env::temp_dir().join(format!("harnes-setup-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(".mcp.json"), r#"{"mcpServers":{"other":{"command":"x"}}}"#).unwrap();
        std::fs::write(dir.join("CLAUDE.md"), "# Mine").unwrap();

        equip(&dir, Path::new("C:/bin/harnes-ctx.exe"), false).unwrap();
        let second = equip(&dir, Path::new("C:/bin/harnes-ctx.exe"), false).unwrap();
        assert!(second.kept.iter().any(|k| k.ends_with("planner.md")));

        let mcp: Value = serde_json::from_str(&std::fs::read_to_string(dir.join(".mcp.json")).unwrap()).unwrap();
        assert!(mcp["mcpServers"]["other"].is_object());
        assert!(mcp["mcpServers"]["harnes-ctx"]["command"].as_str().unwrap().ends_with("harnes-ctx.exe"));
        assert!(mcp["mcpServers"]["harnes-e2e"]["command"].as_str().unwrap().ends_with("harnes-e2e.exe"));
        let md = std::fs::read_to_string(dir.join("CLAUDE.md")).unwrap();
        assert_eq!(md.matches(IMPORT).count(), 1);
        assert!(md.starts_with("# Mine"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
