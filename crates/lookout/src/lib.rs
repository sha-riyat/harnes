//! harnes-lookout: a machine-wide view of every coding-agent session.
//!
//! Polls files each tool already writes (Claude Code registry, Codex
//! rollouts, Gemini CLI chats, status files), maps their own status words to
//! six statuses, and turns changes into events. Read-only: it never starts,
//! stops or messages a session. Inspired by agent-lookout (MIT).

pub mod adapters;
pub mod model;

use adapters::Adapter;
use model::{now_ms, Event, EventKind, HistoryPoint, Session, SourceHealth, Snapshot, Status};
use std::collections::{HashMap, VecDeque};

const EVENT_CAPACITY: usize = 1000;
/// One hour at one point every 2 s.
const HISTORY_CAPACITY: usize = 1800;

pub struct Collector {
    adapters: Vec<Box<dyn Adapter>>,
    sessions: HashMap<String, Session>,
    health: Vec<SourceHealth>,
    events: VecDeque<Event>,
    history: VecDeque<HistoryPoint>,
}

impl Default for Collector {
    fn default() -> Self {
        Self::new(vec![
            Box::new(adapters::ClaudeCode),
            Box::new(adapters::Codex),
            Box::new(adapters::GeminiCli),
            Box::new(adapters::StatusFiles),
        ])
    }
}

impl Collector {
    pub fn new(adapters: Vec<Box<dyn Adapter>>) -> Self {
        Self { adapters, sessions: HashMap::new(), health: vec![], events: VecDeque::new(), history: VecDeque::new() }
    }

    fn push_event(&mut self, e: Event) {
        if self.events.len() == EVENT_CAPACITY {
            self.events.pop_front();
        }
        self.events.push_back(e);
    }

    /// Polls every source, merges `extra` sessions (harnes' own agents), diffs, records history.
    pub fn poll(&mut self, extra: Vec<Session>) -> Vec<Event> {
        let mut next: HashMap<String, Session> = HashMap::new();
        let mut health = vec![];
        for a in &self.adapters {
            match a.poll() {
                Ok(Some(list)) => {
                    health.push(SourceHealth { source: a.id().into(), state: "ok".into(), sessions: list.len(), error: None });
                    next.extend(list.into_iter().map(|s| (s.id.clone(), s)));
                }
                Ok(None) => health.push(SourceHealth { source: a.id().into(), state: "not-set-up".into(), sessions: 0, error: None }),
                Err(e) => health.push(SourceHealth { source: a.id().into(), state: "error".into(), sessions: 0, error: Some(format!("{e:#}")) }),
            }
        }
        for s in extra {
            next.insert(s.id.clone(), s);
        }
        self.health = health;

        let at = now_ms();
        let mut fresh = vec![];
        for (id, s) in &next {
            match self.sessions.get(id) {
                None => fresh.push(Event { at, session_id: id.clone(), session_name: s.name.clone(), kind: EventKind::Appeared, from: None, to: Some(s.status) }),
                Some(old) if old.status != s.status => fresh.push(Event {
                    at,
                    session_id: id.clone(),
                    session_name: s.name.clone(),
                    kind: EventKind::StatusChanged,
                    from: Some(old.status),
                    to: Some(s.status),
                }),
                _ => {}
            }
        }
        for (id, old) in &self.sessions {
            if !next.contains_key(id) {
                fresh.push(Event { at, session_id: id.clone(), session_name: old.name.clone(), kind: EventKind::Ended, from: Some(old.status), to: None });
            }
        }
        for e in &fresh {
            self.push_event(e.clone());
        }
        self.sessions = next;

        let count = |st: Status| self.sessions.values().filter(|s| s.status == st).count() as u32;
        if self.history.len() == HISTORY_CAPACITY {
            self.history.pop_front();
        }
        self.history.push_back(HistoryPoint {
            at,
            needs_you: count(Status::NeedsYou),
            working: count(Status::Working),
            idle: count(Status::Idle),
            total: self.sessions.len() as u32,
        });
        fresh
    }

    /// Sessions sorted: needs-you (longest waiting first), working, idle, the rest.
    pub fn snapshot(&self) -> Snapshot {
        let rank = |s: &Status| match s {
            Status::NeedsYou => 0,
            Status::Working => 1,
            Status::Idle => 2,
            Status::Unknown => 3,
            Status::Failed => 4,
            Status::Finished => 5,
        };
        let mut sessions: Vec<Session> = self.sessions.values().cloned().collect();
        sessions.sort_by(|a, b| rank(&a.status).cmp(&rank(&b.status)).then(a.status_since.cmp(&b.status_since)));
        Snapshot { generated_at: now_ms(), sessions, sources: self.health.clone() }
    }

    pub fn events(&self, limit: usize) -> Vec<Event> {
        self.events.iter().rev().take(limit).cloned().collect()
    }

    pub fn history(&self) -> Vec<HistoryPoint> {
        self.history.iter().copied().collect()
    }
}

#[cfg(test)]
mod tests;
