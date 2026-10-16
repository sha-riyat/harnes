//! The OS packs shipped inside harnes, installed into the library at launch.

use crate::Library;
use anyhow::Result;

include!(concat!(env!("OUT_DIR"), "/bundled.rs"));

/// Marker holding the names of the bundled zips last installed.
const MARKER: &str = ".bundled";

/// Installs the bundled packs once per harnes build. Packs the user imported
/// stay; a bundled pack replaces its older copy. Returns installed ids.
pub fn install_bundled(lib: &Library) -> Result<Vec<String>> {
    let stamp = BUNDLED.iter().map(|(n, b)| format!("{n}:{}", b.len())).collect::<Vec<_>>().join("\n");
    let marker = lib.dir.join(MARKER);
    if std::fs::read_to_string(&marker).ok().as_deref() == Some(stamp.as_str()) && !lib.list().is_empty() {
        return Ok(Vec::new());
    }
    std::fs::create_dir_all(&lib.dir)?;
    let tmp = lib.dir.join(format!(".bundle-{}", std::process::id()));
    std::fs::create_dir_all(&tmp)?;
    let mut ids = Vec::new();
    for (name, bytes) in BUNDLED {
        let zip = tmp.join(name);
        std::fs::write(&zip, bytes)?;
        ids.extend(crate::import(lib, &zip)?);
    }
    let _ = std::fs::remove_dir_all(&tmp);
    std::fs::write(marker, stamp)?;
    Ok(ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installs_every_bundled_pack_once() {
        let lib = Library::new(std::env::temp_dir().join(format!("harnes-os-bundled-{}", std::process::id())));
        let _ = std::fs::remove_dir_all(&lib.dir);
        let first = install_bundled(&lib).unwrap();
        assert_eq!(first.len(), BUNDLED.len());
        assert!(lib.list().iter().any(|p| p.id == "orchestration-os"));
        assert!(install_bundled(&lib).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&lib.dir);
    }
}
