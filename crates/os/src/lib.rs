//! OS packs: method packs for agents. Each pack is a folder with a
//! `manifest.yaml`, a doctrine (`OS.md`), non-overridable judgments
//! (`oracle/ORACLE.md`), intent routing (`routing/intents.yaml`), skills,
//! agents, playbooks and evals. harnes keeps them in one library shared by
//! every project and hands the right ones to each crew role.

mod bundled;
mod import;
mod route;

pub use bundled::{install_bundled, BUNDLED};
pub use import::import;
pub use route::{for_role, role_note, Hit, ROLES};

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Pack {
    pub id: String,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub purpose: String,
    #[serde(default)]
    pub provides: Vec<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub confidence: String,
    #[serde(default, skip_deserializing)]
    pub skills: Vec<String>,
    #[serde(default, skip_deserializing)]
    pub agents: Vec<String>,
    /// Crew roles that load this pack by default.
    #[serde(default, skip_deserializing)]
    pub roles: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Intent {
    pub id: String,
    #[serde(rename = "match", default)]
    pub pattern: String,
    #[serde(default)]
    pub route: String,
}

#[derive(Debug, Deserialize)]
struct Intents {
    #[serde(default)]
    intents: Vec<Intent>,
}

/// The pack library: one folder per pack, named after its id.
#[derive(Debug, Clone)]
pub struct Library {
    pub dir: PathBuf,
}

/// `HARNES_OS_DIR`, else the app data folder (`%APPDATA%\dev.harnes.app\os`).
pub fn default_dir() -> PathBuf {
    if let Ok(d) = std::env::var("HARNES_OS_DIR") {
        return PathBuf::from(d);
    }
    let base = std::env::var("APPDATA")
        .map(PathBuf::from)
        .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".config")))
        .unwrap_or_else(|_| std::env::temp_dir());
    base.join("dev.harnes.app").join("os")
}

fn names(dir: &Path, want_dir: bool) -> Vec<String> {
    let mut v: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.path().is_dir() == want_dir)
        .map(|e| e.file_name().to_string_lossy().trim_end_matches(".md").to_string())
        .collect();
    v.sort();
    v
}

impl Library {
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn open_default() -> Self {
        Self::new(default_dir())
    }

    /// Installed packs, sorted by id. Folders without a manifest are skipped.
    pub fn list(&self) -> Vec<Pack> {
        let mut out: Vec<Pack> = names(&self.dir, true).iter().filter_map(|n| self.pack(n).ok()).collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }

    pub fn pack(&self, id: &str) -> Result<Pack> {
        let root = self.root(id)?;
        let text = std::fs::read_to_string(root.join("manifest.yaml")).map_err(|_| anyhow!("unknown pack {id}"))?;
        let mut p: Pack = serde_yaml::from_str(&text)?;
        p.skills = names(&root.join("skills"), true);
        p.agents = names(&root.join("agents"), false);
        p.roles = ROLES.iter().filter(|(_, ids)| ids.contains(&p.id.as_str())).map(|(r, _)| r.to_string()).collect();
        Ok(p)
    }

    fn root(&self, id: &str) -> Result<PathBuf> {
        if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            bail!("bad pack id {id:?}");
        }
        Ok(self.dir.join(id))
    }

    /// Reads one file of a pack. Paths stay inside the pack.
    pub fn read(&self, id: &str, rel: &str) -> Result<String> {
        let rel = Path::new(rel);
        if rel.components().any(|c| !matches!(c, Component::Normal(_))) {
            bail!("path must stay inside the pack");
        }
        let path = self.root(id)?.join(rel);
        std::fs::read_to_string(&path).map_err(|e| anyhow!("{}: {e}", rel.display()))
    }

    /// What an agent loads before acting with a pack: doctrine, then oracle.
    /// This is the packing order the packs' own Claude adapter asks for.
    pub fn brief(&self, id: &str) -> Result<String> {
        let p = self.pack(id)?;
        let os = self.read(id, "OS.md").unwrap_or_default();
        let oracle = self.read(id, "oracle/ORACLE.md").unwrap_or_default();
        let principles = self.read(id, "knowledge/first-principles.md").unwrap_or_default();
        let skills = p.skills.iter().map(|s| format!("- {s}: os_read(\"{id}\", \"skills/{s}/SKILL.md\")")).collect::<Vec<_>>().join("\n");
        Ok(format!("{os}\n\n{oracle}\n\n{principles}\n\n## Skills\n{skills}\n").trim().to_string())
    }

    pub(crate) fn intents(&self, id: &str) -> Vec<Intent> {
        self.read(id, "routing/intents.yaml")
            .ok()
            .and_then(|t| serde_yaml::from_str::<Intents>(&t).ok())
            .map(|i| i.intents)
            .unwrap_or_default()
    }

    /// Markdown index of installed packs, for the project's HARNESS.md.
    pub fn index_md(&self) -> String {
        let packs = self.list();
        if packs.is_empty() {
            return String::new();
        }
        let mut s = String::from(
            "\n## Méthodes (packs OS)\n\n\
             harnes fournit des packs de méthode via le MCP harnes-ctx. Avant une décision qui relève d'un pack, \
             charge-le avec `os_brief(id)` et respecte son oracle (règles non négociables). \
             `os_route(\"texte\")` trouve le pack et la skill adaptés ; `os_read(id, chemin)` lit une skill, un playbook ou un workflow.\n\n",
        );
        for (role, ids) in ROLES {
            let have: Vec<&str> = ids.iter().copied().filter(|i| packs.iter().any(|p| p.id == *i)).collect();
            if !have.is_empty() {
                s.push_str(&format!("- {role} : {}\n", have.join(", ")));
            }
        }
        s.push('\n');
        for p in &packs {
            s.push_str(&format!("- `{}` : {}\n", p.id, p.purpose));
        }
        s
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A tiny pack on disk for tests.
    pub fn fixture(name: &str) -> Library {
        let dir = std::env::temp_dir().join(format!("harnes-os-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let p = dir.join("builder-os");
        std::fs::create_dir_all(p.join("oracle")).unwrap();
        std::fs::create_dir_all(p.join("routing")).unwrap();
        std::fs::create_dir_all(p.join("skills/review-diff")).unwrap();
        std::fs::write(p.join("manifest.yaml"), "id: builder-os\ntitle: Builder OS\npurpose: Construct working software carefully\nprovides: [plan]\n").unwrap();
        std::fs::write(p.join("OS.md"), "# Builder OS\n").unwrap();
        std::fs::write(p.join("oracle/ORACLE.md"), "## Refuse\n- hero rewrites\n").unwrap();
        std::fs::write(p.join("routing/intents.yaml"), "os: builder-os\nintents:\n  - id: review\n    match: \"review this diff|PR review\"\n    route: skill:review-diff\n").unwrap();
        std::fs::write(p.join("skills/review-diff/SKILL.md"), "# Skill: review-diff\n").unwrap();
        Library::new(dir)
    }

    #[test]
    fn lists_reads_and_briefs() {
        let lib = fixture("list");
        let packs = lib.list();
        assert_eq!(packs.len(), 1);
        assert_eq!(packs[0].skills, vec!["review-diff"]);
        assert!(packs[0].roles.contains(&"worker".to_string()));
        let brief = lib.brief("builder-os").unwrap();
        assert!(brief.contains("hero rewrites") && brief.contains("skills/review-diff/SKILL.md"));
        assert!(lib.read("builder-os", "../x").is_err());
        assert!(lib.read("..", "OS.md").is_err());
        assert!(lib.index_md().contains("`builder-os`"));
    }
}
