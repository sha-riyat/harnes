//! Unified event log for harnes.
//!
//! One append-only JSONL file (`.harnes/telemetry/events.jsonl`) is the source
//! of truth; a SQLite index (`.harnes/telemetry.db`) makes runs, roles, files,
//! commands, approvals and token usage queryable by the cockpit.
//!
//! The schema is deliberately close to OpenTelemetry GenAI names, but kept
//! small and stable: `harnes.event.v1`. Events are redacted before they are
//! written. Large payloads belong in `harnes-ctx`; here we keep a digest.

use anyhow::{Context, Result};
use parking_lot::Mutex;
use regex::Regex;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
    sync::OnceLock,
};

pub const SCHEMA: &str = "harnes.event.v1";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Event {
    pub schema: String,
    pub id: String,
    /// Epoch milliseconds.
    pub at: u64,
    pub project: String,
    pub run: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
    #[serde(default)]
    pub role: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lot: Option<String>,
    /// Dot-separated kind, e.g. `file.modified`, `tool.invoked`, `gate.decided`.
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tool: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// ok | error | denied | asked | running | unknown
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(default)]
    pub input_tokens: u64,
    #[serde(default)]
    pub output_tokens: u64,
    #[serde(default)]
    pub cache_read_tokens: u64,
    #[serde(default)]
    pub cache_write_tokens: u64,
    #[serde(default)]
    pub cost_usd: f64,
    #[serde(default)]
    pub bytes: u64,
    #[serde(default)]
    pub summary: String,
    #[serde(default)]
    pub detail: serde_json::Value,
    /// observed | inferred
    #[serde(default = "observed")]
    pub fidelity: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<String>,
    #[serde(default)]
    pub redacted: bool,
}

fn observed() -> String {
    "observed".into()
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

impl Event {
    pub fn new(
        run: impl Into<String>,
        project: impl Into<String>,
        kind: impl Into<String>,
    ) -> Self {
        Self {
            schema: SCHEMA.into(),
            at: now_ms(),
            run: run.into(),
            project: project.into(),
            kind: kind.into(),
            fidelity: observed(),
            ..Default::default()
        }
    }

    pub fn provider(mut self, provider: impl Into<String>) -> Self {
        self.provider = Some(provider.into());
        self
    }

    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    pub fn usage(mut self, input: u64, output: u64, cache_read: u64, cache_write: u64) -> Self {
        self.input_tokens = input;
        self.output_tokens = output;
        self.cache_read_tokens = cache_read;
        self.cache_write_tokens = cache_write;
        self
    }

    pub fn role(mut self, role: impl Into<String>) -> Self {
        self.role = role.into();
        self
    }

    pub fn lot(mut self, lot: impl Into<String>) -> Self {
        self.lot = Some(lot.into());
        self
    }

    pub fn tool(mut self, tool: impl Into<String>) -> Self {
        self.tool = Some(tool.into());
        self
    }

    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn command(mut self, command: impl Into<String>) -> Self {
        self.command = Some(command.into());
        self
    }

    pub fn status(mut self, status: impl Into<String>) -> Self {
        self.status = Some(status.into());
        self
    }

    pub fn summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = summary.into();
        self
    }

    pub fn evidence(mut self, evidence: Vec<String>) -> Self {
        self.evidence = evidence;
        self
    }

    pub fn detail(mut self, detail: serde_json::Value) -> Self {
        self.detail = detail;
        self
    }

    /// Deterministic id: same facts produce the same event, so retries and
    /// resumed runs do not duplicate history.
    pub fn seal(mut self) -> Self {
        if self.schema.is_empty() {
            self.schema = SCHEMA.into();
        }
        if self.id.is_empty() {
            let mut h = Sha256::new();
            for part in [
                self.run.as_str(),
                self.session.as_deref().unwrap_or(""),
                self.role.as_str(),
                self.lot.as_deref().unwrap_or(""),
                self.kind.as_str(),
                self.tool.as_deref().unwrap_or(""),
                self.path.as_deref().unwrap_or(""),
                self.command.as_deref().unwrap_or(""),
                self.summary.as_str(),
                &self.at.to_string(),
            ] {
                h.update(part.as_bytes());
                h.update([0]);
            }
            let d = h.finalize();
            self.id = d.iter().take(12).map(|b| format!("{b:02x}")).collect();
        }
        self
    }
}

