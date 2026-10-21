//! Permission requests answered from the harnes UI instead of the terminal.
//!
//! Claude Code's `PermissionRequest` hook POSTs the request here and blocks;
//! the UI shows it, the user decides, and the decision is returned as the
//! hook's stdout. If nobody answers in time, no decision is returned and
//! Claude Code falls back to its own prompt.

use parking_lot::Mutex;
use serde::Serialize;
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::oneshot;

/// How long the hook waits for the user (the hook timeout is a little longer).
pub const WAIT: Duration = Duration::from_secs(570);

#[derive(Debug, Clone, Serialize)]
pub struct Ask {
    pub id: String,
    pub agent: String,
    pub tool: String,
    /// Short human summary: the command, file or URL concerned.
    pub summary: String,
    pub input: Value,
    pub can_remember: bool,
    pub at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Decision {
    Allow,
    /// Allow and apply Claude Code's suggested permission rule.
    Always,
    Deny,
}

pub type AskSink = Arc<dyn Fn(&Ask) + Send + Sync>;

#[derive(Default)]
pub struct Broker {
    pending: Mutex<HashMap<String, (Ask, Value, oneshot::Sender<Decision>)>>,
    sink: std::sync::OnceLock<AskSink>,
}

fn summarize(input: &Value) -> String {
    let pick = [
        "command",
        "file_path",
        "path",
        "url",
        "pattern",
        "description",
    ]
    .iter()
    .find_map(|k| input[*k].as_str());
    let s = pick.map(String::from).unwrap_or_else(|| input.to_string());
    let s = s.trim();
    let short: String = s.chars().take(300).collect();
    short
}

impl Broker {
    pub fn set_sink(&self, f: AskSink) {
        let _ = self.sink.set(f);
    }

    pub fn pending(&self) -> Vec<Ask> {
        let mut v: Vec<Ask> = self
            .pending
            .lock()
            .values()
            .map(|(a, ..)| a.clone())
            .collect();
        v.sort_by_key(|a| a.at);
        v
    }

    /// Answers a pending request. Returns false if it already expired.
    pub fn answer(&self, id: &str, d: Decision) -> bool {
        match self.pending.lock().remove(id) {
            Some((_, _, tx)) => tx.send(d).is_ok(),
            None => false,
        }
    }

    /// Called by the hook endpoint: registers the request and waits for the user.
    /// Returns the hook stdout (empty = no decision).
    pub async fn request(&self, agent: &str, payload: Value) -> String {
        let tool = payload["tool_name"].as_str().unwrap_or("").to_string();
        let input = payload["tool_input"].clone();
        let suggestions = payload["permission_suggestions"].clone();
        let id = format!(
            "{:x}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0)
        );
        let ask = Ask {
            id: id.clone(),
            agent: agent.into(),
            summary: summarize(&input),
            tool,
            input,
            can_remember: suggestions.as_array().is_some_and(|a| !a.is_empty()),
            at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        };
        let (tx, rx) = oneshot::channel();
        self.pending
            .lock()
            .insert(id.clone(), (ask.clone(), suggestions.clone(), tx));
        if let Some(sink) = self.sink.get() {
            sink(&ask);
        }
        let decision = tokio::time::timeout(WAIT, rx).await;
        self.pending.lock().remove(&id);
        match decision {
            Ok(Ok(d)) => decision_json(d, &suggestions).to_string(),
            _ => String::new(), // no answer: let Claude Code ask in its own UI
        }
    }
}

pub fn decision_json(d: Decision, suggestions: &Value) -> Value {
    let decision = match d {
        Decision::Allow => json!({"behavior": "allow"}),
        Decision::Always => match suggestions.as_array().filter(|a| !a.is_empty()) {
            Some(s) => json!({"behavior": "allow", "updatedPermissions": s}),
            None => json!({"behavior": "allow"}),
        },
        Decision::Deny => {
            json!({"behavior": "deny", "message": "Refusé par l'utilisateur dans harnes."})
        }
    };
    json!({"hookSpecificOutput": {"hookEventName": "PermissionRequest", "decision": decision}})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn request_waits_for_the_user() {
        let broker = Arc::new(Broker::default());
        let seen = Arc::new(Mutex::new(None::<String>));
        let s2 = seen.clone();
        broker.set_sink(Arc::new(move |a: &Ask| *s2.lock() = Some(a.id.clone())));
        let b2 = broker.clone();
        let task = tokio::spawn(async move {
            b2.request("worker-L1", json!({"tool_name": "Bash", "tool_input": {"command": "npm test"}, "permission_suggestions": [{"type": "addRules"}]})).await
        });
        while seen.lock().is_none() {
            tokio::task::yield_now().await;
        }
        let pending = broker.pending();
        assert_eq!(pending[0].summary, "npm test");
        assert!(pending[0].can_remember);
        assert!(broker.answer(&pending[0].id, Decision::Always));
        let out: Value = serde_json::from_str(&task.await.unwrap()).unwrap();
        assert_eq!(out["hookSpecificOutput"]["decision"]["behavior"], "allow");
        assert!(out["hookSpecificOutput"]["decision"]["updatedPermissions"].is_array());
        assert!(broker.pending().is_empty());
    }

    #[test]
    fn deny_carries_a_message() {
        let v = decision_json(Decision::Deny, &Value::Null);
        assert_eq!(v["hookSpecificOutput"]["decision"]["behavior"], "deny");
    }
}
