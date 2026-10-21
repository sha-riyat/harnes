//! Hooks -> telemetry + runtime guards.
//!
//! The hook server is shared by every project, so the callback resolves the
//! agent key back to its project. PreToolUse gets a decision; every hook is
//! appended to that project's event log (redacted by harnes-telemetry).

use crate::state::Shared;
use harnes_guards::{hook_response, ToolInput};
use serde_json::{json, Value};
use std::sync::Arc;

fn run_of(p: &crate::state::Project) -> String {
    p.run_slot.lock().clone().unwrap_or_default()
}

fn role_of(st: &Shared, key: &str) -> String {
    st.agents
        .read()
        .get(key)
        .map(|a| a.role.clone())
        .unwrap_or_default()
}

fn path_of(payload: &Value) -> Option<String> {
    let input = payload.get("tool_input")?;
    ["file_path", "path", "notebook_path", "target_file"]
        .iter()
        .find_map(|k| input.get(*k).and_then(Value::as_str))
        .map(String::from)
}

fn command_of(payload: &Value) -> Option<String> {
    let input = payload.get("tool_input")?;
    ["command", "cmd", "script"]
        .iter()
        .find_map(|k| input.get(*k).and_then(Value::as_str))
        .map(String::from)
}

fn tool_of(payload: &Value) -> String {
    payload
        .get("tool_name")
        .and_then(Value::as_str)
        .unwrap_or("hook")
        .to_string()
}

fn kind_of(event: &str, tool: Option<&str>) -> &'static str {
    match event {
        "PreToolUse" => "tool.invoked",
        "PostToolUse" => match tool {
            Some("Read") | Some("NotebookRead") => "file.read",
            Some("Write") | Some("Edit") | Some("MultiEdit") | Some("NotebookEdit") => {
                "file.modified"
            }
            Some("Bash") | Some("PowerShell") | Some("Shell") => "command.executed",
            Some("WebFetch") | Some("WebSearch") => "network.request",
            _ => "tool.completed",
        },
        "Stop" => "agent.turn.ended",
        "SessionStart" => "agent.session.started",
        "SessionEnd" => "agent.session.ended",
        "Notification" => "agent.notification",
        "UserPromptSubmit" => "agent.prompt.submitted",
        _ => "hook",
    }
}

pub fn install(st: &Shared) {
    let app = st.app.clone();
    st.hooks.set_guard(Arc::new(move |key: &str, payload: &Value| {
        use tauri::Manager;
        let Some(st) = app.try_state::<Shared>() else { return json!({}) };
        let Some(p) = st.project_of_key(key) else { return json!({}) };
        let event = payload.get("hook_event_name").and_then(Value::as_str).unwrap_or("");
        let tool = tool_of(payload);
        let input = ToolInput::from_hook(payload);
        let run = run_of(&p);
        let role = role_of(&st, key);
        let mut base = harnes_telemetry::Event::new(&run, &p.id, kind_of(event, Some(&tool)))
        .role(role)
        .tool(tool.clone())
        .summary(format!("{event} {tool}"));
        if let Some(path) = path_of(payload) {
            base = base.path(path);
        }
        if let Some(command) = command_of(payload) {
            base = base.command(command);
        }
        if let Some(session) = payload.get("session_id").and_then(Value::as_str) {
            base.session = Some(session.to_string());
        }
        if event == "Notification" {
            // PermissionRequest is the authoritative approval event and is
            // recorded by the permission broker. A Notification may mention
            // the same prompt, so keep it informative without double-counting
            // approvals.
            if let Some(msg) = payload.get("message").and_then(Value::as_str) {
                base.summary = msg.to_string();
            }
            if let Some(kind) = payload.get("notification_type").and_then(Value::as_str) {
                base.status = Some(kind.to_string());
            }
        }
        if event == "PostToolUse" {
            let failed = payload.get("tool_response").and_then(|r| r.get("is_error")).and_then(Value::as_bool).unwrap_or(false);
            base.status = Some(if failed { "error" } else { "ok" }.into());
        }
        let _ = p.telemetry.append(base);

        if event == "PreToolUse" {
            let decision = p.guards.read().decide(&input);
            if decision.action != harnes_guards::Action::Allow {
                let _ = p.telemetry.append(
                    harnes_telemetry::Event::new(&run, &p.id, "guard.decision")
                        .role(role_of(&st, key))
                        .tool(tool.clone())
                        .path(input.path.clone().unwrap_or_default())
                        .status(match decision.action {
                            harnes_guards::Action::Deny => "denied",
                            harnes_guards::Action::Ask => "asked",
                            harnes_guards::Action::Allow => "allow",
                        })
                        .summary(decision.reason.clone().unwrap_or_default())
                        .detail(serde_json::json!({"rule": decision.rule, "severity": decision.severity})),
                );
            }
            return hook_response(&decision);
        }

        // Post-hoc correlation (secret read -> egress, denied -> retry).
        if event == "PostToolUse" {
            let recent = p.telemetry.recent(&run, 500).unwrap_or_default();
            for f in harnes_guards::scan(&recent) {
                let evidence = f.evidence.join("-");
                let mut finding = harnes_telemetry::Event::new(&run, &p.id, format!("finding.{}", f.rule))
                    .role(role_of(&st, key))
                    .status(match f.severity {
                        harnes_guards::Severity::Critical => "critical",
                        harnes_guards::Severity::High => "high",
                        harnes_guards::Severity::Warn => "warn",
                        harnes_guards::Severity::Info => "info",
                    })
                    .summary(f.message.clone())
                    .detail(serde_json::json!({"severity": f.severity, "evidence": f.evidence}));
                // Stable id: the same correlation is not reported on every hook.
                finding.id = format!("finding-{}-{evidence}", f.rule);
                let _ = p.telemetry.append(finding);
            }
        }
        json!({})
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_tool_fields() {
        let p = json!({"hook_event_name":"PreToolUse","tool_name":"Edit","tool_input":{"file_path":"src/a.rs"},"session_id":"s"});
        assert_eq!(path_of(&p).as_deref(), Some("src/a.rs"));
        assert!(command_of(&p).is_none());
        let p = json!({"tool_name":"Bash","tool_input":{"command":"npm test"}});
        assert_eq!(command_of(&p).as_deref(), Some("npm test"));
    }

    #[test]
    fn maps_completed_tools_to_semantic_events() {
        assert_eq!(kind_of("PostToolUse", Some("Read")), "file.read");
        assert_eq!(kind_of("PostToolUse", Some("Edit")), "file.modified");
        assert_eq!(kind_of("PostToolUse", Some("Bash")), "command.executed");
        assert_eq!(kind_of("PostToolUse", Some("WebFetch")), "network.request");
        assert_eq!(kind_of("PreToolUse", Some("Bash")), "tool.invoked");
        assert_eq!(kind_of("Stop", None), "agent.turn.ended");
    }
}
