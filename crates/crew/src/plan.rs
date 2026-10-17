//! JSON contracts between the app and the agents.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Lot {
    pub id: String,
    pub title: String,
    #[serde(default, rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub files: Vec<String>,
    pub instructions: String,
    #[serde(default)]
    pub acceptance: String,
    #[serde(default)]
    pub commit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    #[serde(default)]
    pub context: String,
    #[serde(default)]
    pub test_command: Option<String>,
    pub lots: Vec<Lot>,
    #[serde(default)]
    pub waves: Vec<Vec<String>>,
    #[serde(default)]
    pub risks: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Report {
    pub lot: String,
    pub status: String,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub tests: String,
    #[serde(default)]
    pub notes: String,
    /// Worker's own e2e run: {"command", "exit", "passed", "failed"}.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub e2e: Option<serde_json::Value>,
    /// What became of each fix it was asked for: fixed, wrong, intentional, irrelevant.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub resolved: Vec<Resolution>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    #[serde(default)]
    pub r#where: String,
    pub outcome: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fix {
    pub lot: String,
    #[serde(default)]
    pub r#where: String,
    pub problem: String,
    #[serde(default)]
    pub fix: String,
    /// important (blocks), nit (dropped) or pre-existing (not this run's doing).
    #[serde(default = "important")]
    pub severity: String,
}

fn important() -> String {
    "important".into()
}

impl Fix {
    pub fn blocks(&self) -> bool {
        !matches!(self.severity.trim().to_lowercase().as_str(), "nit" | "pre-existing" | "preexisting")
    }
    pub fn pre_existing(&self) -> bool {
        matches!(self.severity.trim().to_lowercase().as_str(), "pre-existing" | "preexisting")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    pub lot: String,
    pub message: String,
    #[serde(default)]
    pub files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Review {
    pub verdict: String,
    #[serde(default)]
    pub lots: BTreeMap<String, String>,
    #[serde(default)]
    pub fixes: Vec<Fix>,
    #[serde(default)]
    pub commits: Vec<Commit>,
    /// Starts with the count of blocking problems; says plainly when there are none.
    #[serde(default)]
    pub summary: String,
}

/// Validation bar fixed by the Teacher before the work starts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bar {
    pub criteria: Vec<serde_json::Value>,
    #[serde(default)]
    pub threshold: serde_json::Value,
    #[serde(default)]
    pub notify_user_when: String,
    /// low, medium or high, from the paths the plan touches.
    #[serde(default)]
    pub risk: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unmet {
    pub criterion: String,
    #[serde(default)]
    pub lot: String,
    pub problem: String,
    #[serde(default)]
    pub fix: String,
}

/// Teacher's decision: show the work to the user, or send it back.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gate {
    pub ready: bool,
    #[serde(default)]
    pub met: Vec<String>,
    #[serde(default)]
    pub unmet: Vec<Unmet>,
    #[serde(default)]
    pub summary_for_user: String,
}

impl Review {
    /// PASS, or a CHANGES verdict with nothing left that blocks.
    pub fn passed(&self) -> bool {
        self.verdict.trim().eq_ignore_ascii_case("PASS") || !self.fixes.iter().any(Fix::blocks)
    }
}

impl Plan {
    /// A light run's plan: the whole task as one lot.
    pub fn single(task: &str) -> Plan {
        let lot = Lot { id: "L1".into(), title: task.chars().take(80).collect(), kind: String::new(), files: vec![], instructions: task.into(), acceptance: String::new(), commit: String::new() };
        Plan { context: String::new(), test_command: None, lots: vec![lot], waves: vec![], risks: None }
    }

    pub fn lot(&self, id: &str) -> Option<&Lot> {
        self.lots.iter().find(|l| l.id == id)
    }

    /// Waves to run. Without explicit waves, every lot runs in one wave.
    /// Unknown ids are dropped; lots missing from all waves get a final wave.
    pub fn effective_waves(&self) -> Vec<Vec<String>> {
        let mut waves: Vec<Vec<String>> = self
            .waves
            .iter()
            .map(|w| w.iter().filter(|id| self.lot(id).is_some()).cloned().collect::<Vec<_>>())
            .filter(|w| !w.is_empty())
            .collect();
        let placed: Vec<&String> = waves.iter().flatten().collect();
        let missing: Vec<String> = self.lots.iter().map(|l| l.id.clone()).filter(|id| !placed.contains(&id)).collect();
        if !missing.is_empty() {
            waves.push(missing);
        }
        waves
    }

    pub fn validate(&self) -> Result<()> {
        if self.lots.is_empty() {
            bail!("plan has no lots");
        }
        for w in self.effective_waves() {
            let mut seen = std::collections::HashSet::new();
            for id in &w {
                for f in &self.lot(id).map(|l| l.files.clone()).unwrap_or_default() {
                    if !seen.insert(f.replace('\\', "/").to_lowercase()) {
                        bail!("file {f} is assigned to two lots of the same wave");
                    }
                }
            }
        }
        Ok(())
    }
}

/// Reads a JSON file written by an agent. Tolerates a ```json fence.
pub fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<Option<T>> {
    let raw = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let trimmed = raw.trim().trim_start_matches('\u{feff}');
    let body = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .map(|s| s.trim_end().trim_end_matches("```"))
        .unwrap_or(trimmed);
    serde_json::from_str(body).map(Some).with_context(|| format!("invalid JSON in {}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(json: &str) -> Plan {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn waves_fill_in_missing_lots() {
        let p = plan(r#"{"lots":[{"id":"L1","title":"a","instructions":"x"},{"id":"L2","title":"b","instructions":"y"}],"waves":[["L1","LX"]]}"#);
        assert_eq!(p.effective_waves(), vec![vec!["L1".to_string()], vec!["L2".to_string()]]);
    }

    #[test]
    fn only_important_fixes_block() {
        let r: Review = serde_json::from_str(r#"{"verdict":"CHANGES","fixes":[{"lot":"L1","problem":"a","severity":"nit"},{"lot":"L1","problem":"b","severity":"pre-existing"}]}"#).unwrap();
        assert!(r.passed());
        let r: Review = serde_json::from_str(r#"{"verdict":"CHANGES","fixes":[{"lot":"L1","problem":"a"}]}"#).unwrap();
        assert!(!r.passed() && r.fixes[0].blocks());
    }

    #[test]
    fn rejects_shared_files_in_a_wave() {
        let p = plan(r#"{"lots":[{"id":"L1","title":"a","instructions":"x","files":["src/a.rs"]},{"id":"L2","title":"b","instructions":"y","files":["src\\A.rs"]}],"waves":[["L1","L2"]]}"#);
        assert!(p.validate().is_err());
    }
}
