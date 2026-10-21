//! Project memory with a review gate.
//!
//! Agents and the crew produce *candidates* backed by events. A human (or an
//! explicit policy) approves, edits or rejects them. Only approved records are
//! returned by `memory_search` / `memory_context`. Nothing is promoted silently,
//! and every record keeps its provenance.

use anyhow::{bail, Context, Result};
use parking_lot::Mutex;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::Path};

pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub id: i64,
    pub project: String,
    pub run: String,
    /// workflow | correction | debugging_pattern | gotcha | convention
    pub kind: String,
    pub title: String,
    pub body: String,
    /// When this applies, in the agent's own words.
    pub applicability: String,
    pub tags: Vec<String>,
    pub evidence: Vec<String>,
    pub source: String,
    /// candidate | approved | rejected | superseded
    pub status: String,
    pub confidence: f64,
    pub created_at: u64,
    pub reviewed_at: Option<u64>,
    pub review_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Record {
    pub id: i64,
    pub candidate_id: Option<i64>,
    pub project: String,
    pub kind: String,
    pub title: String,
    pub body: String,
    pub applicability: String,
    pub tags: Vec<String>,
    pub evidence: Vec<String>,
    pub provenance: String,
    pub created_at: u64,
    pub superseded_by: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct MemoryStats {
    pub candidates: u64,
    pub pending: u64,
    pub approved: u64,
    pub rejected: u64,
    pub records: u64,
}

pub struct Memory {
    project: String,
    conn: Mutex<Connection>,
}

fn tags_json(tags: &[String]) -> String {
    serde_json::to_string(tags).unwrap_or_else(|_| "[]".into())
}

fn arr(s: &str) -> Vec<String> {
    serde_json::from_str(s).unwrap_or_default()
}

impl Memory {
    pub fn open(root: &Path, project: &str) -> Result<Self> {
        let path = root.join(".harnes").join("memory.db");
        if let Some(d) = path.parent() {
            std::fs::create_dir_all(d)?;
        }
        let conn = Connection::open(&path).with_context(|| format!("open {}", path.display()))?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA busy_timeout=5000;
             CREATE TABLE IF NOT EXISTS candidates(
               id INTEGER PRIMARY KEY AUTOINCREMENT, project TEXT NOT NULL, run TEXT NOT NULL,
               kind TEXT NOT NULL, title TEXT NOT NULL, body TEXT NOT NULL, applicability TEXT NOT NULL DEFAULT '',
               tags TEXT NOT NULL DEFAULT '[]', evidence TEXT NOT NULL DEFAULT '[]', source TEXT NOT NULL DEFAULT '',
               status TEXT NOT NULL DEFAULT 'candidate', confidence REAL NOT NULL DEFAULT 0.5,
               created_at INTEGER NOT NULL, reviewed_at INTEGER, review_reason TEXT);
             CREATE INDEX IF NOT EXISTS candidates_status ON candidates(status, created_at);
             CREATE TABLE IF NOT EXISTS records(
               id INTEGER PRIMARY KEY AUTOINCREMENT, candidate_id INTEGER, project TEXT NOT NULL,
               kind TEXT NOT NULL, title TEXT NOT NULL, body TEXT NOT NULL, applicability TEXT NOT NULL DEFAULT '',
               tags TEXT NOT NULL DEFAULT '[]', evidence TEXT NOT NULL DEFAULT '[]', provenance TEXT NOT NULL DEFAULT '',
               created_at INTEGER NOT NULL, superseded_by INTEGER);
             CREATE INDEX IF NOT EXISTS records_project ON records(project, created_at);",
        )?;
        Ok(Self {
            project: project.into(),
            conn: Mutex::new(conn),
        })
    }

    pub fn in_memory(project: &str) -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let m = Self {
            project: project.into(),
            conn: Mutex::new(conn),
        };
        // Reuse the same schema without touching disk.
        m.conn.lock().execute_batch(
            "CREATE TABLE candidates(id INTEGER PRIMARY KEY AUTOINCREMENT, project TEXT NOT NULL, run TEXT NOT NULL, kind TEXT NOT NULL,
               title TEXT NOT NULL, body TEXT NOT NULL, applicability TEXT NOT NULL DEFAULT '', tags TEXT NOT NULL DEFAULT '[]',
               evidence TEXT NOT NULL DEFAULT '[]', source TEXT NOT NULL DEFAULT '', status TEXT NOT NULL DEFAULT 'candidate',
               confidence REAL NOT NULL DEFAULT 0.5, created_at INTEGER NOT NULL, reviewed_at INTEGER, review_reason TEXT);
             CREATE TABLE records(id INTEGER PRIMARY KEY AUTOINCREMENT, candidate_id INTEGER, project TEXT NOT NULL, kind TEXT NOT NULL,
               title TEXT NOT NULL, body TEXT NOT NULL, applicability TEXT NOT NULL DEFAULT '', tags TEXT NOT NULL DEFAULT '[]',
               evidence TEXT NOT NULL DEFAULT '[]', provenance TEXT NOT NULL DEFAULT '', created_at INTEGER NOT NULL, superseded_by INTEGER);",
        )?;
        Ok(m)
    }

    pub fn project(&self) -> &str {
        &self.project
    }

    #[allow(clippy::too_many_arguments)]
    pub fn create_candidate(
        &self,
        run: &str,
        kind: &str,
        title: &str,
        body: &str,
        applicability: &str,
        tags: &[String],
        evidence: &[String],
        source: &str,
        confidence: f64,
    ) -> Result<Candidate> {
        if title.trim().is_empty() || body.trim().is_empty() {
            bail!("candidat mémoire vide");
        }
        let at = now_ms();
        let c = self.conn.lock();
        c.execute(
            "INSERT INTO candidates(project, run, kind, title, body, applicability, tags, evidence, source, confidence, created_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![self.project, run, kind, title.trim(), body.trim(), applicability, tags_json(tags), serde_json::to_string(evidence)?, source, confidence.clamp(0.0, 1.0), at as i64],
        )?;
        let id = c.last_insert_rowid();
        drop(c);
        self.candidate(id)?.context("candidat disparu")
    }

    pub fn candidate(&self, id: i64) -> Result<Option<Candidate>> {
        let c = self.conn.lock();
        Ok(c.query_row(
            "SELECT id, project, run, kind, title, body, applicability, tags, evidence, source, status, confidence, created_at, reviewed_at, review_reason
             FROM candidates WHERE id = ?1",
            [id],
            row_candidate,
        )
        .optional()?)
    }

    pub fn candidates(
        &self,
        status: &str,
        run: Option<&str>,
        limit: usize,
    ) -> Result<Vec<Candidate>> {
        let c = self.conn.lock();
        let mut st = c.prepare(
            "SELECT id, project, run, kind, title, body, applicability, tags, evidence, source, status, confidence, created_at, reviewed_at, review_reason
             FROM candidates WHERE (?1 = 'all' OR status = ?1) AND (?2 IS NULL OR run = ?2)
             ORDER BY created_at DESC, id DESC LIMIT ?3",
        )?;
        let rows = st.query_map(params![status, run, limit as i64], row_candidate)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Approves or rejects a candidate. `edit` optionally replaces title/body.
    pub fn review(
        &self,
        id: i64,
        decision: &str,
        reason: &str,
        edit: Option<(String, String)>,
    ) -> Result<Candidate> {
        let decision = decision.to_lowercase();
        if decision != "approve" && decision != "reject" {
            bail!("décision attendue : approve | reject");
        }
        let mut cand = self.candidate(id)?.context("candidat inconnu")?;
        if cand.status != "candidate" {
            bail!("candidat déjà revu ({})", cand.status);
        }
        if let Some((title, body)) = edit {
            if !title.trim().is_empty() {
                cand.title = title;
            }
            if !body.trim().is_empty() {
                cand.body = body;
            }
        }
        let at = now_ms();
        let mut c = self.conn.lock();
        let tx = c.transaction()?;
        let status = if decision == "approve" {
            "approved"
        } else {
            "rejected"
        };
        let changed = tx.execute(
            "UPDATE candidates SET status=?2, reviewed_at=?3, review_reason=?4, title=?5, body=?6
             WHERE id=?1 AND status='candidate'",
            params![id, status, at as i64, reason, cand.title, cand.body],
        )?;
        if changed == 0 {
            bail!("candidat déjà revu");
        }
        if decision == "approve" {
            tx.execute(
                "INSERT INTO records(candidate_id, project, kind, title, body, applicability, tags, evidence, provenance, created_at)
                 VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                params![
                    id, self.project, cand.kind, cand.title, cand.body, cand.applicability, tags_json(&cand.tags),
                    serde_json::to_string(&cand.evidence)?, format!("run:{} source:{}", cand.run, cand.source), at as i64
                ],
            )?;
        }
        tx.commit()?;
        drop(c);
        self.candidate(id)?.context("candidat disparu")
    }

    pub fn records(&self, limit: usize) -> Result<Vec<Record>> {
        let c = self.conn.lock();
        let mut st = c.prepare(
            "SELECT id, candidate_id, project, kind, title, body, applicability, tags, evidence, provenance, created_at, superseded_by
             FROM records WHERE superseded_by IS NULL ORDER BY created_at DESC LIMIT ?1",
        )?;
        let rows = st.query_map([limit as i64], row_record)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    pub fn record(&self, id: i64) -> Result<Option<Record>> {
        let c = self.conn.lock();
        Ok(c.query_row(
            "SELECT id, candidate_id, project, kind, title, body, applicability, tags, evidence, provenance, created_at, superseded_by FROM records WHERE id = ?1",
            [id],
            row_record,
        )
        .optional()?)
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<Record>> {
        let terms: Vec<String> = query
            .split_whitespace()
            .filter(|t| t.len() > 1)
            .map(|t| format!("%{}%", t.to_lowercase()))
            .collect();
        if terms.is_empty() {
            return self.records(limit);
        }
        let mut sql = String::from(
            "SELECT id, candidate_id, project, kind, title, body, applicability, tags, evidence, provenance, created_at, superseded_by
             FROM records WHERE superseded_by IS NULL AND (",
        );
        for (i, _) in terms.iter().enumerate() {
            if i > 0 {
                sql.push_str(" OR ");
            }
            sql.push_str(&format!("(lower(title) LIKE ?{n} OR lower(body) LIKE ?{n} OR lower(tags) LIKE ?{n} OR lower(applicability) LIKE ?{n})", n = i + 1));
        }
        sql.push_str(") ORDER BY created_at DESC LIMIT ");
        sql.push_str(&(limit.clamp(1, 100) as i64).to_string());
        let c = self.conn.lock();
        let mut st = c.prepare(&sql)?;
        let rows = st.query_map(rusqlite::params_from_iter(terms.iter()), row_record)?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Compact block suitable for a planner or worker prompt.
    pub fn context(&self, query: &str, limit: usize) -> Result<String> {
        let records = self.search(query, limit)?;
        if records.is_empty() {
            return Ok(String::new());
        }
        let mut s = String::from("## Mémoire approuvée du projet\n\n");
        for r in records {
            s.push_str(&format!("### {} ({})\n{}\n", r.title, r.kind, r.body));
            if !r.applicability.trim().is_empty() {
                s.push_str(&format!("Quand : {}\n", r.applicability));
            }
            s.push_str(&format!("Provenance : {}\n\n", r.provenance));
        }
        Ok(s)
    }

    pub fn stats(&self) -> Result<MemoryStats> {
        let c = self.conn.lock();
        let count =
            |sql: &str| -> Result<u64> { Ok(c.query_row(sql, [], |r| r.get::<_, i64>(0))? as u64) };
        Ok(MemoryStats {
            candidates: count("SELECT COUNT(*) FROM candidates")?,
            pending: count("SELECT COUNT(*) FROM candidates WHERE status='candidate'")?,
            approved: count("SELECT COUNT(*) FROM candidates WHERE status='approved'")?,
            rejected: count("SELECT COUNT(*) FROM candidates WHERE status='rejected'")?,
            records: count("SELECT COUNT(*) FROM records WHERE superseded_by IS NULL")?,
        })
    }

    /// Harvests a finished run: review fixes + worker resolutions become one
    /// candidate. No fixes means no lesson, which keeps the queue quiet.
    pub fn harvest_run(
        &self,
        root: &Path,
        run: &str,
        telemetry: Option<&harnes_telemetry::Telemetry>,
    ) -> Result<Option<Candidate>> {
        let dir = root.join(".harnes").join("runs").join(run);
        let mut reviews: Vec<(u32, serde_json::Value)> = Vec::new();
        for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(n) = name
                .strip_prefix("review-")
                .and_then(|s| s.strip_suffix(".json"))
                .and_then(|s| s.parse::<u32>().ok())
            {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(
                    &std::fs::read_to_string(entry.path()).unwrap_or_default(),
                ) {
                    reviews.push((n, v));
                }
            }
        }
        reviews.sort_by_key(|(n, _)| *n);
        let Some((_, last)) = reviews.last() else {
            return Ok(None);
        };
        let fixes = last["fixes"].as_array().cloned().unwrap_or_default();
        if fixes.is_empty() {
            return Ok(None);
        }
        let mut resolutions: Vec<String> = Vec::new();
        for entry in std::fs::read_dir(dir.join("reports"))
            .into_iter()
            .flatten()
            .flatten()
        {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(
                &std::fs::read_to_string(entry.path()).unwrap_or_default(),
            ) {
                for r in v["resolved"].as_array().into_iter().flatten() {
                    let w = r["where"].as_str().unwrap_or("");
                    let o = r["outcome"].as_str().unwrap_or("");
                    if !w.is_empty() && !o.is_empty() {
                        resolutions.push(format!("{w} -> {o}"));
                    }
                }
            }
        }
        let mut body = String::from("Points de vigilance issus de ce run :\n");
        for f in &fixes {
            let w = f["where"].as_str().unwrap_or("?");
            let p = f["problem"].as_str().unwrap_or("");
            let fix = f["fix"].as_str().unwrap_or("");
            body.push_str(&format!("- `{w}` : {p} → {fix}\n"));
        }
        if !resolutions.is_empty() {
            body.push_str("\nRésolutions vérifiées :\n");
            for r in &resolutions {
                body.push_str(&format!("- {r}\n"));
            }
        }
        let evidence = telemetry
            .and_then(|t| t.recent(run, 500).ok())
            .unwrap_or_default()
            .into_iter()
            .filter(|e| e.kind.starts_with("review.") || e.kind.starts_with("gate."))
            .map(|e| e.id)
            .collect::<Vec<_>>();
        let tags = fixes
            .iter()
            .filter_map(|f| f["lot"].as_str().map(String::from))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        self.create_candidate(
            run,
            "correction",
            &format!("Leçon de revue — run {run}"),
            body.trim(),
            "Avant de reproduire un lot similaire ou de relire un diff qui touche ces fichiers.",
            &tags,
            &evidence,
            "review",
            0.65,
        )
        .map(Some)
    }
}

fn row_candidate(r: &rusqlite::Row) -> rusqlite::Result<Candidate> {
    Ok(Candidate {
        id: r.get(0)?,
        project: r.get(1)?,
        run: r.get(2)?,
        kind: r.get(3)?,
        title: r.get(4)?,
        body: r.get(5)?,
        applicability: r.get(6)?,
        tags: arr(&r.get::<_, String>(7)?),
        evidence: arr(&r.get::<_, String>(8)?),
        source: r.get(9)?,
        status: r.get(10)?,
        confidence: r.get(11)?,
        created_at: r.get::<_, i64>(12)? as u64,
        reviewed_at: r.get::<_, Option<i64>>(13)?.map(|v| v as u64),
        review_reason: r.get(14)?,
    })
}

fn row_record(r: &rusqlite::Row) -> rusqlite::Result<Record> {
    Ok(Record {
        id: r.get(0)?,
        candidate_id: r.get(1)?,
        project: r.get(2)?,
        kind: r.get(3)?,
        title: r.get(4)?,
        body: r.get(5)?,
        applicability: r.get(6)?,
        tags: arr(&r.get::<_, String>(7)?),
        evidence: arr(&r.get::<_, String>(8)?),
        provenance: r.get(9)?,
        created_at: r.get::<_, i64>(10)? as u64,
        superseded_by: r.get(11)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn review_gate_promotes_only_approved_records() {
        let m = Memory::in_memory("p").unwrap();
        let c = m
            .create_candidate(
                "r1",
                "gotcha",
                "Windows paths",
                "Use forward slashes in prompts.",
                "Any Windows task",
                &["windows".into()],
                &["evt-1".into()],
                "test",
                0.8,
            )
            .unwrap();
        assert!(m.search("forward", 10).unwrap().is_empty());
        let approved = m.review(c.id, "approve", "utile", None).unwrap();
        assert_eq!(approved.status, "approved");
        let hits = m.search("windows", 10).unwrap();
        assert_eq!(hits.len(), 1);
        assert!(m.context("slashes", 5).unwrap().contains("forward slashes"));
        assert_eq!(m.stats().unwrap().records, 1);
        let c2 = m
            .create_candidate(
                "r1",
                "gotcha",
                "Noise",
                "Ignore me",
                "",
                &[],
                &[],
                "test",
                0.2,
            )
            .unwrap();
        m.review(c2.id, "reject", "bruit", None).unwrap();
        assert!(m.search("noise", 10).unwrap().is_empty());
    }

    #[test]
    fn harvests_review_fixes() {
        let root =
            std::env::temp_dir().join(format!("harnes-memory-harvest-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let dir = root.join(".harnes/runs/7");
        std::fs::create_dir_all(dir.join("reports")).unwrap();
        std::fs::write(
            dir.join("review-0.json"),
            r#"{"verdict":"CHANGES","fixes":[{"lot":"L1","where":"a.rs:2","problem":"panic","fix":"guard None"}]}"#,
        )
        .unwrap();
        std::fs::write(
            dir.join("reports/L1-r1.json"),
            r#"{"lot":"L1","resolved":[{"where":"a.rs:2","outcome":"fixed"}]}"#,
        )
        .unwrap();
        let m = Memory::in_memory("p").unwrap();
        let c = m.harvest_run(&root, "7", None).unwrap().unwrap();
        assert!(c.body.contains("guard None") && c.body.contains("fixed"));
        assert_eq!(c.status, "candidate");
        let _ = std::fs::remove_dir_all(&root);
    }
}
