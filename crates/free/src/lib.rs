//! Free OpenCode models, used where a wrong answer is cheap to catch.
//!
//! The idea: a free model proposes, a paid one checks. harnes only hands a free
//! model work whose result is verified afterwards (a first attempt a Sonnet
//! reviewer reads, a reviewer among several that a verifier arbitrates, a
//! summary of an output that can be retrieved in full), and never the planner,
//! the bar, the verdicts or anything sensitive. If a free model is missing,
//! fails or goes silent, the step is simply run again on Claude.
//!
//! Using it is the user's choice: the default profile is `claude`, which never
//! sends anything to a third party. The profile is per project, in
//! `.harnes/profile.json`.

mod opencode;
mod policy;

pub use opencode::{binary, equip, live_models, live_models_with, run_prompt, usable, valid_model};
pub use policy::{sensitive, wants_free, Kind, Task};

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Free models whose provider keeps nothing and does not train on the data,
/// as OpenCode Zen's own page states. Others (Big Pickle, MiMo, Nemotron, LL
/// Alpha, Beam…) may keep or reuse what they receive: they only run if the
/// user adds them to `models` on purpose.
pub const ZERO_RETENTION: &[&str] = &[
    "opencode/glm-5.3-flash-free",
    "opencode/deepseek-v4.1-flash-free",
    "opencode/space-bunny-free",
    "opencode/longcat-2.5-preview-free",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Profile {
    /// Claude only. Nothing leaves for a third party.
    #[default]
    Claude,
    /// Free models for the deep review's finders and for the first try of
    /// small tasks; Claude checks everything they produce.
    Mixed,
    /// Also the first try of every worker whose lot is not sensitive.
    MaxEco,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub profile: Profile,
    /// When the week runs ahead of its limits, `mixed` becomes `max-eco`.
    /// It never moves a project off `claude`: that needs the user's yes.
    #[serde(default = "yes")]
    pub auto_escalate: bool,
    /// Free models allowed to receive project content (OpenCode ids).
    #[serde(default = "default_models")]
    pub models: Vec<String>,
    /// Let `ctx_digest` summarise large outputs with a free model.
    #[serde(default = "yes")]
    pub digest: bool,
}

fn yes() -> bool {
    true
}

fn default_models() -> Vec<String> {
    ZERO_RETENTION.iter().map(|m| m.to_string()).collect()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            profile: Profile::Claude,
            auto_escalate: true,
            models: default_models(),
            digest: true,
        }
    }
}

fn path(root: &Path) -> PathBuf {
    root.join(".harnes").join("profile.json")
}

impl Config {
    /// A missing or unreadable file is the safe default: Claude only.
    pub fn load(root: &Path) -> Self {
        std::fs::read_to_string(path(root))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, root: &Path) -> Result<()> {
        let p = path(root);
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::write(p, serde_json::to_string_pretty(self)? + "\n")?;
        Ok(())
    }

    /// The profile a run actually gets, given the week's pace.
    pub fn effective(&self, eco: bool) -> Profile {
        match self.profile {
            Profile::Mixed if eco && self.auto_escalate => Profile::MaxEco,
            p => p,
        }
    }

    /// Models allowed here that OpenCode really offers right now. Empty when
    /// OpenCode is missing, or the profile is `claude`: then nothing routes.
    pub fn available(&self, eco: bool) -> Vec<String> {
        if self.effective(eco) == Profile::Claude {
            return vec![];
        }
        usable(&self.models)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn claude_is_the_default_and_eco_only_escalates_mixed() {
        let root = std::env::temp_dir().join(format!("harnes-free-cfg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let c = Config::load(&root);
        assert_eq!(
            (c.profile, c.effective(true)),
            (Profile::Claude, Profile::Claude),
            "no file, nothing leaves"
        );
        assert!(c.available(true).is_empty());
        let mut m = Config {
            profile: Profile::Mixed,
            ..Config::default()
        };
        assert_eq!(
            (m.effective(false), m.effective(true)),
            (Profile::Mixed, Profile::MaxEco)
        );
        m.auto_escalate = false;
        assert_eq!(m.effective(true), Profile::Mixed);
        m.save(&root).unwrap();
        assert_eq!(Config::load(&root).profile, Profile::Mixed);
        assert!(std::fs::read_to_string(root.join(".harnes/profile.json"))
            .unwrap()
            .contains("\"mixed\""));
        std::fs::write(root.join(".harnes/profile.json"), "{ not json").unwrap();
        assert_eq!(
            Config::load(&root).profile,
            Profile::Claude,
            "a broken file is the safe default"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn only_zero_retention_models_are_allowed_by_default() {
        let c = Config::default();
        assert_eq!(c.models.len(), 4);
        assert!(c
            .models
            .iter()
            .all(|m| m.ends_with("-free") && !m.contains("big-pickle") && !m.contains("nemotron")));
    }
}