#[derive(Debug, Clone, Default)]
pub struct Filter {
    pub run: Option<String>,
    pub kind: Option<String>,
    pub role: Option<String>,
    pub path: Option<String>,
    pub since: Option<u64>,
    pub limit: usize,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Summary {
    pub events: u64,
    /// Completed model requests, counted on the stream start event.
    pub requests: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub cost_usd: f64,
    pub files: Vec<String>,
    pub commands: u64,
    pub errors: u64,
    pub approvals: u64,
    pub denials: u64,
    pub findings: u64,
    pub by_kind: BTreeMap<String, u64>,
    pub by_role: BTreeMap<String, u64>,
}

pub struct Telemetry {
    root: PathBuf,
    jsonl: PathBuf,
    conn: Mutex<Connection>,
}

impl Telemetry {
    pub fn open(root: &Path) -> Result<Self> {
        let dir = root.join(".harnes").join("telemetry");
        std::fs::create_dir_all(&dir)?;
        let jsonl = dir.join("events.jsonl");
        let conn = Connection::open(root.join(".harnes").join("telemetry.db"))
            .context("open telemetry.db")?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA busy_timeout=5000;
             CREATE TABLE IF NOT EXISTS events(
               id TEXT PRIMARY KEY, at INTEGER NOT NULL, project TEXT NOT NULL, run TEXT NOT NULL,
               session TEXT, role TEXT NOT NULL DEFAULT '', lot TEXT, kind TEXT NOT NULL,
               provider TEXT, model TEXT, tool TEXT, path TEXT, command TEXT, status TEXT,
               input_tokens INTEGER NOT NULL DEFAULT 0, output_tokens INTEGER NOT NULL DEFAULT 0,
               cache_read_tokens INTEGER NOT NULL DEFAULT 0, cache_write_tokens INTEGER NOT NULL DEFAULT 0,
               cost_usd REAL NOT NULL DEFAULT 0, bytes INTEGER NOT NULL DEFAULT 0,
               summary TEXT NOT NULL DEFAULT '', detail TEXT NOT NULL DEFAULT '{}',
               fidelity TEXT NOT NULL DEFAULT 'observed', evidence TEXT NOT NULL DEFAULT '[]', redacted INTEGER NOT NULL DEFAULT 0);
             CREATE INDEX IF NOT EXISTS events_run ON events(run, at);
             CREATE INDEX IF NOT EXISTS events_kind ON events(kind, at);
             CREATE INDEX IF NOT EXISTS events_role ON events(run, role);
             CREATE INDEX IF NOT EXISTS events_path ON events(path);",
        )?;
        Ok(Self {
            root: root.to_path_buf(),
            jsonl,
            conn: Mutex::new(conn),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Redacts, seals, indexes and appends one event.
    pub fn append(&self, event: Event) -> Result<Event> {
        let mut e = redact_event(event);
        e = e.seal();
        {
            let c = self.conn.lock();
            let inserted = c.execute(
                "INSERT OR IGNORE INTO events(id, at, project, run, session, role, lot, kind, provider, model, tool, path, command, status,
                  input_tokens, output_tokens, cache_read_tokens, cache_write_tokens, cost_usd, bytes, summary, detail, fidelity, evidence, redacted)
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25)",
                params![
                    e.id, e.at as i64, e.project, e.run, e.session, e.role, e.lot, e.kind, e.provider, e.model, e.tool, e.path, e.command,
                    e.status, e.input_tokens as i64, e.output_tokens as i64, e.cache_read_tokens as i64, e.cache_write_tokens as i64,
                    e.cost_usd, e.bytes as i64, e.summary, e.detail.to_string(), e.fidelity, serde_json::to_string(&e.evidence)?, e.redacted as i32
                ],
            )?;
            if inserted > 0 {
                use std::io::Write;
                let mut f = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&self.jsonl)?;
                writeln!(f, "{}", serde_json::to_string(&e)?)?;
            }
        }
        Ok(e)
    }

    pub fn append_many(&self, events: impl IntoIterator<Item = Event>) -> Result<Vec<Event>> {
        events.into_iter().map(|e| self.append(e)).collect()
    }

