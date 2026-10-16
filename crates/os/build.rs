// Embeds every zip in `packs/` so harnes ships its OS packs installed.

use std::{env, fs, path::PathBuf};

fn main() {
    let dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("packs");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut zips: Vec<PathBuf> = fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "zip"))
        .collect();
    zips.sort();
    let mut out = String::from("pub const BUNDLED: &[(&str, &[u8])] = &[\n");
    for z in &zips {
        let name = z.file_name().unwrap().to_string_lossy();
        let path = z.display().to_string().replace('\\', "/");
        out.push_str(&format!("    ({name:?}, include_bytes!({path:?})),\n"));
    }
    out.push_str("];\n");
    fs::write(PathBuf::from(env::var("OUT_DIR").unwrap()).join("bundled.rs"), out).unwrap();
}
