//! `ctx_digest`: a short, faithful summary of a large output or file.
//!
//! Reading a 5,000-line log costs every model that reads it. A free OpenCode
//! model can read it instead and hand back a few lines, and the full text stays
//! one `ctx_retrieve` away if a detail matters. This runs only when the
//! project's profile allows free models; sensitive files never go; secrets are
//! masked before anything leaves; any failure falls back to the deterministic
//! head, tail and error lines the sandbox already produces.

use crate::tools::{str_arg, usize_arg, Ctx};
use anyhow::{anyhow, Result};
use serde_json::{json, Value};
use std::{path::Path, time::Duration};

/// Under this many characters there is nothing worth summarising.
const MIN_CHARS: usize = 3_000;
/// What is sent to the model: the start and the end, where logs say what matters.
const HEAD_CHARS: usize = 40_000;
const TAIL_CHARS: usize = 20_000;

pub fn definition() -> Value {
    json!({
        "name": "ctx_digest",
        "description": "A short summary of a large output, stored id or file, written by a free model when the project allows it (otherwise head, tail and error lines). The full text stays retrievable with ctx_retrieve. Use it instead of reading a long log or file yourself.",
        "inputSchema": {"type": "object", "properties": {
            "id": {"type": "string", "description": "A stored item (from a digest or an elided marker)."},
            "path": {"type": "string", "description": "A file inside the project."},
            "text": {"type": "string"},
            "focus": {"type": "string", "description": "What you need from it, e.g. 'why the build fails'."},
            "max_words": {"type": "integer", "default": 150}
        }}
    })
}

fn clip(text: &str) -> String {
    if text.len() <= HEAD_CHARS + TAIL_CHARS {
        return text.to_string();
    }
    let (mut a, mut b) = (HEAD_CHARS, text.len() - TAIL_CHARS);
    while !text.is_char_boundary(a) {
        a -= 1;
    }
    while !text.is_char_boundary(b) {
        b += 1;
    }
    format!(
        "{}\n[… {} caractères omis …]\n{}",
        &text[..a],
        b - a,
        &text[b..]
    )
}

fn prompt(focus: &str, words: usize, body: &str) -> String {
    format!(
        "Tu résumes un texte pour un autre agent de code, qui ne le lira pas.\n\
         Ce qu'il cherche : {}\n\
         Réponds en {words} mots au plus, en puces. Garde tels quels les chemins, numéros de ligne, noms et messages d'erreur. \
         N'invente rien : si ce qu'il cherche n'est pas dans le texte, dis-le.\n\n--- texte ---\n{body}",
        if focus.trim().is_empty() { "l'essentiel : erreurs, échecs, résultat final" } else { focus }
    )
}

/// The path, if it is a file inside the project root.
fn inside(root: &Path, rel: &str) -> Result<std::path::PathBuf> {
    let full = root
        .join(rel)
        .canonicalize()
        .map_err(|e| anyhow!("{rel} : {e}"))?;
    let base = root.canonicalize()?;
    if !full.starts_with(&base) {
        return Err(anyhow!("{rel} est hors du projet"));
    }
    Ok(full)
}

