//! Git worktrees per crew lot.
//!
//! A lot runs in its own worktree and branch. When the worker is done, harnes
//! commits whatever is left uncommitted and merges the branch back; conflicts
//! leave the worktree in place for a human. This is deliberately conservative:
//! harnes never rewrites history and never force-removes dirty work.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorktreeConfig {
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default = "head")]
    pub base: String,
}

fn yes() -> bool {
    true
}
fn head() -> String {
    "HEAD".into()
}
impl Default for WorktreeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            base: "HEAD".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Worktree {
    pub lot: String,
    pub branch: String,
    pub path: PathBuf,
    pub base: String,
}

#[derive(Debug, Clone)]
pub struct Worktrees {
    pub root: PathBuf,
    pub dir: PathBuf,
    pub config: WorktreeConfig,
}

fn git(root: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .with_context(|| format!("git {} (git installé ?)", args.join(" ")))?;
    let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr).trim().to_string();
        bail!(
            "git {} : {}",
            args.join(" "),
            if err.is_empty() { text } else { err }
        );
    }
    Ok(text)
}

fn clean_part(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect()
}

fn branch_name(run: &str, lot: &str) -> String {
    format!("harnes/{}/{}", clean_part(run), clean_part(lot))
}

impl Worktrees {
    pub fn load(root: &Path) -> Self {
        let path = root.join(".harnes").join("worktrees.json");
        let config = match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|_| WorktreeConfig {
                // A malformed policy must not silently opt the project into
                // isolation. Missing file keeps the default (enabled).
                enabled: false,
                base: head(),
            }),
            Err(_) => WorktreeConfig::default(),
        };
        Self {
            root: root.to_path_buf(),
            dir: root.join(".harnes").join("worktrees"),
            config,
        }
    }

    pub fn is_git(&self) -> bool {
        // Worktrees are rooted at the repository top level. Running them for a
        // project opened in a subdirectory would hand the agent the whole repo
        // and a cwd different from the project, so those projects stay on the
        // plain checkout instead.
        let Ok(top) = git(&self.root, &["rev-parse", "--show-toplevel"]) else {
            return false;
        };
        let (Ok(root), Ok(top)) = (
            std::fs::canonicalize(&self.root),
            std::fs::canonicalize(PathBuf::from(top)),
        ) else {
            return false;
        };
        root == top
    }

    pub fn active(&self) -> bool {
        self.config.enabled && self.is_git()
    }

    pub fn path_for(&self, run: &str, lot: &str) -> PathBuf {
        self.dir.join(clean_part(run)).join(clean_part(lot))
    }

    /// Creates a worktree from `base`. If it already exists (resume), reuse it.
    pub fn create(&self, run: &str, lot: &str) -> Result<Worktree> {
        if run.trim().is_empty() || lot.trim().is_empty() {
            bail!("worktree : run et lot requis");
        }
        if !self.active() {
            bail!("worktrees désactivés ou projet non-git");
        }
        let path = self.path_for(run, lot);
        let branch = branch_name(run, lot);
        let wt = Worktree {
            lot: lot.into(),
            branch: branch.clone(),
            path: path.clone(),
            base: self.config.base.clone(),
        };
        if path.exists() {
            return Ok(wt);
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // Reuse the branch after an interrupted run instead of failing on `-b`.
        let has_branch = git(&self.root, &["rev-parse", "--verify", &branch]).is_ok();
        let sp = path.to_string_lossy().to_string();
        if has_branch {
            git(&self.root, &["worktree", "add", &sp, &branch])?;
        } else {
            git(
                &self.root,
                &["worktree", "add", "-b", &branch, &sp, &self.config.base],
            )?;
        }
        Ok(wt)
    }

    /// Prepares the worktree for an agent: the project's `.claude` files and an
    /// `.mcp.json` whose `--root` points at the project (threads, OS packs, e2e)
    /// instead of the worktree. The agent edits the worktree; its MCP data stays
    /// in the project.
    pub fn prepare_for_agent(&self, wt: &Worktree) -> Result<()> {
        // Only inject harness configuration that the base revision does not
        // already have. This keeps a tracked `.claude`/`CLAUDE.md` untouched and
        // lets `finish` remove exactly the files harnes generated.
        let src = self.root.join(".claude");
        let dst = wt.path.join(".claude");
        if src.is_dir() && !dst.exists() {
            copy_dir(&src, &dst)?;
        }
        let claude = self.root.join("CLAUDE.md");
        if claude.exists() && !wt.path.join("CLAUDE.md").exists() {
            std::fs::copy(&claude, wt.path.join("CLAUDE.md"))?;
        }
        if let Ok(text) = std::fs::read_to_string(self.root.join(".mcp.json")) {
            if let Ok(mut doc) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(servers) = doc.get_mut("mcpServers").and_then(|v| v.as_object_mut()) {
                    for server in servers.values_mut() {
                        if let Some(args) = server.get_mut("args").and_then(|v| v.as_array_mut()) {
                            for i in 0..args.len() {
                                if args[i].as_str() == Some(".") {
                                    args[i] =
                                        serde_json::Value::String(self.root.display().to_string());
                                }
                            }
                        }
                    }
                }
                std::fs::write(
                    wt.path.join(".mcp.json"),
                    serde_json::to_string_pretty(&doc)? + "\n",
                )?;
            }
        }
        Ok(())
    }

    pub fn status(&self, wt: &Worktree) -> Result<String> {
        git(&wt.path, &["status", "--porcelain"])
    }

    pub fn changed_files(&self, wt: &Worktree) -> Result<Vec<String>> {
        let tracked = git(&wt.path, &["diff", "--name-only", "HEAD"])?;
        let untracked = git(&wt.path, &["ls-files", "--others", "--exclude-standard"])?;
        let mut out: Vec<String> = tracked
            .lines()
            .chain(untracked.lines())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        out.sort();
        out.dedup();
        Ok(out)
    }

    /// Commits everything left by the agent (it was told not to commit).
    pub fn commit_all(&self, wt: &Worktree, message: &str) -> Result<bool> {
        if self.status(wt)?.trim().is_empty() {
            return Ok(false);
        }
        git(&wt.path, &["add", "-A"])?;
        git(&wt.path, &["commit", "-m", message])?;
        Ok(true)
    }

    pub fn diff_stat(&self, wt: &Worktree, base: &str) -> Result<String> {
        git(&wt.path, &["diff", "--stat", &format!("{base}...HEAD")])
    }

    /// Removes harness files that `prepare_for_agent` injected into a clean
    /// worktree, and restores a tracked `.mcp.json`. They are not part of the
    /// lot's change and must never be merged accidentally.
    fn clean_generated(&self, wt: &Worktree) {
        let tracked = |rel: &str| {
            git(&wt.path, &["ls-files", "--", rel])
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false)
        };
        let mcp = wt.path.join(".mcp.json");
        if tracked(".mcp.json") {
            let _ = git(&wt.path, &["checkout", "--", ".mcp.json"]);
        } else if mcp.exists() {
            let _ = std::fs::remove_file(mcp);
        }
        let claude_md = wt.path.join("CLAUDE.md");
        if claude_md.exists() && !tracked("CLAUDE.md") {
            let _ = std::fs::remove_file(claude_md);
        }
        let claude_dir = wt.path.join(".claude");
        if claude_dir.is_dir() && !tracked(".claude") {
            let _ = std::fs::remove_dir_all(claude_dir);
        }
    }

    /// Commits then merges into the project checkout. On conflict the merge is
    /// aborted and the worktree is kept; nothing is discarded.
    pub fn finish(&self, wt: &Worktree, message: &str) -> Result<()> {
        if git(&self.root, &["rev-parse", "-q", "--verify", "MERGE_HEAD"]).is_ok() {
            bail!(
                "merge déjà en cours dans le projet : termine-le avant de fusionner {}",
                wt.branch
            );
        }
        self.clean_generated(wt);
        self.commit_all(wt, message)?;
        let merge = git(&self.root, &["merge", "--no-ff", &wt.branch, "-m", message]);
        if let Err(e) = merge {
            let _ = git(&self.root, &["merge", "--abort"]);
            bail!(
                "merge de {} impossible (worktree conservé dans {}) : {e:#}",
                wt.branch,
                wt.path.display()
            );
        }
        let sp = wt.path.to_string_lossy().to_string();
        let _ = git(&self.root, &["worktree", "remove", "--force", &sp]);
        Ok(())
    }

    pub fn remove(&self, run: &str, lot: &str, force: bool) -> Result<()> {
        if run.trim().is_empty() || lot.trim().is_empty() {
            bail!("worktree : run et lot requis");
        }
        let path = self.path_for(run, lot);
        let branch = branch_name(run, lot);
        let sp = path.to_string_lossy().to_string();
        if force {
            let _ = git(&self.root, &["worktree", "remove", "--force", &sp]);
        } else {
            git(&self.root, &["worktree", "remove", &sp])?;
        }
        let _ = git(&self.root, &["branch", "-D", &branch]);
        Ok(())
    }

    pub fn list(&self) -> Result<Vec<Worktree>> {
        if !self.is_git() {
            return Ok(vec![]);
        }
        let text = git(&self.root, &["worktree", "list", "--porcelain"])?;
        let mut out = Vec::new();
        let mut path = PathBuf::new();
        let mut branch = String::new();
        for line in text.lines() {
            if let Some(p) = line.strip_prefix("worktree ") {
                path = PathBuf::from(p);
            } else if let Some(b) = line.strip_prefix("branch refs/heads/") {
                branch = b.to_string();
            } else if line.is_empty() && !branch.is_empty() {
                if branch.starts_with("harnes/") {
                    let mut parts = branch.splitn(3, '/');
                    parts.next();
                    let run = parts.next().unwrap_or("").to_string();
                    let lot = parts.next().unwrap_or("").to_string();
                    out.push(Worktree {
                        lot,
                        branch: branch.clone(),
                        path: path.clone(),
                        base: run,
                    });
                }
                branch.clear();
            }
        }
        if branch.starts_with("harnes/") {
            let mut parts = branch.splitn(3, '/');
            parts.next();
            let run = parts.next().unwrap_or("").to_string();
            let lot = parts.next().unwrap_or("").to_string();
            out.push(Worktree {
                lot,
                branch,
                path,
                base: run,
            });
        }
        Ok(out)
    }
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)?.flatten() {
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if from.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sh(root: &Path, args: &[&str]) -> String {
        git(root, args).unwrap()
    }

    #[test]
    fn creates_commits_merges_and_cleans_up() {
        let root = std::env::temp_dir().join(format!("harnes-wt-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        sh(&root, &["init"]);
        sh(&root, &["config", "user.email", "t@harnes.local"]);
        sh(&root, &["config", "user.name", "harnes test"]);
        std::fs::write(root.join("a.txt"), "one").unwrap();
        sh(&root, &["add", "."]);
        sh(&root, &["commit", "-m", "init"]);

        std::fs::create_dir_all(root.join(".claude/agents")).unwrap();
        std::fs::write(root.join(".claude/agents/worker.md"), "role").unwrap();
        std::fs::write(root.join("CLAUDE.md"), "harness").unwrap();
        std::fs::write(root.join(".mcp.json"), r#"{"mcpServers":{}}"#).unwrap();

        let w = Worktrees::load(&root);
        assert!(w.active());
        let wt = w.create("42", "L1").unwrap();
        assert!(wt.path.exists());
        w.prepare_for_agent(&wt).unwrap();
        assert!(wt.path.join(".claude/agents/worker.md").exists());
        assert!(wt.path.join("CLAUDE.md").exists());
        assert!(wt.path.join(".mcp.json").exists());
        std::fs::write(wt.path.join("a.txt"), "two").unwrap();
        assert!(w.changed_files(&wt).unwrap().contains(&"a.txt".to_string()));
        w.finish(&wt, "harnes: L1").unwrap();
        assert_eq!(std::fs::read_to_string(root.join("a.txt")).unwrap(), "two");
        assert!(root.join(".claude/agents/worker.md").exists());
        assert!(sh(
            &root,
            &["ls-files", "--", ".mcp.json", "CLAUDE.md", ".claude"]
        )
        .is_empty());
        assert!(!wt.path.exists());
        assert!(w.list().unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }
}
