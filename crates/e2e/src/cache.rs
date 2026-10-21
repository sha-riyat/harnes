//! Replay cache: the concrete actions an agent found for an `act` step.
//! Stored in `.harnes/e2e/cache/<key>.json`; meant to be committed so runs
//! stay deterministic and model-free.

use crate::spec::Step;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    pub test: String,
    pub act: String,
    pub actions: Vec<Step>,
    pub recorded_at: u64,
}

/// Key = hash(test file, test name, step index, act text). Editing the act
/// text or moving the step invalidates the replay.
pub fn key(file: &str, test: &str, index: usize, act: &str) -> String {
    let h =
        Sha256::digest(format!("{}\n{test}\n{index}\n{act}", file.replace('\\', "/")).as_bytes());
    h.iter().take(8).map(|b| format!("{b:02x}")).collect()
}

fn path(root: &Path, key: &str) -> PathBuf {
    root.join(".harnes")
        .join("e2e")
        .join("cache")
        .join(format!("{key}.json"))
}

pub fn get(root: &Path, key: &str) -> Option<Resolution> {
    let raw = std::fs::read_to_string(path(root, key)).ok()?;
    serde_json::from_str(&raw).ok()
}

pub fn put(root: &Path, key: &str, res: &Resolution) -> Result<PathBuf> {
    if res.actions.is_empty() {
        bail!("aucune action enregistrée pour « {} »", res.act);
    }
    if let Some(bad) = res.actions.iter().find(|s| !s.is_primitive()) {
        bail!("action non rejouable : {bad:?}");
    }
    if res.actions.iter().any(|s| {
        matches!(s, Step::Click(l) if l.id.is_some())
            || matches!(s, Step::Fill(f) if f.target.id.is_some())
    }) {
        bail!("les locators enregistrés ne doivent pas utiliser `id` (instable) : utilise role/name/label/text/testid");
    }
    let p = path(root, key);
    std::fs::create_dir_all(p.parent().expect("cache dir"))?;
    std::fs::write(&p, serde_json::to_string_pretty(res)?)?;
    Ok(p)
}

pub fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
