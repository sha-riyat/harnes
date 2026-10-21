//! Brings packs into the library from a zip, a folder of zips, or folders.

use crate::Library;
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

/// Imports every pack found at `src` and returns their ids. A pack already
/// in the library is replaced. Zip entries that would escape are refused.
pub fn import(lib: &Library, src: &Path) -> Result<Vec<String>> {
    std::fs::create_dir_all(&lib.dir)?;
    let mut ids = Vec::new();
    if src.is_file() {
        ids.extend(from_zip(lib, src)?);
    } else if src.join("manifest.yaml").is_file() {
        ids.push(from_dir(lib, src)?);
    } else if src.is_dir() {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(src)?
            .flatten()
            .map(|e| e.path())
            .collect();
        entries.sort();
        for p in entries {
            if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("zip")) {
                ids.extend(from_zip(lib, &p)?);
            } else if p.join("manifest.yaml").is_file() {
                ids.push(from_dir(lib, &p)?);
            } else if p.is_dir() {
                // `name-v0.1.0/name/manifest.yaml`, as zips unpack.
                for inner in std::fs::read_dir(&p)?.flatten().map(|e| e.path()) {
                    if inner.join("manifest.yaml").is_file() {
                        ids.push(from_dir(lib, &inner)?);
                    }
                }
            }
        }
    } else {
        bail!("{} not found", src.display());
    }
    ids.sort();
    ids.dedup();
    Ok(ids)
}

fn pack_id(manifest: &str) -> Result<String> {
    #[derive(serde::Deserialize)]
    struct M {
        id: String,
    }
    let id = serde_yaml::from_str::<M>(manifest)?.id;
    if id.is_empty()
        || !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        bail!("bad pack id {id:?}");
    }
    Ok(id)
}

fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)?.flatten() {
        let (src, dst) = (e.path(), to.join(e.file_name()));
        if e.file_type()?.is_dir() {
            copy_dir(&src, &dst)?;
        } else if e.file_type()?.is_file() {
            std::fs::copy(&src, &dst)?;
        }
    }
    Ok(())
}

fn from_dir(lib: &Library, dir: &Path) -> Result<String> {
    let id = pack_id(&std::fs::read_to_string(dir.join("manifest.yaml"))?)?;
    let dst = lib.dir.join(&id);
    if dst.exists() && std::fs::canonicalize(&dst).ok() == std::fs::canonicalize(dir).ok() {
        return Ok(id);
    }
    let _ = std::fs::remove_dir_all(&dst);
    copy_dir(dir, &dst)?;
    Ok(id)
}

/// A zip holds one or more packs; each is the folder around a `manifest.yaml`.
fn from_zip(lib: &Library, path: &Path) -> Result<Vec<String>> {
    let file = std::fs::File::open(path)?;
    let mut zip =
        zip::ZipArchive::new(file).with_context(|| format!("{} is not a zip", path.display()))?;
    let tmp = lib.dir.join(format!(".import-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&tmp);
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let Some(rel) = entry.enclosed_name() else {
            bail!("unsafe path in {}", path.display())
        };
        let out = tmp.join(rel);
        if entry.is_dir() {
            std::fs::create_dir_all(&out)?;
            continue;
        }
        if let Some(parent) = out.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::io::copy(&mut entry, &mut std::fs::File::create(&out)?)?;
    }
    let found = import(lib, &tmp);
    let _ = std::fs::remove_dir_all(&tmp);
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::fixture;
    use std::io::Write;

    #[test]
    fn imports_folders_and_zips() {
        let src = fixture("src");
        let dst = Library::new(src.dir.with_extension("dst"));
        let _ = std::fs::remove_dir_all(&dst.dir);
        assert_eq!(import(&dst, &src.dir).unwrap(), vec!["builder-os"]);
        assert!(dst.dir.join("builder-os/OS.md").is_file());

        let zip_path = src.dir.with_extension("zip");
        let mut w = zip::ZipWriter::new(std::fs::File::create(&zip_path).unwrap());
        let opt = zip::write::SimpleFileOptions::default();
        w.start_file("pricing-os-v0.1.0/pricing-os/manifest.yaml", opt)
            .unwrap();
        w.write_all(b"id: pricing-os\npurpose: Price\n").unwrap();
        w.finish().unwrap();
        assert_eq!(import(&dst, &zip_path).unwrap(), vec!["pricing-os"]);
        assert_eq!(dst.list().len(), 2);
    }
}
