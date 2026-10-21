//! Which CLI runs each role. Claude Code is the default; any agent CLI that
//! can take an initial prompt works. Optional `.harnes/providers.json`:
//!
//! ```json
//! { "roles": { "worker": "codex" },
//!   "providers": { "codex": { "program": "codex", "args": ["--full-auto", "{prompt_with_role}"] } } }
//! ```
//!
//! Placeholders: `{agent}`, `{settings}`, `{prompt}`, `{prompt_with_role}`.

use crate::Role;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSpec {
    pub program: String,
    pub args: Vec<String>,
    /// Supports Claude Code hooks (`--settings`) for exact lifecycle events.
    #[serde(default)]
    pub hooks: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProvidersFile {
    #[serde(default)]
    pub roles: HashMap<String, String>,
    #[serde(default)]
    pub providers: HashMap<String, ProviderSpec>,
}

pub struct Providers {
    roles: HashMap<String, String>,
    specs: HashMap<String, ProviderSpec>,
}

fn spec(program: &str, args: &[&str], hooks: bool) -> ProviderSpec {
    ProviderSpec {
        program: program.into(),
        args: args.iter().map(|s| s.to_string()).collect(),
        hooks,
    }
}

/// Best-effort defaults; override them in `.harnes/providers.json` if a CLI changes its flags.
pub fn builtin() -> HashMap<String, ProviderSpec> {
    HashMap::from([
        (
            "claude".into(),
            spec(
                "claude",
                &[
                    "--agent",
                    "{agent}",
                    "--permission-mode",
                    "auto",
                    "--settings",
                    "{settings}",
                    "{prompt}",
                ],
                true,
            ),
        ),
        (
            "codex".into(),
            spec("codex", &["--full-auto", "{prompt_with_role}"], false),
        ),
        (
            "gemini".into(),
            spec(
                "gemini",
                &["--yolo", "--prompt-interactive", "{prompt_with_role}"],
                false,
            ),
        ),
        (
            "opencode".into(),
            spec("opencode", &["--prompt", "{prompt_with_role}"], false),
        ),
    ])
}

/// A fully resolved launch command.
#[derive(Debug, Clone, PartialEq)]
pub struct Launch {
    pub provider: String,
    pub program: String,
    pub args: Vec<String>,
    pub hooks: bool,
}

impl Providers {
    pub fn load(root: &Path) -> Result<Self> {
        let path = root.join(".harnes").join("providers.json");
        let file: ProvidersFile = match std::fs::read_to_string(&path) {
            Ok(s) => {
                serde_json::from_str(&s).with_context(|| format!("{} invalide", path.display()))?
            }
            Err(_) => ProvidersFile::default(),
        };
        let mut specs = builtin();
        specs.extend(file.providers);
        Ok(Self {
            roles: file.roles,
            specs,
        })
    }

    pub fn provider_for(&self, role: Role) -> &str {
        self.roles
            .get(role.agent())
            .map(String::as_str)
            .unwrap_or("claude")
    }

    pub fn names(&self) -> Vec<String> {
        let mut v: Vec<String> = self.specs.keys().cloned().collect();
        v.sort();
        v
    }

    /// Interactive session on `provider` without a crew role (`--agent` dropped).
    pub fn launch_session(&self, provider: &str, prompt: &str, settings: &Path) -> Result<Launch> {
        let spec = self
            .specs
            .get(provider)
            .with_context(|| format!("fournisseur inconnu : {provider}"))?;
        let mut args = vec![];
        let mut it = spec.args.iter();
        while let Some(a) = it.next() {
            if a == "--agent" {
                it.next();
                continue;
            }
            args.push(
                a.replace("{settings}", &settings.display().to_string())
                    .replace("{prompt_with_role}", prompt)
                    .replace("{prompt}", prompt),
            );
        }
        Ok(Launch {
            provider: provider.into(),
            program: spec.program.clone(),
            args,
            hooks: spec.hooks,
        })
    }

    pub fn launch(&self, role: Role, prompt: &str, settings: &Path) -> Result<Launch> {
        let name = self.provider_for(role).to_string();
        let spec = self
            .specs
            .get(&name)
            .with_context(|| format!("fournisseur inconnu : {name}"))?;
        let with_role = format!(
            "Lis et applique les instructions de `.claude/agents/{}.md` (ignore l'en-tête YAML) : c'est ton rôle. {prompt}",
            role.agent()
        );
        let args = spec
            .args
            .iter()
            .map(|a| {
                a.replace("{agent}", role.agent())
                    .replace("{settings}", &settings.display().to_string())
                    .replace("{prompt_with_role}", &with_role)
                    .replace("{prompt}", prompt)
            })
            .collect();
        Ok(Launch {
            provider: name,
            program: spec.program.clone(),
            args,
            hooks: spec.hooks,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_claude_and_honours_overrides() {
        let dir = std::env::temp_dir().join(format!("harnes-prov-{}", std::process::id()));
        std::fs::create_dir_all(dir.join(".harnes")).unwrap();
        let p = Providers::load(&dir).unwrap();
        let l = p.launch(Role::Planner, "go", Path::new("s.json")).unwrap();
        assert_eq!((l.program.as_str(), l.hooks), ("claude", true));
        assert_eq!(l.args.last().unwrap(), "go");

        std::fs::write(
            dir.join(".harnes/providers.json"),
            r#"{"roles":{"worker":"codex"}}"#,
        )
        .unwrap();
        let p = Providers::load(&dir).unwrap();
        let l = p
            .launch(Role::Worker, "do L1", Path::new("s.json"))
            .unwrap();
        assert_eq!((l.provider.as_str(), l.hooks), ("codex", false));
        assert!(
            l.args.last().unwrap().contains(".claude/agents/worker.md")
                && l.args.last().unwrap().ends_with("do L1")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
