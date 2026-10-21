//! HarnessSpec: the explicit, versioned description of what an agent gets.
//!
//! Inspired by Harness2's eight editable components. harnes already had these
//! pieces (agent prompts, guards, memory, MCP, roles, OS packs, ctx/e2e,
//! provider hooks) but they were scattered. The spec makes them inspectable,
//! budgetable and tunable.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    path::{Component, Path, PathBuf},
};

pub const VERSION: u32 = 1;

/// A component path may only point below the project root. `Path::join` alone
/// is not enough: on Windows an absolute path silently replaces the root.
fn safe_relative(value: &str) -> bool {
    if value.trim().is_empty() {
        return false;
    }
    let path = Path::new(value);
    !path.is_absolute()
        && path
            .components()
            .all(|c| matches!(c, Component::Normal(_) | Component::CurDir))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budgets {
    #[serde(default = "twelve")]
    pub always_on_tokens: u64,
    #[serde(default = "eight")]
    pub on_demand_tokens: u64,
}

fn twelve() -> u64 {
    12_000
}
fn eight() -> u64 {
    8_000
}

impl Default for Budgets {
    fn default() -> Self {
        Self {
            always_on_tokens: twelve(),
            on_demand_tokens: eight(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Components {
    pub system_prompt: String,
    pub guardrails: String,
    pub memory: String,
    pub tool_guidance: String,
    #[serde(default)]
    pub subagents: Vec<String>,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub scripts: Vec<String>,
    #[serde(default)]
    pub plugins: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HarnessSpec {
    pub version: u32,
    pub role: String,
    pub provider: String,
    pub components: Components,
    #[serde(default)]
    pub budgets: Budgets,
    #[serde(default)]
    pub provenance: String,
}

pub fn path(root: &Path) -> PathBuf {
    root.join(".harnes").join("harness.json")
}

impl HarnessSpec {
    pub fn worker(provider: &str) -> Self {
        Self {
            version: VERSION,
            role: "worker".into(),
            provider: provider.into(),
            components: Components {
                system_prompt: ".claude/agents/worker.md".into(),
                guardrails: ".harnes/guards.json".into(),
                memory: "project".into(),
                tool_guidance: "mcp/harnes-ctx".into(),
                subagents: vec!["scout".into()],
                skills: vec!["ecc-engineering-os".into(), "builder-os".into()],
                scripts: vec!["ctx_execute".into(), "e2e".into()],
                plugins: vec!["hooks".into()],
            },
            budgets: Budgets::default(),
            provenance: "harnes default HarnessSpec v1".into(),
        }
    }

    pub fn ensure(root: &Path, provider: &str) -> Result<Self> {
        let p = path(root);
        if p.exists() {
            return Self::load(root);
        }
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d)?;
        }
        let spec = Self::worker(provider);
        std::fs::write(&p, serde_json::to_string_pretty(&spec)? + "\n")?;
        Ok(spec)
    }

    pub fn load(root: &Path) -> Result<Self> {
        let p = path(root);
        let text =
            std::fs::read_to_string(&p).with_context(|| format!("{} absent", p.display()))?;
        let spec: Self =
            serde_json::from_str(&text).with_context(|| format!("{} invalide", p.display()))?;
        spec.validate(root)?;
        Ok(spec)
    }

    pub fn save(&self, root: &Path) -> Result<()> {
        self.validate(root)?;
        let p = path(root);
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::write(&p, serde_json::to_string_pretty(self)? + "\n")?;
        Ok(())
    }

    pub fn validate(&self, root: &Path) -> Result<()> {
        if self.version != VERSION {
            bail!(
                "HarnessSpec version {} non supportée (attendu {VERSION})",
                self.version
            );
        }
        if self.role.trim().is_empty() || self.provider.trim().is_empty() {
            bail!("HarnessSpec: role et provider requis");
        }
        if self.budgets.always_on_tokens == 0 || self.budgets.on_demand_tokens == 0 {
            bail!("HarnessSpec: budgets doivent être > 0");
        }
        // Referenced files must stay inside the project, when they exist.
        for rel in [&self.components.system_prompt, &self.components.guardrails] {
            if !safe_relative(rel) {
                bail!("HarnessSpec: chemin hors projet ({rel})");
            }
            if !root.join(rel).exists() && rel != &self.components.guardrails {
                // A missing prompt is a real problem; a missing guardrail file
                // is normal: the defaults apply.
                bail!("HarnessSpec: fichier introuvable ({rel})");
            }
        }
        Ok(())
    }

    /// Components as a flat map for the UI.
    pub fn flat(&self) -> BTreeMap<String, serde_json::Value> {
        let mut m = BTreeMap::new();
        m.insert(
            "system_prompt".into(),
            serde_json::json!(self.components.system_prompt),
        );
        m.insert(
            "guardrails".into(),
            serde_json::json!(self.components.guardrails),
        );
        m.insert("memory".into(), serde_json::json!(self.components.memory));
        m.insert(
            "tool_guidance".into(),
            serde_json::json!(self.components.tool_guidance),
        );
        m.insert(
            "subagents".into(),
            serde_json::json!(self.components.subagents),
        );
        m.insert("skills".into(), serde_json::json!(self.components.skills));
        m.insert("scripts".into(), serde_json::json!(self.components.scripts));
        m.insert("plugins".into(), serde_json::json!(self.components.plugins));
        m
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_loads_and_rejects_escape() {
        let root = std::env::temp_dir().join(format!("harnes-spec-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".claude/agents")).unwrap();
        std::fs::write(root.join(".claude/agents/worker.md"), "worker").unwrap();
        let spec = HarnessSpec::ensure(&root, "claude").unwrap();
        assert_eq!(HarnessSpec::load(&root).unwrap().role, "worker");
        let mut bad = spec.clone();
        bad.components.system_prompt = "../escape.md".into();
        assert!(bad.validate(&root).is_err());
        let mut absolute = spec.clone();
        absolute.components.system_prompt =
            root.join(".claude/agents/worker.md").display().to_string();
        assert!(absolute.validate(&root).is_err());
        assert_eq!(spec.flat().len(), 8);
        let _ = std::fs::remove_dir_all(&root);
    }
}
