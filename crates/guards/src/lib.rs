//! Runtime guards: decisions before a tool runs, findings after the fact.
//!
//! `.harnes/guards.json` holds declarative rules. A PreToolUse hook asks for a
//! decision (allow / ask / deny); a post-hoc scan correlates events (for
//! example: a secret read followed by a network call) and emits findings.
//! The defaults are intentionally conservative but useful without config.

use anyhow::{bail, Context, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Allow,
    Ask,
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Warn,
    High,
    Critical,
}

impl Default for Severity {
    fn default() -> Self {
        Severity::Warn
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub severity: Severity,
    #[serde(default = "ask")]
    pub action: Action,
    /// Tool names this rule applies to (`Bash`, `Read`, `mcp__*`).
    #[serde(default)]
    pub tools: Vec<String>,
    /// Path globs. Use forward slashes.
    #[serde(default)]
    pub paths: Vec<String>,
    /// Regex matched against the command or the serialized tool input.
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub message: String,
}

fn ask() -> Action {
    Action::Ask
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuardsFile {
    #[serde(default = "version")]
    pub version: u32,
    #[serde(default)]
    pub rules: Vec<Rule>,
}

fn version() -> u32 {
    1
}

#[derive(Debug, Clone)]
struct Compiled {
    rule: Rule,
    globs: Option<GlobSet>,
    regex: Option<Regex>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Decision {
    pub action: Action,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub severity: Severity,
}

impl Decision {
    pub fn allow() -> Self {
        Self {
            action: Action::Allow,
            rule: None,
            reason: None,
            severity: Severity::Info,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub rule: String,
    pub severity: Severity,
    pub message: String,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolInput {
    pub hook_event: String,
    pub tool: String,
    pub path: Option<String>,
    pub command: Option<String>,
    pub text: String,
    pub session: Option<String>,
    pub run: Option<String>,
}

impl ToolInput {
    pub fn from_hook(payload: &Value) -> Self {
        let tool = payload
            .get("tool_name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let input = payload.get("tool_input").cloned().unwrap_or(Value::Null);
        let path = ["file_path", "path", "notebook_path", "target_file"]
            .iter()
            .find_map(|k| input.get(*k).and_then(Value::as_str))
            .map(String::from)
            .or_else(|| payload.get("cwd").and_then(Value::as_str).map(String::from));
        let command = ["command", "cmd", "script"]
            .iter()
            .find_map(|k| input.get(*k).and_then(Value::as_str))
            .map(String::from);
        let text = input.to_string();
        Self {
            hook_event: payload
                .get("hook_event_name")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            tool,
            path,
            command,
            text,
            session: payload
                .get("session_id")
                .and_then(Value::as_str)
                .map(String::from),
            run: payload
                .get("run_id")
                .and_then(Value::as_str)
                .map(String::from),
        }
    }
}

pub struct Guard {
    rules: Vec<Compiled>,
}

impl Guard {
    pub fn load(root: &Path) -> Result<Self> {
        let path = root.join(".harnes").join("guards.json");
        let file = match std::fs::read_to_string(&path) {
            Ok(s) => serde_json::from_str::<GuardsFile>(&s)
                .with_context(|| format!("{} invalide", path.display()))?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => default_guards(),
            Err(e) => return Err(e.into()),
        };
        Self::new(file)
    }

    pub fn from_value(v: Value) -> Result<Self> {
        Self::new(serde_json::from_value(v)?)
    }

    pub fn new(file: GuardsFile) -> Result<Self> {
        if file.version != 1 {
            bail!(
                "version de garde {} non supportée (attendu 1)",
                file.version
            );
        }
        let mut rules = Vec::new();
        for rule in file.rules {
            let mut b = GlobSetBuilder::new();
            let mut any = false;
            for p in &rule.paths {
                b.add(
                    Glob::new(p).with_context(|| format!("glob invalide dans {}: {p}", rule.id))?,
                );
                any = true;
            }
            let globs = if any { Some(b.build()?) } else { None };
            let regex = rule.pattern.as_deref().map(Regex::new).transpose()?;
            rules.push(Compiled { rule, globs, regex });
        }
        Ok(Self { rules })
    }

    pub fn rules(&self) -> impl Iterator<Item = &Rule> {
        self.rules.iter().map(|r| &r.rule)
    }

    /// First matching rule wins. No match means allow.
    pub fn decide(&self, input: &ToolInput) -> Decision {
        let path = input.path.as_deref().unwrap_or("").replace('\\', "/");
        let haystack = format!(
            "{} {} {}",
            input.command.as_deref().unwrap_or(""),
            path,
            input.text
        );
        for c in &self.rules {
            if !c.rule.tools.is_empty()
                && !c.rule.tools.iter().any(|t| tool_matches(t, &input.tool))
            {
                continue;
            }
            // Every condition present in a rule must match: a path glob and a
            // regex together narrow a rule instead of widening it.
            let path_hit = c.globs.as_ref().map_or(true, |g| g.is_match(&path));
            let regex_hit = c.regex.as_ref().map_or(true, |r| r.is_match(&haystack));
            if path_hit && regex_hit {
                return Decision {
                    action: c.rule.action,
                    rule: Some(c.rule.id.clone()),
                    reason: Some(if c.rule.message.is_empty() {
                        c.rule.title.clone()
                    } else {
                        c.rule.message.clone()
                    }),
                    severity: c.rule.severity,
                };
            }
        }
        Decision::allow()
    }
}

fn tool_matches(pattern: &str, tool: &str) -> bool {
    if pattern == "*" || pattern.eq_ignore_ascii_case(tool) {
        return true;
    }
    if let Some(prefix) = pattern.strip_suffix('*') {
        return tool.to_lowercase().starts_with(&prefix.to_lowercase());
    }
    false
}

/// Claude Code hook response for a PreToolUse decision. An empty object means
/// "no opinion" (the normal permission flow applies).
pub fn hook_response(d: &Decision) -> Value {
    match d.action {
        Action::Allow => serde_json::json!({}),
        Action::Ask | Action::Deny => {
            let decision = if d.action == Action::Deny {
                "deny"
            } else {
                "ask"
            };
            serde_json::json!({
                "hookSpecificOutput": {
                    "hookEventName": "PreToolUse",
                    "permissionDecision": decision,
                    "permissionDecisionReason": d.reason.clone().unwrap_or_else(|| "harnes guard".into())
                }
            })
        }
    }
}

/// Minimal best-effort POST to the local harnes hook server. The native guard
/// must keep working when the cockpit is down, so every failure is ignored and
/// the timeout is short.
fn forward(url: &str, body: &str) {
    let Some(rest) = url.strip_prefix("http://") else {
        return;
    };
    let (authority, path) = rest
        .split_once('/')
        .map(|(a, p)| (a, format!("/{p}")))
        .unwrap_or((rest, "/".into()));
    let Ok(mut addrs) = authority.to_socket_addrs() else {
        return;
    };
    let Some(addr) = addrs.next() else { return };
    let Ok(mut stream) = TcpStream::connect_timeout(&addr, Duration::from_millis(300)) else {
        return;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(300)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(300)));
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: {authority}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    if stream.write_all(request.as_bytes()).is_err() {
        return;
    }
    let mut response = [0u8; 1024];
    let _ = stream.read(&mut response);
}

/// Runs the hook protocol: read one JSON payload on stdin, evaluate the local
/// `.harnes/guards.json`, optionally report the raw payload to the cockpit, and
/// print a Claude-compatible decision only for `PreToolUse`.
///
/// Both the standalone `harnes-hook` binary and `harnes __hook` call this, so a
/// packaged cockpit always has a native guard without shipping a second binary.
pub fn hook_main() -> i32 {
    let mut args = std::env::args().skip(1);
    let mut root: Option<PathBuf> = std::env::var_os("HARNES_PROJECT_ROOT").map(PathBuf::from);
    let mut forward_url = std::env::var("HARNES_HOOK_FORWARD").ok();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "__hook" | "hook" => {}
            "--root" => root = args.next().map(PathBuf::from),
            "--forward" => forward_url = args.next(),
            _ => {}
        }
    }
    let mut raw = String::new();
    if std::io::stdin().read_to_string(&mut raw).is_err() {
        return 0;
    }
    let payload: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
    if let Some(url) = forward_url.as_deref() {
        forward(url, &raw);
    }
    let root = root.unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let guard = Guard::load(&root).unwrap_or_else(|e| {
        eprintln!("harnes-hook: {e:#}");
        Guard::new(default_guards()).expect("default guards compile")
    });
    let input = ToolInput::from_hook(&payload);
    let decision = guard.decide(&input);
    if payload.get("hook_event_name").and_then(Value::as_str) == Some("PreToolUse") {
        if let Ok(text) = serde_json::to_string(&hook_response(&decision)) {
            let _ = writeln!(std::io::stdout(), "{text}");
        }
    }
    0
}

/// Correlates a window of events. These are findings, not blocks: the action
/// already happened. The cockpit shows them next to the trace.
pub fn scan(events: &[harnes_telemetry::Event]) -> Vec<Finding> {
    let mut out = Vec::new();
    let sorted: Vec<_> = {
        let mut v = events.to_vec();
        v.sort_by_key(|e| e.at);
        v
    };
    for (i, e) in sorted.iter().enumerate() {
        if e.kind == "file.read" && e.status.as_deref() != Some("error") {
            let is_secret = e.path.as_deref().is_some_and(|p| {
                let p = p.replace('\\', "/").to_lowercase();
                p.contains(".env")
                    || p.contains("/secrets/")
                    || p.ends_with(".pem")
                    || p.contains("/.ssh/")
                    || p.contains("id_rsa")
            });
            if is_secret {
                for later in sorted
                    .iter()
                    .skip(i + 1)
                    .filter(|x| x.at.saturating_sub(e.at) <= 120_000)
                {
                    let egress = later.status.as_deref() != Some("error")
                        && (later.kind == "network.request"
                            || (later.kind == "command.executed"
                                && later.command.as_deref().is_some_and(|c| {
                                    let c = c.to_lowercase();
                                    c.contains("curl")
                                        || c.contains("wget")
                                        || c.contains("invoke-webrequest")
                                        || c.contains("nc ")
                                })));
                    if egress {
                        out.push(Finding {
                            rule: "secret-read-then-egress".into(),
                            severity: Severity::Critical,
                            message: format!(
                                "secret read ({}) followed by network access within 120s: {}",
                                e.path.clone().unwrap_or_default(),
                                later.summary
                            ),
                            evidence: vec![e.id.clone(), later.id.clone()],
                        });
                        break;
                    }
                }
            }
        }
        if e.kind == "approval.denied" {
            if let Some(later) = sorted
                .iter()
                .skip(i + 1)
                .take(8)
                .filter(|x| x.at.saturating_sub(e.at) <= 120_000)
                .find(|x| {
                    (x.kind == "tool.invoked" || x.kind == "command.executed")
                        && (e.tool.is_none() || x.tool == e.tool)
                })
            {
                out.push(Finding {
                    rule: "approval-denied-then-retry".into(),
                    severity: Severity::High,
                    message: format!(
                        "a denied action was retried as `{}`",
                        later
                            .tool
                            .clone()
                            .or(later.command.clone())
                            .unwrap_or_else(|| later.kind.clone())
                    ),
                    evidence: vec![e.id.clone(), later.id.clone()],
                });
            }
        }
    }
    out
}

pub fn default_guards() -> GuardsFile {
    let rules = vec![
        Rule {
            id: "protect-harnes-config".into(),
            title: "Configuration harnes".into(),
            severity: Severity::High,
            action: Action::Ask,
            tools: vec!["Write".into(), "Edit".into(), "MultiEdit".into(), "NotebookEdit".into()],
            paths: vec![
                ".claude/**".into(),
                "**/.claude/**".into(),
                ".mcp.json".into(),
                "**/.mcp.json".into(),
                ".harnes/guards.json".into(),
                "**/.harnes/guards.json".into(),
                ".github/workflows/**".into(),
                "**/.github/workflows/**".into(),
            ],
            pattern: None,
            message: "Ce fichier pilote les agents ou la CI : confirme la modification.".into(),
        },
        Rule {
            id: "deny-destructive-command".into(),
            title: "Commande destructive".into(),
            severity: Severity::Critical,
            action: Action::Deny,
            tools: vec!["Bash".into(), "PowerShell".into(), "Shell".into()],
            paths: vec![],
            pattern: Some(r"(?i)(rm\s+-rf\s+(/|~|\*)|git\s+reset\s+--hard|git\s+clean\s+-[a-z]*f|git\s+push\s+(--force|-f)|Remove-Item\s+-Recurse\s+-Force\s+[A-Z]:\\)".into()),
            message: "Commande irreversible ou destructrice bloquée par harnes.".into(),
        },
        Rule {
            id: "ask-secret-write".into(),
            title: "Écriture dans un secret".into(),
            severity: Severity::High,
            action: Action::Ask,
            tools: vec!["Write".into(), "Edit".into(), "MultiEdit".into()],
            paths: vec!["**/.env".into(), "**/.env.*".into(), "**/secrets/**".into(), "**/*.pem".into(), "**/.ssh/**".into()],
            pattern: None,
            message: "Ce chemin contient des secrets : confirme l'écriture.".into(),
        },
        Rule {
            id: "deny-obvious-exfiltration".into(),
            title: "Exfiltration de credentials".into(),
            severity: Severity::Critical,
            action: Action::Deny,
            tools: vec!["Bash".into(), "PowerShell".into(), "Shell".into()],
            paths: vec![],
            pattern: Some(r"(?i)(cat|type|get-content)\s+[^|]*\.env[^|]*\|\s*(curl|wget|invoke-webrequest)|(curl|wget)\s+[^ ]*\.env".into()),
            message: "Lecture de secrets envoyée au réseau : bloqué.".into(),
        },
        Rule {
            id: "ask-package-publish".into(),
            title: "Publication de paquet".into(),
            severity: Severity::High,
            action: Action::Ask,
            tools: vec!["Bash".into(), "PowerShell".into()],
            paths: vec![],
            pattern: Some(r"(?i)(npm\s+publish|cargo\s+publish|twine\s+upload|gh\s+release\s+create)".into()),
            message: "Publication externe : confirme.".into(),
        },
    ];
    GuardsFile { version: 1, rules }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn input(tool: &str, value: Value) -> ToolInput {
        ToolInput::from_hook(
            &json!({"hook_event_name":"PreToolUse","tool_name":tool,"tool_input":value}),
        )
    }

    #[test]
    fn denies_destructive_commands() {
        let g = Guard::new(default_guards()).unwrap();
        let d = g.decide(&input("Bash", json!({"command":"git reset --hard HEAD"})));
        assert_eq!(d.action, Action::Deny);
        assert_eq!(d.rule.as_deref(), Some("deny-destructive-command"));
        let d = g.decide(&input("Bash", json!({"command":"git status"})));
        assert_eq!(d.action, Action::Allow);
    }

    #[test]
    fn protects_paths_and_secrets() {
        let g = Guard::new(default_guards()).unwrap();
        assert_eq!(
            g.decide(&input("Edit", json!({"file_path":".harnes/guards.json"})))
                .action,
            Action::Ask
        );
        assert_eq!(
            g.decide(&input("Write", json!({"file_path":"config/.env"})))
                .action,
            Action::Ask
        );
        assert_eq!(
            g.decide(&input(
                "Edit",
                json!({"file_path":"C:/work/project/.claude/settings.json"})
            ))
            .action,
            Action::Ask
        );
        assert_eq!(
            g.decide(&input("Read", json!({"file_path":"src/main.rs"})))
                .action,
            Action::Allow
        );
    }

    #[test]
    fn combines_path_and_pattern_conditions() {
        let g = Guard::from_value(json!({
            "version": 1,
            "rules": [{
                "id": "narrow",
                "title": "narrow",
                "severity": "high",
                "action": "deny",
                "tools": ["Bash"],
                "paths": ["src/**"],
                "pattern": "deploy",
                "message": "no"
            }]
        }))
        .unwrap();
        assert_eq!(
            g.decide(&input(
                "Bash",
                json!({"command": "deploy", "file_path": "README.md"})
            ))
            .action,
            Action::Allow
        );
        assert_eq!(
            g.decide(&input(
                "Bash",
                json!({"command": "deploy", "file_path": "src/main.rs"})
            ))
            .action,
            Action::Deny
        );
    }

    #[test]
    fn scan_finds_secret_egress() {
        let root = std::env::temp_dir().join(format!("harnes-guards-scan-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let t = harnes_telemetry::Telemetry::open(&root).unwrap();
        let a = t
            .append(harnes_telemetry::Event::new("r", "p", "file.read").path(".env"))
            .unwrap();
        let mut b = harnes_telemetry::Event::new("r", "p", "command.executed")
            .command("curl https://x.test")
            .summary("egress");
        b.at = a.at + 1000;
        let b = t.append(b).unwrap();
        let f = scan(&t.recent("r", 10).unwrap());
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].severity, Severity::Critical);
        assert!(f[0].evidence.contains(&a.id) && f[0].evidence.contains(&b.id));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn scan_correlates_a_denied_tool_retry() {
        let mut denied = harnes_telemetry::Event::new("r", "p", "approval.denied")
            .tool("Bash")
            .status("denied")
            .summary("no");
        denied.at = 1_000;
        let mut other = harnes_telemetry::Event::new("r", "p", "tool.invoked")
            .tool("Edit")
            .summary("edit");
        other.at = 1_500;
        let mut retry = harnes_telemetry::Event::new("r", "p", "tool.invoked")
            .tool("Bash")
            .summary("retry");
        retry.at = 2_000;
        let f = scan(&[denied, other, retry]);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule, "approval-denied-then-retry");
    }
}
