//! Talking to the `opencode` CLI: find it, list its models, run one prompt.
//!
//! Prompts never travel on a command line: project content can hold any
//! character, and on Windows an npm shim (`opencode.cmd`) runs through
//! `cmd.exe`, where quoting cannot be trusted. The prompt goes in a file and
//! the command line carries only fixed words, a validated model id and a path.

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};
use std::{
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant},
};

/// Where `opencode` is, or None. `HARNES_OPENCODE_BIN` overrides the search.
pub fn binary() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("HARNES_OPENCODE_BIN")
        .map(PathBuf::from)
        .filter(|p| p.is_file())
    {
        return Some(p);
    }
    let names: &[&str] = if cfg!(windows) {
        &["opencode.exe", "opencode.cmd", "opencode.bat"]
    } else {
        &["opencode"]
    };
    std::env::split_paths(&std::env::var_os("PATH")?)
        .flat_map(|d| names.iter().map(move |n| d.join(n)))
        .find(|p| p.is_file())
}

/// A command for `bin`: npm shims on Windows need `cmd /C`.
fn command(bin: &Path) -> Command {
    let shim = bin
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case("cmd") || e.eq_ignore_ascii_case("bat"));
    if shim {
        let mut c = Command::new("cmd");
        c.arg("/C").arg(bin);
        c
    } else {
        Command::new(bin)
    }
}

/// Runs `cmd` to the end or kills it at `timeout`. Returns (exit code, stdout).
fn run_capped(mut cmd: Command, timeout: Duration) -> Result<(i32, String)> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // no console window
    }
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("opencode ne démarre pas")?;
    // Drain both pipes on threads: a full pipe would block the child forever.
    let drain = |mut r: Box<dyn Read + Send>| {
        std::thread::spawn(move || {
            let mut s = String::new();
            let _ = r.read_to_string(&mut s);
            s
        })
    };
    let out = drain(Box::new(
        child.stdout.take().ok_or_else(|| anyhow!("stdout"))?,
    ));
    let err = drain(Box::new(
        child.stderr.take().ok_or_else(|| anyhow!("stderr"))?,
    ));
    let deadline = Instant::now() + timeout;
    let status = loop {
        if let Some(s) = child.try_wait()? {
            break s;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            bail!("opencode : délai de {} s dépassé", timeout.as_secs());
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let stdout = out.join().unwrap_or_default();
    let stderr = err.join().unwrap_or_default();
    let code = status.code().unwrap_or(-1);
    if code != 0 && stdout.trim().is_empty() {
        bail!(
            "opencode a échoué ({code}) : {}",
            stderr.lines().last().unwrap_or("").trim()
        );
    }
    Ok((code, stdout))
}

/// Model ids in the output of `opencode models`: one per line, `provider/model`.
fn parse_models(text: &str) -> Vec<String> {
    text.lines()
        .map(str::trim)
        .filter(|l| l.starts_with("opencode/") && valid_model(l))
        .map(String::from)
        .collect()
}

pub fn valid_model(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 80
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '/' | '-'))
}

/// The models OpenCode offers right now, asked at most every ten minutes.
/// Empty when OpenCode is missing or does not answer.
pub fn live_models() -> Vec<String> {
    static CACHE: OnceLock<Mutex<Option<(Instant, Vec<String>)>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(None));
    if let Ok(g) = cache.lock() {
        if let Some((at, v)) = g.as_ref() {
            if at.elapsed() < Duration::from_secs(600) {
                return v.clone();
            }
        }
    }
    let found = binary().map(|b| live_models_with(&b)).unwrap_or_default();
    if let Ok(mut g) = cache.lock() {
        // A failed look is remembered for a minute, not ten: it may be a blip.
        *g = Some((
            if found.is_empty() {
                Instant::now() - Duration::from_secs(540)
            } else {
                Instant::now()
            },
            found.clone(),
        ));
    }
    found
}

pub fn live_models_with(bin: &Path) -> Vec<String> {
    let mut c = command(bin);
    c.args(["models", "opencode"]);
    run_capped(c, Duration::from_secs(20))
        .map(|(_, out)| parse_models(&out))
        .unwrap_or_default()
}

/// `allowed` models that OpenCode offers now, in the order of `allowed`.
pub fn usable(allowed: &[String]) -> Vec<String> {
    let live = live_models();
    allowed
        .iter()
        .filter(|m| live.contains(m))
        .cloned()
        .collect()
}

