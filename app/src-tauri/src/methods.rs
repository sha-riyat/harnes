//! OS packs (method packs) in the app: list, read, import.

use crate::state::{ctx_bin, Shared};
use harnes_os::{Library, Pack};
use serde_json::{json, Value};
use tauri::State;

#[tauri::command]
pub fn os_list() -> Value {
    let lib = Library::open_default();
    json!({ "dir": lib.dir, "packs": lib.list() })
}

#[tauri::command]
pub fn os_pack(id: String) -> Result<Value, String> {
    let lib = Library::open_default();
    let pack: Pack = lib.pack(&id).map_err(|e| e.to_string())?;
    Ok(json!({
        "pack": pack,
        "os": lib.read(&id, "OS.md").unwrap_or_default(),
        "oracle": lib.read(&id, "oracle/ORACLE.md").unwrap_or_default(),
    }))
}

#[tauri::command]
pub fn os_read(id: String, path: String) -> Result<String, String> {
    Library::open_default().read(&id, &path).map_err(|e| e.to_string())
}

/// Imports packs, then refreshes every open project's HARNESS.md so agents see them.
#[tauri::command]
pub fn os_import(st: State<'_, Shared>, path: String) -> Result<Vec<String>, String> {
    let ids = harnes_os::import(&Library::open_default(), std::path::Path::new(path.trim())).map_err(|e| format!("{e:#}"))?;
    for p in st.projects.read().iter() {
        let _ = harnes_crew::setup::equip(&p.root, &ctx_bin(), false);
    }
    Ok(ids)
}