pub fn call(ctx: &Ctx, args: &Value) -> Result<String> {
    let focus = args.get("focus").and_then(Value::as_str).unwrap_or("");
    let words = usize_arg(args, "max_words", 150).clamp(30, 400);
    let (source, content, sensitive) = if let Some(id) = args.get("id").and_then(Value::as_str) {
        let (text, _) = ctx
            .store
            .get(id, 1, 1_000_000)?
            .ok_or_else(|| anyhow!("id inconnu : {id}"))?;
        (format!("id {id}"), text, false)
    } else if let Some(rel) = args.get("path").and_then(Value::as_str) {
        let full = inside(&ctx.root, rel)?;
        let shown = full
            .strip_prefix(ctx.root.canonicalize()?)
            .unwrap_or(&full)
            .display()
            .to_string();
        (
            shown.clone(),
            std::fs::read_to_string(&full)?,
            harnes_free::sensitive(&shown),
        )
    } else {
        ("texte".into(), str_arg(args, "text")?.to_string(), false)
    };
    if content.len() < MIN_CHARS {
        ctx.store.add_returned(content.len());
        return Ok(content);
    }
    let deterministic = || {
        crate::summary::digest(
            &ctx.store,
            &source,
            &content,
            Some(focus).filter(|f| !f.is_empty()),
        )
    };

    let cfg = harnes_free::Config::load(&ctx.root);
    if cfg.profile == harnes_free::Profile::Claude || !cfg.digest {
        return deterministic();
    }
    if sensitive {
        return Ok(format!("[harnes-ctx] {source} est un fichier sensible : il ne part pas vers un modèle gratuit.\n{}", deterministic()?));
    }
    let Some(model) = harnes_free::usable(&cfg.models).into_iter().next() else {
        return deterministic();
    };
    // Secrets are masked before anything leaves the machine.
    let (masked, _) = harnes_telemetry::redact_text(&clip(&content));
    match harnes_free::run_prompt(
        &model,
        &prompt(focus, words, &masked),
        &ctx.root,
        Duration::from_secs(90),
    ) {
        Ok(answer) if answer.chars().count() >= 20 => {
            let id = ctx.store.put(&source, &content)?;
            let lines = content.lines().count();
            ctx.store.add_returned(answer.len());
            Ok(format!(
                "[digest par {model} · {lines} lignes · détail : ctx_retrieve id={id}]\n{answer}"
            ))
        }
        // Empty, too short or failed: the deterministic digest is always right.
        _ => deterministic(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::Store;

    fn ctx(root: &Path) -> Ctx {
        Ctx {
            store: Store::in_memory().unwrap(),
            root: root.to_path_buf(),
            threads: None,
            os: harnes_os::Library::new(root.join("no-os")),
            memory: None,
        }
    }

    fn project(profile: &str) -> std::path::PathBuf {
        use std::sync::atomic::{AtomicU32, Ordering::Relaxed};
        static N: AtomicU32 = AtomicU32::new(0);
        let root = std::env::temp_dir().join(format!(
            "harnes-digest-{}-{}",
            std::process::id(),
            N.fetch_add(1, Relaxed)
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join(".harnes")).unwrap();
        std::fs::write(
            root.join(".harnes/profile.json"),
            format!(r#"{{"profile":"{profile}"}}"#),
        )
        .unwrap();
        root
    }

    fn big() -> String {
        (0..400)
            .map(|i| {
                if i == 333 {
                    "error: boom in src/a.rs:7\n".to_string()
                } else {
                    format!("line {i} of a long build log\n")
                }
            })
            .collect()
    }

    #[test]
    fn small_text_comes_back_as_it_is() {
        let root = project("mixed");
        assert_eq!(
            call(&ctx(&root), &json!({"text": "short"})).unwrap(),
            "short"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn claude_profile_never_sends_anything_out() {
        let root = project("claude");
        let out = call(&ctx(&root), &json!({"text": big()})).unwrap();
        assert!(
            out.contains("[harnes-ctx]") && out.contains("boom"),
            "{out}"
        );
        assert!(!out.contains("digest par"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn sensitive_files_and_paths_outside_the_project_are_refused() {
        let root = project("mixed");
        std::fs::write(root.join(".env"), big()).unwrap();
        let out = call(&ctx(&root), &json!({"path": ".env"})).unwrap();
        assert!(out.contains("fichier sensible"), "{out}");
        let outside =
            std::env::temp_dir().join(format!("harnes-outside-{}.txt", std::process::id()));
        std::fs::write(&outside, "x").unwrap();
        assert!(call(&ctx(&root), &json!({"path": outside.display().to_string()})).is_err());
        assert!(call(&ctx(&root), &json!({"path": "../../x"})).is_err());
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_file(outside);
    }

    /// A stand-in `opencode`: lists one free model, answers `run` with a summary.
    fn fake_opencode(dir: &Path, answer: &str) -> std::path::PathBuf {
        std::fs::create_dir_all(dir).unwrap();
        let p = dir.join(if cfg!(windows) {
            "opencode.cmd"
        } else {
            "opencode"
        });
        let body = if cfg!(windows) {
            [
                "@echo off",
                "if \"%1\"==\"models\" (",
                "echo opencode/glm-5.3-flash-free",
                ") else (",
                &format!("echo {answer}"),
                ")",
                "",
            ]
            .join("\r\n")
        } else {
            ["#!/bin/sh", &format!("if [ \"$1\" = models ]; then echo opencode/glm-5.3-flash-free; else echo '{answer}'; fi"), ""].join("\n")
        };
        std::fs::write(&p, body).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        p
    }

    #[test]
    fn a_free_model_summarises_and_the_full_text_stays_retrievable() {
        let root = project("mixed");
        std::env::set_var(
            "HARNES_OPENCODE_BIN",
            fake_opencode(
                &root.join("bin"),
                "- the build fails at src/a.rs:7 with: error: boom",
            ),
        );
        let c = ctx(&root);
        let out = call(&c, &json!({"text": big(), "focus": "why it fails"})).unwrap();
        assert!(
            out.starts_with("[digest par opencode/glm-5.3-flash-free"),
            "{out}"
        );
        assert!(out.contains("src/a.rs:7") && out.contains("ctx_retrieve id="));
        let id = out.split("id=").nth(1).unwrap().split(']').next().unwrap();
        assert!(
            c.store
                .get(id, 1, 1000)
                .unwrap()
                .unwrap()
                .0
                .contains("boom"),
            "the whole text is kept"
        );
        // A one-word answer is not trusted: the deterministic digest is returned.
        std::env::set_var(
            "HARNES_OPENCODE_BIN",
            fake_opencode(&root.join("bin2"), "ok"),
        );
        let again = call(&ctx(&root), &json!({"text": big()})).unwrap();
        assert!(!again.contains("digest par"), "{again}");
        std::env::remove_var("HARNES_OPENCODE_BIN");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn clip_keeps_the_start_and_the_end() {
        let t = format!(
            "{}{}{}",
            "a".repeat(50_000),
            "M".repeat(30_000),
            "z".repeat(50_000)
        );
        let c = clip(&t);
        assert!(
            c.starts_with("aaa") && c.ends_with("zzz") && c.contains("omis") && c.len() < 61_000
        );
        assert!(prompt("", 100, "x").contains("l'essentiel"));
    }
}