/// Asks `model` one question non-interactively and returns its answer.
pub fn run_prompt(model: &str, prompt: &str, cwd: &Path, timeout: Duration) -> Result<String> {
    if !valid_model(model) {
        bail!("identifiant de modèle invalide");
    }
    let bin = binary().ok_or_else(|| anyhow!("opencode introuvable"))?;
    static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let file = std::env::temp_dir().join(format!(
        "harnes-free-{}-{}.md",
        std::process::id(),
        N.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let shown = file.display().to_string();
    if !shown.chars().all(|c| {
        c.is_ascii_alphanumeric()
            || matches!(c, ' ' | '.' | '_' | '-' | ':' | '/' | '\\' | '(' | ')')
    }) {
        bail!("chemin temporaire non sûr pour une ligne de commande");
    }
    std::fs::write(&file, prompt)?;
    let mut c = command(&bin);
    c.current_dir(cwd).args([
        "run",
        "--model",
        model,
        "--file",
        &shown,
        "Réponds à la demande du fichier joint, rien d'autre.",
    ]);
    let res = run_capped(c, timeout);
    let _ = std::fs::remove_file(&file);
    res.map(|(_, out)| out.trim().to_string())
}

/// Declares harnes' MCP servers to OpenCode in the project's `opencode.json`,
/// keeping whatever else is there, and refuses web fetches. Returns whether the
/// file changed. A free agent then has the same threads and context tools.
pub fn equip(root: &Path, ctx_bin: &Path) -> Result<bool> {
    let path = root.join("opencode.json");
    let mut doc: Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| json!({}));
    let before = doc.clone();
    if !doc.is_object() {
        doc = json!({});
    }
    doc["$schema"] = doc
        .get("$schema")
        .cloned()
        .unwrap_or_else(|| json!("https://opencode.ai/config.json"));
    if !doc.get("mcp").is_some_and(Value::is_object) {
        doc["mcp"] = json!({});
    }
    let e2e = ctx_bin.with_file_name(if cfg!(windows) {
        "harnes-e2e.exe"
    } else {
        "harnes-e2e"
    });
    doc["mcp"]["harnes-ctx"] = json!({"type": "local", "command": [ctx_bin.display().to_string(), "--root", "."], "enabled": true});
    doc["mcp"]["harnes-e2e"] = json!({"type": "local", "command": [e2e.display().to_string(), "mcp", "--root", "."], "enabled": true});
    if !doc.get("permission").is_some_and(Value::is_object) {
        doc["permission"] = json!({});
    }
    if doc["permission"].get("webfetch").is_none() {
        doc["permission"]["webfetch"] = json!("deny");
    }
    if doc == before {
        return Ok(false);
    }
    std::fs::write(&path, serde_json::to_string_pretty(&doc)? + "\n")?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    fn make_executable(p: &Path) {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(p, std::fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[cfg(not(unix))]
    fn make_executable(_: &Path) {}

    /// A stand-in `opencode` that lists two models and answers any `run`.
    fn fake(dir: &Path) -> PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        if cfg!(windows) {
            let p = dir.join("opencode.cmd");
            std::fs::write(&p, "@echo off\r\nif \"%1\"==\"models\" (\r\necho opencode/glm-5.3-flash-free\r\necho opencode/big-pickle\r\necho anthropic/claude\r\n) else (\r\necho SUMMARY of %5\r\n)\r\n").unwrap();
            p
        } else {
            let p = dir.join("opencode");
            std::fs::write(&p, "#!/bin/sh\nif [ \"$1\" = models ]; then echo opencode/glm-5.3-flash-free; echo opencode/big-pickle; echo anthropic/claude; else echo \"SUMMARY of $5\"; fi\n").unwrap();
            make_executable(&p);
            p
        }
    }

    #[test]
    fn lists_only_opencode_models_and_validates_ids() {
        let dir = std::env::temp_dir().join(format!("harnes-fake-oc-{}", std::process::id()));
        let bin = fake(&dir);
        let models = live_models_with(&bin);
        assert_eq!(
            models,
            ["opencode/glm-5.3-flash-free", "opencode/big-pickle"]
        );
        assert!(
            valid_model("opencode/glm-5.3-flash-free")
                && !valid_model("a b")
                && !valid_model("x;rm")
                && !valid_model("")
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn runs_a_prompt_through_a_file_and_cleans_up() {
        let dir = std::env::temp_dir().join(format!("harnes-fake-oc2-{}", std::process::id()));
        let bin = fake(&dir);
        std::env::set_var("HARNES_OPENCODE_BIN", &bin);
        let out = run_prompt(
            "opencode/glm-5.3-flash-free",
            "résume & \"ça\" | tout",
            &dir,
            Duration::from_secs(30),
        )
        .unwrap();
        assert!(out.starts_with("SUMMARY"), "{out}");
        assert!(
            run_prompt("x; del *", "p", &dir, Duration::from_secs(5)).is_err(),
            "bad model id"
        );
        let left = std::fs::read_dir(std::env::temp_dir())
            .unwrap()
            .flatten()
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with(&format!("harnes-free-{}-", std::process::id()))
            })
            .count();
        assert_eq!(left, 0, "the prompt file is removed");
        std::env::remove_var("HARNES_OPENCODE_BIN");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_silent_opencode_is_killed_at_the_timeout() {
        let dir = std::env::temp_dir().join(format!("harnes-slow-oc-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let bin = if cfg!(windows) {
            let p = dir.join("opencode.cmd");
            std::fs::write(&p, "@echo off\r\nping -n 30 127.0.0.1 > nul\r\n").unwrap();
            p
        } else {
            let p = dir.join("opencode");
            std::fs::write(&p, "#!/bin/sh\nsleep 30\n").unwrap();
            make_executable(&p);
            p
        };
        let started = Instant::now();
        let mut c = command(&bin);
        c.args(["run"]);
        assert!(run_capped(c, Duration::from_secs(2)).is_err());
        assert!(started.elapsed() < Duration::from_secs(15));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn equip_merges_mcp_and_keeps_the_users_keys() {
        let dir = std::env::temp_dir().join(format!("harnes-oc-equip-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("opencode.json"),
            r#"{"theme":"dark","mcp":{"mine":{"type":"local","command":["x"]}}}"#,
        )
        .unwrap();
        assert!(equip(&dir, Path::new("C:/bin/harnes-ctx.exe")).unwrap());
        let doc: Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("opencode.json")).unwrap())
                .unwrap();
        assert_eq!(doc["theme"], "dark");
        assert!(
            doc["mcp"]["mine"].is_object() && doc["mcp"]["harnes-ctx"]["command"][1] == "--root"
        );
        assert_eq!(doc["permission"]["webfetch"], "deny");
        assert!(
            !equip(&dir, Path::new("C:/bin/harnes-ctx.exe")).unwrap(),
            "idempotent"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