    pub fn query(&self, f: &Filter) -> Result<Vec<Event>> {
        let c = self.conn.lock();
        let limit = if f.limit == 0 { 500 } else { f.limit.min(5000) } as i64;
        let mut st = c.prepare(
            "SELECT id, at, project, run, session, role, lot, kind, provider, model, tool, path, command, status,
                    input_tokens, output_tokens, cache_read_tokens, cache_write_tokens, cost_usd, bytes, summary, detail, fidelity, evidence, redacted
             FROM events
             WHERE (?1 IS NULL OR run = ?1) AND (?2 IS NULL OR kind = ?2) AND (?3 IS NULL OR role = ?3)
               AND (?4 IS NULL OR path LIKE ?4) AND (?5 IS NULL OR at >= ?5)
             ORDER BY at DESC, id DESC LIMIT ?6",
        )?;
        let rows = st.query_map(
            params![
                f.run,
                f.kind,
                f.role,
                f.path,
                f.since.map(|v| v as i64),
                limit
            ],
            row_event,
        )?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn recent(&self, run: &str, limit: usize) -> Result<Vec<Event>> {
        self.query(&Filter {
            run: Some(run.into()),
            limit,
            ..Default::default()
        })
    }

    pub fn summary(&self, run: &str) -> Result<Summary> {
        let events = self.query(&Filter {
            run: Some(run.into()),
            limit: 5000,
            ..Default::default()
        })?;
        let mut s = Summary {
            events: events.len() as u64,
            ..Default::default()
        };
        let mut files = BTreeSet::new();
        for e in &events {
            s.input_tokens += e.input_tokens;
            s.output_tokens += e.output_tokens;
            s.cache_read_tokens += e.cache_read_tokens;
            s.cache_write_tokens += e.cache_write_tokens;
            s.cost_usd += e.cost_usd;
            *s.by_kind.entry(e.kind.clone()).or_default() += 1;
            *s.by_role
                .entry(if e.role.is_empty() {
                    "system".into()
                } else {
                    e.role.clone()
                })
                .or_default() += 1;
            if let Some(p) = &e.path {
                files.insert(p.clone());
            }
            if e.kind == "command.executed" {
                s.commands += 1;
            }
            if e.kind == "model.usage" && e.status.as_deref() == Some("message_start") {
                s.requests += 1;
            }
            if e.kind == "error" || e.status.as_deref() == Some("error") {
                s.errors += 1;
            }
            if e.kind == "approval.requested" {
                s.approvals += 1;
            }
            if e.kind == "approval.denied"
                || (e.kind == "guard.decision" && e.status.as_deref() == Some("denied"))
            {
                s.denials += 1;
            }
            if e.kind.starts_with("finding.") {
                s.findings += 1;
            }
        }
        s.files = files.into_iter().collect();
        Ok(s)
    }

    /// Distinct runs, newest first.
    pub fn runs(&self) -> Result<Vec<(String, u64, u64)>> {
        let c = self.conn.lock();
        let mut st = c.prepare(
            "SELECT run, MIN(at), MAX(at) FROM events GROUP BY run ORDER BY MAX(at) DESC LIMIT 200",
        )?;
        let rows = st.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)? as u64,
                r.get::<_, i64>(2)? as u64,
            ))
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn get(&self, id: &str) -> Result<Option<Event>> {
        let c = self.conn.lock();
        Ok(c.query_row(
            "SELECT id, at, project, run, session, role, lot, kind, provider, model, tool, path, command, status,
                    input_tokens, output_tokens, cache_read_tokens, cache_write_tokens, cost_usd, bytes, summary, detail, fidelity, evidence, redacted
             FROM events WHERE id = ?1",
            [id],
            row_event,
        )
        .optional()?)
    }
}

fn row_event(r: &rusqlite::Row) -> rusqlite::Result<Event> {
    let evidence: String = r.get(23)?;
    let detail: String = r.get(21)?;
    Ok(Event {
        schema: SCHEMA.into(),
        id: r.get(0)?,
        at: r.get::<_, i64>(1)? as u64,
        project: r.get(2)?,
        run: r.get(3)?,
        session: r.get(4)?,
        role: r.get(5)?,
        lot: r.get(6)?,
        kind: r.get(7)?,
        provider: r.get(8)?,
        model: r.get(9)?,
        tool: r.get(10)?,
        path: r.get(11)?,
        command: r.get(12)?,
        status: r.get(13)?,
        input_tokens: r.get::<_, i64>(14)? as u64,
        output_tokens: r.get::<_, i64>(15)? as u64,
        cache_read_tokens: r.get::<_, i64>(16)? as u64,
        cache_write_tokens: r.get::<_, i64>(17)? as u64,
        cost_usd: r.get(18)?,
        bytes: r.get::<_, i64>(19)? as u64,
        summary: r.get(20)?,
        detail: serde_json::from_str(&detail).unwrap_or_default(),
        fidelity: r.get(22)?,
        evidence: serde_json::from_str(&evidence).unwrap_or_default(),
        redacted: r.get::<_, i32>(24)? != 0,
    })
}

fn patterns() -> &'static Vec<Regex> {
    static P: OnceLock<Vec<Regex>> = OnceLock::new();
    P.get_or_init(|| {
        [
            r"sk-[A-Za-z0-9_-]{16,}",
            r"ghp_[A-Za-z0-9]{20,}",
            r"gho_[A-Za-z0-9]{20,}",
            r"github_pat_[A-Za-z0-9_]{20,}",
            r"xox[baprs]-[A-Za-z0-9-]{10,}",
            r"AKIA[0-9A-Z]{16}",
            r"eyJ[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}\.[A-Za-z0-9_-]{10,}",
            r"-----BEGIN [A-Z ]*PRIVATE KEY-----[\s\S]*?-----END [A-Z ]*PRIVATE KEY-----",
            r"(?i)(token|secret|password|api[_-]?key)\s*[:=]\s*\S+",
        ]
        .iter()
        .filter_map(|p| Regex::new(p).ok())
        .collect()
    })
}

