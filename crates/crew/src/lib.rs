//! Crew orchestration: project setup, agent state from Claude Code hooks,
//! and the Planner -> Workers -> Reviewer loop.

pub mod context;
pub mod e2e;
pub mod engine;
pub mod hooks;
pub mod permissions;
pub mod plan;
pub mod prompts;
pub mod providers;
pub mod setup;
pub mod threading;

use serde::Serialize;

/// Lifecycle of one agent pane, as reported by Claude Code hooks.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentState {
    Starting,
    Idle,
    Working,
    Blocked,
    Done,
    Exited,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Planner,
    Teacher,
    Worker,
    Reviewer,
}

impl Role {
    pub fn agent(self) -> &'static str {
        match self {
            Role::Planner => "planner",
            Role::Teacher => "teacher",
            Role::Worker => "worker",
            Role::Reviewer => "reviewer",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Idle,
    Planning,
    Calibrating,
    Working,
    Reviewing,
    Gating,
    Fixing,
    Done,
    Failed,
    Cancelled,
}

/// Progress reported to the UI.
#[derive(Debug, Clone, Serialize)]
pub struct CrewEvent {
    pub run: String,
    pub phase: Phase,
    pub round: u32,
    pub message: String,
    /// Set when the run ends: final review (commits proposed, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review: Option<plan::Review>,
    /// Set when the Teacher lets the work through: what to tell the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gate: Option<plan::Gate>,
}
