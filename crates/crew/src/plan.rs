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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fix {
    pub lot: String,
    #[serde(default)]
    pub r#where: String,
    pub problem: String,
    #[serde(default)]
    pub fix: String,
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
}

/// Validation bar fixed by the Teacher before the work starts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bar {
    pub criteria: Vec<serde_json::Value>,
    #[serde(default)]
    pub threshold: serde_json::Value,
    #[serde(default)]
    pub notify_user_when: String,
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
    pub fn passed(&self) -> bool {
        self.verdict.trim().eq_ignore_ascii_case("PASS")
    }
}

impl Plan {
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

    /// The lot a fix belongs to: its own lot when it exists, else the only lot
    /// owning the file named in `where`, else the only lot of the plan.
    /// `None` when the fix cannot be attributed without guessing.
    pub fn route_fix(&self, fix: &Fix) -> Option<&str> {
        if let Some(l) = self.lot(&fix.lot) {
            return Some(l.id.as_str());
        }
        let place = fix.r#where.replace('\\', "/").to_lowercase();
        let owners: Vec<&Lot> = self
            .lots
            .iter()
            .filter(|l| l.files.iter().any(|f| {
                let f = f.replace('\\', "/").to_lowercase();
                // Whole path segments only: `a.rs` must not match `data.rs`.
                !f.is_empty() && place.match_indices(&f).any(|(i, _)| i == 0 || (!place.as_bytes()[i - 1].is_ascii_alphanumeric() && place.as_bytes()[i - 1] != b'_'))
            }))
            .collect();
        match (owners.as_slice(), self.lots.as_slice()) {
            ([one], _) => Some(one.id.as_str()),
            ([], [one]) => Some(one.id.as_str()),
            _ => None,
        }
    }

    /// Splits fix jobs into the plan's waves, so lots that may share files
    /// are never fixed at the same time.
    pub fn fix_waves<T: Clone>(&self, jobs: &[(String, T)]) -> Vec<Vec<(String, T)>> {
        self.effective_waves()
            .iter()
            .map(|w| jobs.iter().filter(|(id, _)| w.contains(id)).cloned().collect::<Vec<_>>())
            .filter(|w| !w.is_empty())
            .collect()
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
    fn rejects_shared_files_in_a_wave() {
        let p = plan(r#"{"lots":[{"id":"L1","title":"a","instructions":"x","files":["src/a.rs"]},{"id":"L2","title":"b","instructions":"y","files":["src\\A.rs"]}],"waves":[["L1","L2"]]}"#);
        assert!(p.validate().is_err());
    }

    fn fix(lot: &str, place: &str) -> Fix {
        Fix { lot: lot.into(), r#where: place.into(), problem: "p".into(), fix: String::new() }
    }

    #[test]
    fn fixes_are_routed_by_lot_then_file() {
        let p = plan(r#"{"lots":[{"id":"L1","title":"a","instructions":"x","files":["src/a.rs"]},{"id":"L2","title":"b","instructions":"y","files":["src/b.rs"]}]}"#);
        assert_eq!(p.route_fix(&fix("L2", "")), Some("L2"));
        assert_eq!(p.route_fix(&fix("LX", "src\\B.rs:12")), Some("L2"));
        assert_eq!(p.route_fix(&fix("LX", "somewhere else")), None, "no guess between two lots");
        let short = plan(r#"{"lots":[{"id":"L1","title":"a","instructions":"x","files":["a.rs"]},{"id":"L2","title":"b","instructions":"y","files":["b.rs"]}]}"#);
        assert_eq!(short.route_fix(&fix("LX", "src/data.rs:3")), None, "partial names do not match");
        assert_eq!(short.route_fix(&fix("LX", "src/a.rs:3")), Some("L1"));
        let single = plan(r#"{"lots":[{"id":"L1","title":"a","instructions":"x"}]}"#);
        assert_eq!(single.route_fix(&fix("LX", "")), Some("L1"));
    }

    #[test]
    fn fix_jobs_follow_the_waves() {
        let p = plan(r#"{"lots":[{"id":"L1","title":"a","instructions":"x"},{"id":"L2","title":"b","instructions":"y"},{"id":"L3","title":"c","instructions":"z"}],"waves":[["L1","L2"],["L3"]]}"#);
        let jobs = vec![("L3".to_string(), 3), ("L1".to_string(), 1)];
        assert_eq!(p.fix_waves(&jobs), vec![vec![("L1".to_string(), 1)], vec![("L3".to_string(), 3)]]);
    }
}
