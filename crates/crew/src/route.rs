//! Which model takes a step: a free OpenCode model first, where the result is
//! checked afterwards, otherwise (or when the free try fails) Claude.

use crate::{
    budget,
    engine::{AgentHost, Engine, Run},
    Role,
};
use anyhow::Result;
use harnes_free::{Config, Kind, Task};
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use std::path::Path;

/// A fingerprint of the working tree: what git says changed, and how. Equal
/// before and after means the agent left the project's files alone. None when
/// the folder is not a git repository, which cannot be checked.
pub(crate) fn tree_digest(root: &Path) -> Option<String> {
    let git = |args: &[&str]| -> Option<Vec<u8>> {
        let mut c = std::process::Command::new("git");
        c.arg("-C").arg(root).args(args);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            c.creation_flags(0x0800_0000);
        }
        let out = c.output().ok()?;
        out.status.success().then_some(out.stdout)
    };
    // harnes' own OpenCode config is written when the first free agent starts.
    let skip = ":(exclude)opencode.json";
    let status = git(&[
        "status",
        "--porcelain=v1",
        "-z",
        "--untracked-files=all",
        "--",
        ".",
        skip,
    ])?;
    let diff = git(&["diff", "--no-ext-diff", "--binary", "--", ".", skip])?;
    let mut h = Sha256::new();
    h.update(&status);
    h.update([0]);
    h.update(&diff);
    Some(
        h.finalize()
            .iter()
            .take(8)
            .map(|b| format!("{b:02x}"))
            .collect(),
    )
}

/// Spreads attempts over the allowed models: finders `f0`, `f1`, `f2` get
/// different ones, so they do not share one model's blind spots.
fn pick<'a>(models: &'a [String], name: &str) -> Option<&'a str> {
    if models.is_empty() {
        return None;
    }
    let seed = name
        .rsplit('-')
        .next()
        .and_then(|t| t.strip_prefix('f'))
        .and_then(|n| n.parse::<usize>().ok())
        .unwrap_or_else(|| name.bytes().map(usize::from).sum());
    models.get(seed % models.len()).map(String::as_str)
}

impl<H: AgentHost> Engine<H> {
    /// Runs one agent. A free model goes first when the profile, the task and
    /// the safety checks allow it; whatever goes wrong (no OpenCode, an error,
    /// a silence, an edit the task never allowed) the step is run again on
    /// Claude, so a free model can cost time but never a result.
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn agent_routed<T: DeserializeOwned>(
        &self,
        run: &Run,
        name: &str,
        role: Role,
        prompt: &str,
        out: &Path,
        cwd: Option<&Path>,
        task: &Task<'_>,
    ) -> Result<T> {
        let claude = Some(budget::tune(role));
        if let Ok(Some(done)) = crate::plan::read_json::<T>(out) {
            return Ok(done); // resumed run
        }
        let cfg = Config::load(&self.root);
        let profile = cfg.effective(self.eco);
        let models = if harnes_free::wants_free(profile, task) {
            cfg.available(self.eco)
        } else {
            vec![]
        };
        // A finder only reads: prove it afterwards, so it needs a repository.
        let before = (task.kind == Kind::Finder).then(|| tree_digest(&self.root));
        let model = match (&before, pick(&models, name)) {
            (Some(None), _) => None,
            (_, m) => m,
        };
        if let Some(model) = model {
            self.event(
                run,
                harnes_telemetry::Event::new(&run.id, "", "route.free")
                    .role(role.agent())
                    .summary(format!("{name} essaie {model} ({:?})", profile)),
            );
            let tried = self
                .agent_tuned_at::<T>(run, name, role, prompt, out, None, cwd, Some(model))
                .await;
            let verdict = match tried {
                Ok(v) => match (&before, task.kind) {
                    (Some(Some(b)), Kind::Finder)
                        if tree_digest(&self.root).as_deref() != Some(b.as_str()) =>
                    {
                        Err(anyhow::anyhow!(
                            "a modifié des fichiers du projet alors qu'il ne devait que lire"
                        ))
                    }
                    _ => Ok(v),
                },
                Err(e) => Err(e),
            };
            match verdict {
                Ok(v) => return Ok(v),
                Err(e) if *self.cancel.borrow() => return Err(e),
                Err(e) => {
                    // Whatever it left in the output file is not trusted.
                    let _ = std::fs::remove_file(out);
                    self.event(
                        run,
                        harnes_telemetry::Event::new(&run.id, "", "route.fallback")
                            .role(role.agent())
                            .status("warn")
                            .summary(format!(
                                "{name} : {model} a échoué ({e:#}), reprise sur Claude"
                            )),
                    );
                    self.say(
                        run,
                        crate::Phase::Working,
                        0,
                        format!("{name} : modèle gratuit abandonné ({e:#}), reprise sur Claude"),
                    );
                }
            }
        }
        self.agent_tuned_at(run, name, role, prompt, out, claude, cwd, None)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finders_get_different_models() {
        let m: Vec<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
        assert_eq!(pick(&m, "reviewer-0-f0"), Some("a"));
        assert_eq!(pick(&m, "reviewer-0-f1"), Some("b"));
        assert_eq!(pick(&m, "reviewer-0-f4"), Some("b"));
        assert!(pick(&[], "x").is_none());
        assert!(pick(&m, "worker-L1-r0").is_some());
    }

    #[test]
    fn the_tree_digest_moves_when_a_file_does() {
        let root = std::env::temp_dir().join(format!("harnes-digest-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        assert!(tree_digest(&root).is_none(), "not a repository");
        let git = |a: &[&str]| {
            assert!(std::process::Command::new("git")
                .arg("-C")
                .arg(&root)
                .args(a)
                .output()
                .unwrap()
                .status
                .success())
        };
        git(&["init", "-q"]);
        std::fs::write(root.join("a.txt"), "one").unwrap();
        git(&["add", "."]);
        git(&[
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@t",
            "commit",
            "-qm",
            "x",
        ]);
        let clean = tree_digest(&root).unwrap();
        assert_eq!(clean, tree_digest(&root).unwrap());
        std::fs::write(root.join("a.txt"), "two").unwrap();
        assert_ne!(clean, tree_digest(&root).unwrap(), "an edit");
        std::fs::write(root.join("a.txt"), "one").unwrap();
        std::fs::write(root.join("opencode.json"), "{}").unwrap();
        assert_eq!(
            clean,
            tree_digest(&root).unwrap(),
            "harnes' own OpenCode config does not count"
        );
        std::fs::write(root.join("new.txt"), "x").unwrap();
        assert_ne!(clean, tree_digest(&root).unwrap(), "a new file");
        let _ = std::fs::remove_dir_all(&root);
    }
}
