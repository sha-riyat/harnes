use anyhow::{Context, Result};
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::Path;

/// Lines per indexed chunk.
const CHUNK_LINES: usize = 40;

#[derive(Debug, Clone, Serialize)]
pub struct Hit {
    pub id: String,
    pub source: String,
    pub line: usize,
    pub snippet: String,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Stats {
    pub blobs: u64,
    pub bytes_stored: u64,
    pub bytes_returned: u64,
}

/// Content-addressed blob store with full-text search.
pub struct Store {
    conn: Mutex<Connection>,
}

/// Stable short id for a piece of content.
pub fn content_id(content: &str) -> String {
    let digest = Sha256::digest(content.as_bytes());
    digest.iter().take(6).map(|b| format!("{b:02x}")).collect()
}

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).ok();
        }
        let conn = Connection::open(path).with_context(|| format!("open {}", path.display()))?;
        Self::init(conn)
    }

    pub fn in_memory() -> Result<Self> {
        Self::init(Connection::open_in_memory()?)
    }

    fn init(conn: Connection) -> Result<Self> {
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA busy_timeout=3000;
             CREATE TABLE IF NOT EXISTS blobs(
                id TEXT PRIMARY KEY, source TEXT NOT NULL, created INTEGER NOT NULL,
                bytes INTEGER NOT NULL, content TEXT NOT NULL);
             CREATE VIRTUAL TABLE IF NOT EXISTS chunks USING fts5(
                blob_id UNINDEXED, line UNINDEXED, body, tokenize='unicode61');
             CREATE TABLE IF NOT EXISTS meta(key TEXT PRIMARY KEY, value INTEGER NOT NULL);",
        )?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Stores and indexes `content`. Returns its id. Idempotent.
    pub fn put(&self, source: &str, content: &str) -> Result<String> {
        let id = content_id(content);
        let mut conn = self.conn.lock();
        let exists: Option<String> = conn.query_row("SELECT id FROM blobs WHERE id=?1", [&id], |r| r.get(0)).optional()?;
        if exists.is_some() {
            return Ok(id);
        }
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let tx = conn.transaction()?;
        tx.execute(
            "INSERT INTO blobs(id, source, created, bytes, content) VALUES(?1, ?2, ?3, ?4, ?5)",
            params![id, source, now as i64, content.len() as i64, content],
        )?;
        {
            let mut stmt = tx.prepare("INSERT INTO chunks(blob_id, line, body) VALUES(?1, ?2, ?3)")?;
            let lines: Vec<&str> = content.lines().collect();
            for (i, chunk) in lines.chunks(CHUNK_LINES).enumerate() {
                stmt.execute(params![id, (i * CHUNK_LINES + 1) as i64, chunk.join("\n")])?;
            }
        }
        tx.commit()?;
        Ok(id)
    }

    /// Full-text search. `query` is free text; it is tokenised safely.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<Hit>> {
        let terms: Vec<String> = query
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|t| t.len() > 1)
            .map(|t| format!("\"{}\"", t.replace('"', "")))
            .collect();
        if terms.is_empty() {
            return Ok(vec![]);
        }
        let fts = terms.join(" OR ");
        let conn = self.conn.lock();
        let mut stmt = conn.prepare(
            "SELECT c.blob_id, b.source, c.line, snippet(chunks, 2, '>>', '<<', ' … ', 24)
             FROM chunks c JOIN blobs b ON b.id = c.blob_id
             WHERE chunks MATCH ?1 ORDER BY bm25(chunks) LIMIT ?2",
        )?;
        let rows = stmt.query_map(params![fts, limit as i64], |r| {
            Ok(Hit { id: r.get(0)?, source: r.get(1)?, line: r.get::<_, i64>(2)? as usize, snippet: r.get(3)? })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Returns lines `[offset, offset+limit)` (1-based offset) of a blob.
    pub fn get(&self, id: &str, offset: usize, limit: usize) -> Result<Option<(String, usize)>> {
        let conn = self.conn.lock();
        let content: Option<String> = conn.query_row("SELECT content FROM blobs WHERE id=?1", [id], |r| r.get(0)).optional()?;
        Ok(content.map(|c| {
            let total = c.lines().count();
            let slice: Vec<&str> = c.lines().skip(offset.saturating_sub(1)).take(limit).collect();
            (slice.join("\n"), total)
        }))
    }

    pub fn add_returned(&self, bytes: usize) {
        let conn = self.conn.lock();
        let _ = conn.execute(
            "INSERT INTO meta(key, value) VALUES('returned', ?1)
             ON CONFLICT(key) DO UPDATE SET value = value + excluded.value",
            [bytes as i64],
        );
    }

    pub fn stats(&self) -> Result<Stats> {
        let conn = self.conn.lock();
        let (blobs, bytes): (i64, i64) =
            conn.query_row("SELECT COUNT(*), COALESCE(SUM(bytes), 0) FROM blobs", [], |r| Ok((r.get(0)?, r.get(1)?)))?;
        let returned: i64 =
            conn.query_row("SELECT value FROM meta WHERE key='returned'", [], |r| r.get(0)).optional()?.unwrap_or(0);
        Ok(Stats { blobs: blobs as u64, bytes_stored: bytes as u64, bytes_returned: returned as u64 })
    }
}
