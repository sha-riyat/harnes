//! Session model shared by every source. Adding a field is fine; renaming
//! or removing one breaks the UI.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    /// Waiting on the user: permission prompt or question.
    NeedsYou,
    Working,
    Idle,
    Finished,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WaitingReason {
    Permission,
    Question,
    Other,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tokens {
    pub input: u64,
    pub cached: u64,
    pub output: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Session {
    /// `<source>:<native id>`.
    pub id: String,
    pub source: String,
    /// Provider family: claude, codex, gemini, other.
    pub provider: String,
    pub name: String,
    pub cwd: String,
    pub project: String,
    pub status: Status,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waiting_reason: Option<WaitingReason>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub waiting_text: Option<String>,
    /// Unix millis.
    pub started_at: u64,
    pub status_since: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<Tokens>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    /// Set for agents launched by harnes (crew key).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub harnes_key: Option<String>,
}

impl Session {
    pub fn new(source: &str, provider: &str, native_id: &str, cwd: &str) -> Self {
        let project = std::path::Path::new(cwd)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(cwd)
            .to_string();
        Self {
            id: format!("{source}:{native_id}"),
            source: source.into(),
            provider: provider.into(),
            name: String::new(),
            cwd: cwd.into(),
            project,
            status: Status::Unknown,
            waiting_reason: None,
            waiting_text: None,
            started_at: 0,
            status_since: 0,
            tokens: None,
            pid: None,
            harnes_key: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum EventKind {
    Appeared,
    StatusChanged,
    Ended,
}

#[derive(Debug, Clone, Serialize)]
pub struct Event {
    pub at: u64,
    pub session_id: String,
    pub session_name: String,
    pub kind: EventKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from: Option<Status>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<Status>,
}

/// One point per poll, for the "last hour" chart.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct HistoryPoint {
    pub at: u64,
    pub needs_you: u32,
    pub working: u32,
    pub idle: u32,
    pub total: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct SourceHealth {
    pub source: String,
    /// ok | not-set-up | error
    pub state: String,
    pub sessions: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub generated_at: u64,
    pub sessions: Vec<Session>,
    pub sources: Vec<SourceHealth>,
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
