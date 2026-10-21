//! Run report (`.harnes/e2e/report.json`) and per-failure trace pages.

use serde::Serialize;
use std::path::{Path, PathBuf};

pub const EXIT_PASS: i32 = 0;
pub const EXIT_FAILED: i32 = 1;
pub const EXIT_CONFIG: i32 = 2;
pub const EXIT_ENV: i32 = 3;
pub const EXIT_UNRESOLVED: i32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Passed,
    Failed,
    /// An `act` step has no recorded resolution yet (or its replay broke).
    Unresolved,
    Error,
}

#[derive(Debug, Clone, Serialize)]
pub struct Unresolved {
    pub file: String,
    pub test: String,
    pub step: usize,
    pub act: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TestResult {
    pub file: String,
    pub name: String,
    pub status: Status,
    pub millis: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub failed_step: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace: Option<String>,
    /// Number of `act` steps replayed from the cache.
    pub replayed: usize,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct Summary {
    pub total: usize,
    pub passed: usize,
    pub failed: usize,
    pub unresolved: usize,
    pub errors: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    pub status: String,
    pub exit_code: i32,
    pub summary: Summary,
    pub results: Vec<TestResult>,
    pub unresolved: Vec<Unresolved>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

impl Report {
    pub fn build(
        results: Vec<TestResult>,
        unresolved: Vec<Unresolved>,
        errors: Vec<String>,
        env_error: bool,
    ) -> Self {
        let count = |s: Status| results.iter().filter(|r| r.status == s).count();
        let summary = Summary {
            total: results.len(),
            passed: count(Status::Passed),
            failed: count(Status::Failed),
            unresolved: count(Status::Unresolved),
            errors: count(Status::Error),
        };
        let exit_code = if env_error {
            EXIT_ENV
        } else if !errors.is_empty() || summary.errors > 0 {
            EXIT_CONFIG
        } else if summary.failed > 0 {
            EXIT_FAILED
        } else if summary.unresolved > 0 {
            EXIT_UNRESOLVED
        } else {
            EXIT_PASS
        };
        let status = match exit_code {
            EXIT_PASS => "passed",
            EXIT_FAILED => "failed",
            EXIT_UNRESOLVED => "unresolved",
            EXIT_ENV => "environment-error",
            _ => "config-error",
        }
        .to_string();
        Self {
            status,
            exit_code,
            summary,
            results,
            unresolved,
            errors,
        }
    }

    pub fn describe(&self) -> String {
        let s = &self.summary;
        let mut out = format!(
            "harnes-e2e exit={} ({}) : {} test(s), {} ok, {} en échec, {} à résoudre, {} erreur(s)",
            self.exit_code, self.status, s.total, s.passed, s.failed, s.unresolved, s.errors
        );
        for r in self.results.iter().filter(|r| r.status != Status::Passed) {
            out.push_str(&format!(
                "\n- {} › {} : {}",
                r.file,
                r.name,
                r.error.as_deref().unwrap_or("?")
            ));
        }
        for e in &self.errors {
            out.push_str(&format!("\n- {e}"));
        }
        out
    }
}

pub fn dir(root: &Path) -> PathBuf {
    root.join(".harnes").join("e2e")
}

pub fn slug(s: &str) -> String {
    let mut out: String = s
        .chars()
        .map(|c| {
            if c.is_alphanumeric() {
                c.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    while out.contains("--") {
        out = out.replace("--", "-");
    }
    out.trim_matches('-').chars().take(60).collect()
}

/// Markdown page for a failed test: error, step, screen as text, screenshot.
pub fn write_trace(
    root: &Path,
    r: &TestResult,
    step_desc: &str,
    screen: &str,
    screenshot: Option<&Path>,
) -> Option<String> {
    let d = dir(root).join("results");
    std::fs::create_dir_all(&d).ok()?;
    let p = d.join(format!("{}.md", slug(&format!("{}-{}", r.file, r.name))));
    let mut md = format!(
        "# ✗ {}\n\n`{}` · {:?} · {} ms\n\n**Erreur** à l'étape {} : `{}`\n\n```text\n{}\n```\n\n## Écran au moment de l'échec\n\n```text\n{}```\n",
        r.name,
        r.file,
        r.status,
        r.millis,
        r.failed_step.map(|i| i + 1).unwrap_or(0),
        step_desc,
        r.error.as_deref().unwrap_or(""),
        screen
    );
    if let Some(s) = screenshot {
        md.push_str(&format!(
            "\n![capture]({})\n",
            s.file_name().and_then(|n| n.to_str()).unwrap_or("")
        ));
    }
    std::fs::write(&p, md).ok()?;
    Some(
        p.strip_prefix(root)
            .unwrap_or(&p)
            .display()
            .to_string()
            .replace('\\', "/"),
    )
}
