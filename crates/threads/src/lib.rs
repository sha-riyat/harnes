//! Threads: how harnes agents, sub-agents, the orchestrator and the user talk.
//!
//! One SQLite file per project (`.harnes/threads.db`), shared by every
//! process (each agent's MCP server, the app). Closed threads stay readable.

use anyhow::{bail, Context, Result};
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Thread {
    pub id: String,
    pub title: String,
    /// run | lot | review | question | chat
    pub kind: String,
    pub run: Option<String>,
    /// open | closed
    pub status: String,
    pub created_by: String,
    pub created_at: u64,
    pub closed_at: Option<u64>,
    pub summary: Option<String>,
    pub messages: u64,
    pub last_at: u64,
    pub last_author: Option<String>,
    pub last_body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: i64,
    pub thread: String,
    pub author: String,
    pub at: u64,
    /// message | event | handoff
    pub kind: String,
    pub body: String,
}

pub struct Threads {
    conn: Mutex<Connection>,
}

pub fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

const THREAD_COLS: &str = "t.id, t.title, t.kind, t.run, t.status, t.created_by, t.created_at, t.closed_at, t.summary,
    (SELECT COUNT(*) FROM messages m WHERE m.thread = t.id),
    COALESCE((SELECT MAX(at) FROM messages m WHERE m.thread = t.id), t.created_at),
    (SELECT author FROM messages m WHERE m.thread = t.id ORDER BY id DESC LIMIT 1),
    (SELECT body FROM messages m WHERE m.thread = t.id ORDER BY id DESC LIMIT 1)";

fn row_thread(r: &rusqlite::Row) -> rusqlite::Result<Thread> {
    Ok(Thread {
        id: r.get(0)?,
        title: r.get(1)?,
        kind: r.get(2)?,
        run: r.get(3)?,
        status: r.get(4)?,
        created_by: r.get(5)?,
        created_at: r.get::<_, i64>(6)? as u64,
        closed_at: r.get::<_, Option<i64>>(7)?.map(|v| v as u64),
        summary: r.get(8)?,
        messages: r.get::<_, i64>(9)? as u64,
        last_at: r.get::<_, i64>(10)? as u64,
        last_author: r.get(11)?,
        last_body: r.get(12)?,
    })
}

