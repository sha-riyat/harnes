//! Test specs (`tests/**/*.e2e.yaml`) and project config (`harnes-e2e.yaml`).
//!
//! ```yaml
//! name: le compteur augmente
//! steps:
//!   - open: /
//!   - expect: { target: { role: status }, text: "0" }
//!   - act: clique deux fois sur Ajouter     # natural language, resolved by an agent
//!   - expect: { target: { role: status }, text: "2" }
//! ```

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Finds one element. All given fields must match. Text comparisons are
/// case-insensitive substring matches unless `exact` is set.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Locator {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placeholder: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub testid: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub css: Option<String>,
    /// Snapshot node id (`n12`). Only valid within one session; never saved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub exact: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub nth: Option<usize>,
}

impl Locator {
    pub fn describe(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expect {
    pub target: Locator,
    /// Exact text (whitespace-normalised).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contains: Option<String>,
    /// Input value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    /// `false` asserts the element is absent or hidden. Default: visible.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visible: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub count: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Fill {
    pub target: Locator,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Step {
    /// Path relative to the app URL, or an absolute URL.
    Open(String),
    Click(Locator),
    Fill(Fill),
    /// Key on the focused element: Enter, Tab, Escape, ArrowDown…
    Press(String),
    /// Milliseconds.
    Wait(u64),
    Expect(Expect),
    /// Natural-language goal. Replayed from the cache; resolved by an agent when missing.
    Act(String),
}

impl Step {
    pub fn is_primitive(&self) -> bool {
        matches!(
            self,
            Step::Open(_) | Step::Click(_) | Step::Fill(_) | Step::Press(_) | Step::Wait(_)
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TestSpec {
    pub name: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(deserialize_with = "steps_from_maps")]
    pub steps: Vec<Step>,
}

/// serde_yaml 0.9 wants `!tag` syntax for enums; route through JSON values
/// so the natural `- open: /` map form works.
fn steps_from_maps<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Step>, D::Error> {
    let raw: Vec<serde_json::Value> = Deserialize::deserialize(d)?;
    raw.into_iter()
        .enumerate()
        .map(|(i, v)| {
            serde_json::from_value(v)
                .map_err(|e| serde::de::Error::custom(format!("étape {} : {e}", i + 1)))
        })
        .collect()
}

/// Parses one step written in YAML (used by tests and tools).
pub fn parse_step(yaml: &str) -> Result<Step> {
    let v: serde_json::Value = serde_yaml::from_str(yaml)?;
    Ok(serde_json::from_value(v)?)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    pub url: String,
    /// Shell command that starts the app; skipped if the URL already answers.
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default = "default_ready")]
    pub ready_timeout_ms: u64,
}

fn default_ready() -> u64 {
    60_000
}
fn default_timeout() -> u64 {
    5_000
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub app: AppConfig,
    /// Browser executable; default: HARNES_BROWSER, Chrome, then Edge.
    #[serde(default)]
    pub browser: Option<PathBuf>,
    /// Per-step timeout for locators and expectations.
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    #[serde(default)]
    pub headed: bool,
}

pub const CONFIG_FILE: &str = "harnes-e2e.yaml";

pub fn load_config(root: &Path) -> Result<Config> {
    let path = root.join(CONFIG_FILE);
    let raw = std::fs::read_to_string(&path)
        .with_context(|| format!("{} introuvable", path.display()))?;
    serde_yaml::from_str(&raw).with_context(|| format!("config invalide : {}", path.display()))
}

pub fn load_test(path: &Path) -> Result<TestSpec> {
    let raw =
        std::fs::read_to_string(path).with_context(|| format!("lecture {}", path.display()))?;
    let spec: TestSpec = serde_yaml::from_str(&raw)
        .with_context(|| format!("spec invalide : {}", path.display()))?;
    if spec.steps.is_empty() {
        bail!("{} : aucune étape", path.display());
    }
    Ok(spec)
}

/// All `*.e2e.yaml` / `*.e2e.yml` under `tests/`, sorted.
pub fn discover(root: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(rd) = std::fs::read_dir(dir) else {
            return;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.ends_with(".e2e.yaml") || n.ends_with(".e2e.yml"))
            {
                out.push(p);
            }
        }
    }
    let mut out = Vec::new();
    walk(&root.join("tests"), &mut out);
    out.sort();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mixed_steps() {
        let s: TestSpec = serde_yaml::from_str(
            "name: t\nsteps:\n  - open: /\n  - click: { role: button, name: Ajouter }\n  - fill: { target: { label: Email }, text: a@b.c }\n  - press: Enter\n  - act: ajoute deux articles\n  - expect: { target: { role: status }, text: '2' }\n",
        )
        .unwrap();
        assert_eq!(s.steps.len(), 6);
        assert!(matches!(&s.steps[4], Step::Act(a) if a == "ajoute deux articles"));
        assert!(s.steps[1].is_primitive() && !s.steps[4].is_primitive());
    }

    #[test]
    fn rejects_unknown_locator_fields() {
        assert!(parse_step("click: { rolle: button }").is_err());
        assert!(matches!(parse_step("open: /").unwrap(), Step::Open(p) if p == "/"));
    }
}