pub fn redact_text(text: &str) -> (String, bool) {
    let mut out = text.to_string();
    let mut changed = false;
    for re in patterns() {
        if re.is_match(&out) {
            out = re.replace_all(&out, "[redacted]").to_string();
            changed = true;
        }
    }
    (out, changed)
}

fn redact_value(v: &mut serde_json::Value, changed: &mut bool) {
    match v {
        serde_json::Value::String(s) => {
            let (r, c) = redact_text(s);
            if c {
                *s = r;
                *changed = true;
            }
        }
        serde_json::Value::Array(a) => a.iter_mut().for_each(|x| redact_value(x, changed)),
        serde_json::Value::Object(o) => o.values_mut().for_each(|x| redact_value(x, changed)),
        _ => {}
    }
}

fn redact_event(mut e: Event) -> Event {
    let mut changed = false;
    let (summary, c) = redact_text(&e.summary);
    if c {
        e.summary = summary;
        changed = true;
    }
    if let Some(command) = &mut e.command {
        let (r, c) = redact_text(command);
        if c {
            *command = r;
            changed = true;
        }
    }
    if let Some(s) = &mut e.path {
        let (r, c) = redact_text(s);
        if c {
            *s = r;
            changed = true;
        }
    }
    redact_value(&mut e.detail, &mut changed);
    e.redacted |= changed;
    e
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("harnes-telemetry-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn appends_queries_and_summarises() {
        let dir = temp("query");
        let t = Telemetry::open(&dir).unwrap();
        t.append(
            Event::new("r1", "p", "command.executed")
                .role("worker")
                .lot("L1")
                .command("npm test")
                .status("ok")
                .summary("tests"),
        )
        .unwrap();
        t.append(
            Event::new("r1", "p", "file.modified")
                .role("worker")
                .path("src/a.rs")
                .summary("edit"),
        )
        .unwrap();
        t.append(
            Event::new("r1", "p", "model.usage")
                .role("worker")
                .usage(120, 0, 80, 7)
                .status("message_start")
                .summary("start"),
        )
        .unwrap();
        t.append(
            Event::new("r1", "p", "model.usage")
                .role("worker")
                .usage(0, 42, 0, 0)
                .status("message_delta")
                .summary("delta"),
        )
        .unwrap();
        let ev = t.recent("r1", 10).unwrap();
        assert_eq!(ev.len(), 4);
        let s = t.summary("r1").unwrap();
        assert_eq!((s.commands, s.files.len()), (1, 1));
        assert_eq!(
            (
                s.input_tokens,
                s.output_tokens,
                s.cache_read_tokens,
                s.cache_write_tokens
            ),
            (120, 42, 80, 7)
        );
        assert_eq!((s.requests, s.by_role.get("worker")), (1, Some(&4)));
        assert!(dir.join(".harnes/telemetry/events.jsonl").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn counts_approvals_without_double_counting_answers() {
        let dir = temp("approvals");
        let t = Telemetry::open(&dir).unwrap();
        t.append(
            Event::new("r", "p", "approval.requested")
                .status("pending")
                .summary("ask"),
        )
        .unwrap();
        t.append(
            Event::new("r", "p", "approval.granted")
                .status("allowed")
                .summary("ok"),
        )
        .unwrap();
        t.append(
            Event::new("r", "p", "approval.denied")
                .status("denied")
                .summary("no"),
        )
        .unwrap();
        t.append(
            Event::new("r", "p", "guard.decision")
                .status("denied")
                .summary("guard"),
        )
        .unwrap();
        let s = t.summary("r").unwrap();
        assert_eq!((s.approvals, s.denials), (1, 2));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn redacts_secrets_and_deduplicates_ids() {
        let dir = temp("redact");
        let t = Telemetry::open(&dir).unwrap();
        let e = Event::new("r", "p", "tool.invoked")
            .command("curl -H 'x: sk-abcdefghijklmnopqrstuvwx'")
            .summary("token=supersecret");
        let a = t.append(e.clone()).unwrap();
        let b = t.append(e).unwrap();
        assert!(a.redacted && a.command.as_deref().unwrap().contains("[redacted]"));
        assert_eq!(a.id, b.id);
        assert_eq!(t.recent("r", 10).unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