impl Threads {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d).ok();
        }
        Self::init(Connection::open(path).with_context(|| format!("open {}", path.display()))?)
    }

    pub fn in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self> {
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA busy_timeout=5000;
             CREATE TABLE IF NOT EXISTS threads(
               id TEXT PRIMARY KEY, title TEXT NOT NULL, kind TEXT NOT NULL, run TEXT,
               status TEXT NOT NULL DEFAULT 'open', created_by TEXT NOT NULL, created_at INTEGER NOT NULL,
               closed_at INTEGER, summary TEXT);
             CREATE TABLE IF NOT EXISTS messages(
               id INTEGER PRIMARY KEY AUTOINCREMENT, thread TEXT NOT NULL REFERENCES threads(id),
               author TEXT NOT NULL, at INTEGER NOT NULL, kind TEXT NOT NULL DEFAULT 'message', body TEXT NOT NULL);
             CREATE INDEX IF NOT EXISTS messages_thread ON messages(thread, id);",
        )?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Creates the thread if missing (idempotent on `id`) and returns it.
    pub fn ensure(&self, id: &str, title: &str, kind: &str, run: Option<&str>, by: &str) -> Result<Thread> {
        {
            let c = self.conn.lock();
            c.execute(
                "INSERT OR IGNORE INTO threads(id, title, kind, run, created_by, created_at) VALUES(?1, ?2, ?3, ?4, ?5, ?6)",
                params![id, title, kind, run, by, now_ms() as i64],
            )?;
        }
        self.get(id)?.context("thread vanished")
    }

    /// New thread with a generated id.
    pub fn create(&self, title: &str, kind: &str, run: Option<&str>, by: &str) -> Result<Thread> {
        let id = format!("t-{}", now_ms() % 100_000_000);
        self.ensure(&id, title, kind, run, by)
    }

    pub fn get(&self, id: &str) -> Result<Option<Thread>> {
        let c = self.conn.lock();
        Ok(c.query_row(&format!("SELECT {THREAD_COLS} FROM threads t WHERE t.id = ?1"), [id], row_thread).optional()?)
    }

    pub fn post(&self, thread: &str, author: &str, kind: &str, body: &str) -> Result<Message> {
        if body.trim().is_empty() {
            bail!("message vide");
        }
        let c = self.conn.lock();
        let status: Option<String> = c.query_row("SELECT status FROM threads WHERE id = ?1", [thread], |r| r.get(0)).optional()?;
        match status.as_deref() {
            None => bail!("thread inconnu : {thread}"),
            // Posting to a closed thread reopens it: the history stays in one place.
            Some("closed") => {
                c.execute("UPDATE threads SET status = 'open', closed_at = NULL WHERE id = ?1", [thread])?;
            }
            _ => {}
        }
        let at = now_ms();
        c.execute("INSERT INTO messages(thread, author, at, kind, body) VALUES(?1, ?2, ?3, ?4, ?5)", params![thread, author, at as i64, kind, body])?;
        Ok(Message { id: c.last_insert_rowid(), thread: thread.into(), author: author.into(), at, kind: kind.into(), body: body.into() })
    }

    pub fn close(&self, thread: &str, by: &str, summary: Option<&str>) -> Result<()> {
        if let Some(s) = summary.filter(|s| !s.trim().is_empty()) {
            self.post(thread, by, "event", &format!("Fil clos : {s}"))?;
        }
        let c = self.conn.lock();
        let n = c.execute("UPDATE threads SET status = 'closed', closed_at = ?2, summary = COALESCE(?3, summary) WHERE id = ?1", params![thread, now_ms() as i64, summary])?;
        if n == 0 {
            bail!("thread inconnu : {thread}");
        }
        Ok(())
    }

    /// Threads, most recently active first. `status`: open | closed | all. `run` filters a crew run.
    pub fn list(&self, status: &str, run: Option<&str>, limit: usize) -> Result<Vec<Thread>> {
        let c = self.conn.lock();
        let mut st = c.prepare(&format!(
            "SELECT {THREAD_COLS} FROM threads t
             WHERE (?1 = 'all' OR t.status = ?1) AND (?2 IS NULL OR t.run = ?2)
             ORDER BY 11 DESC LIMIT ?3"
        ))?;
        let rows = st.query_map(params![status, run, limit as i64], row_thread)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Messages after `since` (exclusive), oldest first.
    pub fn read(&self, thread: &str, since: i64, limit: usize) -> Result<Vec<Message>> {
        let c = self.conn.lock();
        let mut st = c.prepare("SELECT id, thread, author, at, kind, body FROM messages WHERE thread = ?1 AND id > ?2 ORDER BY id LIMIT ?3")?;
        let rows = st.query_map(params![thread, since, limit as i64], |r| {
            Ok(Message { id: r.get(0)?, thread: r.get(1)?, author: r.get(2)?, at: r.get::<_, i64>(3)? as u64, kind: r.get(4)?, body: r.get(5)? })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Latest messages across all threads (for the activity animation).
    pub fn recent(&self, since: i64, limit: usize) -> Result<Vec<Message>> {
        let c = self.conn.lock();
        let mut st = c.prepare("SELECT id, thread, author, at, kind, body FROM messages WHERE id > ?1 ORDER BY id DESC LIMIT ?2")?;
        let rows = st.query_map(params![since, limit as i64], |r| {
            Ok(Message { id: r.get(0)?, thread: r.get(1)?, author: r.get(2)?, at: r.get::<_, i64>(3)? as u64, kind: r.get(4)?, body: r.get(5)? })
        })?;
        let mut v: Vec<Message> = rows.filter_map(|r| r.ok()).collect();
        v.reverse();
        Ok(v)
    }

    /// Cheap change detector.
    pub fn version(&self) -> Result<(i64, i64)> {
        let c = self.conn.lock();
        Ok(c.query_row(
            "SELECT COALESCE((SELECT MAX(id) FROM messages), 0), COALESCE((SELECT SUM(COALESCE(closed_at, 0)) FROM threads), 0)",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_post_close_reopen() {
        let t = Threads::in_memory().unwrap();
        let th = t.ensure("lot-1-L1", "L1 : page contact", "lot", Some("1"), "harnes").unwrap();
        assert_eq!(th.status, "open");
        t.ensure("lot-1-L1", "autre titre", "lot", Some("1"), "x").unwrap(); // idempotent
        t.post(&th.id, "worker-L1", "message", "Question : quel endpoint ?").unwrap();
        let m = t.post(&th.id, "planner", "message", "POST /api/contact").unwrap();
        assert_eq!(t.read(&th.id, 0, 10).unwrap().len(), 2);
        assert_eq!(t.read(&th.id, m.id - 1, 10).unwrap()[0].author, "planner");

        t.close(&th.id, "reviewer", Some("lot validé")).unwrap();
        let closed = t.list("closed", None, 10).unwrap();
        assert_eq!(closed.len(), 1);
        assert_eq!(closed[0].summary.as_deref(), Some("lot validé"));
        assert_eq!(closed[0].messages, 3, "history kept after closing");

        t.post(&th.id, "worker-L1", "message", "relance").unwrap();
        assert_eq!(t.get(&th.id).unwrap().unwrap().status, "open");
        assert!(t.post("nope", "x", "message", "y").is_err());
        assert_eq!(t.list("all", Some("1"), 10).unwrap()[0].last_body.as_deref(), Some("relance"));
    }
}
