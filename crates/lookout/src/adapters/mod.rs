//! One adapter per source. Each reads files the tool already writes; none
//! starts or changes anything.

mod claude;
mod codex;
mod gemini;
mod status_files;

pub use claude::ClaudeCode;
pub use codex::Codex;
pub use gemini::GeminiCli;
pub use status_files::StatusFiles;

use crate::model::Session;
use anyhow::Result;
use std::path::PathBuf;

pub trait Adapter: Send + Sync {
    fn id(&self) -> &'static str;
    /// `Ok(None)` when the tool is not installed / never used here.
    fn poll(&self) -> Result<Option<Vec<Session>>>;
}

pub fn home() -> PathBuf {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_default()
}

/// Is a process alive? Windows: real check; elsewhere: /proc or assume yes.
pub fn alive(pid: u32) -> bool {
    #[cfg(windows)]
    unsafe {
        use windows_sys::Win32::{
            Foundation::{CloseHandle, STILL_ACTIVE},
            System::Threading::{
                GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
            },
        };
        let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if h.is_null() {
            return false;
        }
        let mut code = 0u32;
        let ok = GetExitCodeProcess(h, &mut code) != 0;
        CloseHandle(h);
        ok && code == STILL_ACTIVE as u32
    }
    #[cfg(not(windows))]
    {
        let proc = std::path::Path::new("/proc");
        !proc.exists() || proc.join(pid.to_string()).exists()
    }
}

/// Last modification time in unix millis.
pub fn mtime_ms(p: &std::path::Path) -> u64 {
    p.metadata()
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Reads at most the last `max` bytes of a file as lines.
pub fn tail_lines(p: &std::path::Path, max: u64) -> Vec<String> {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut f) = std::fs::File::open(p) else {
        return vec![];
    };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let start = len.saturating_sub(max);
    if f.seek(SeekFrom::Start(start)).is_err() {
        return vec![];
    }
    let mut buf = Vec::new();
    let _ = f.read_to_end(&mut buf);
    let text = String::from_utf8_lossy(&buf);
    let mut lines: Vec<String> = text.lines().map(String::from).collect();
    if start > 0 && !lines.is_empty() {
        lines.remove(0); // partial first line
    }
    lines
}
