//! What may go to a free model. Pure functions: easy to test, easy to read.

use crate::Profile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// One reviewer among several in a deep review; a verifier arbitrates.
    Finder,
    /// A worker's first attempt at a lot; a Claude reviewer reads the result.
    Worker,
}

pub struct Task<'a> {
    pub kind: Kind,
    /// 0 is the first attempt. A retry after a review is Claude's.
    pub round: u32,
    /// The run is in light mode (a small change).
    pub light: bool,
    /// The files of the lot (workers). Empty for finders.
    pub files: &'a [String],
    /// The worker would run in its own worktree: its mistakes stay on a branch.
    pub isolated: bool,
    /// The Teacher rated the run high risk.
    pub high_risk: bool,
}

/// Paths where a mistake costs too much to bet on a free model: credentials,
/// CI, authentication, payment, migrations.
pub fn sensitive(path: &str) -> bool {
    let p = path.replace('\\', "/").to_lowercase();
    let name = p.rsplit('/').next().unwrap_or("");
    if name.starts_with(".env")
        || [".pem", ".key", ".p12", ".pfx", ".keystore"]
            .iter()
            .any(|e| name.ends_with(e))
        || [".npmrc", ".pypirc", "id_rsa", "id_ed25519", ".netrc"].contains(&name)
    {
        return true;
    }
    p.split('/').any(|seg| {
        let stem = seg.split('.').next().unwrap_or(seg);
        [".github", ".ssh", ".aws", ".gnupg"].contains(&seg)
            || ["auth", "oauth", "security", "secrets", "secret"].contains(&stem)
            || [
                "secret",
                "credential",
                "password",
                "payment",
                "billing",
                "migration",
                "privatekey",
                "private_key",
            ]
            .iter()
            .any(|k| seg.contains(k))
    })
}

/// Whether this task may be tried on a free model under `profile`.
pub fn wants_free(profile: Profile, t: &Task) -> bool {
    if profile == Profile::Claude || t.high_risk || t.files.iter().any(|f| sensitive(f)) {
        return false;
    }
    match t.kind {
        Kind::Finder => true,
        Kind::Worker => {
            t.isolated
                && t.round == 0
                && match profile {
                    Profile::Mixed => t.light,
                    Profile::MaxEco => true,
                    Profile::Claude => false,
                }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn worker<'a>(files: &'a [String], round: u32, light: bool, isolated: bool) -> Task<'a> {
        Task {
            kind: Kind::Worker,
            round,
            light,
            files,
            isolated,
            high_risk: false,
        }
    }

    #[test]
    fn sensitive_paths_stay_on_claude() {
        for p in [
            ".env",
            "app/.env.local",
            "src/auth/login.rs",
            ".github/workflows/ci.yml",
            "db/migrations/001.sql",
            "keys/server.pem",
            "src\\billing\\invoice.ts",
            "C:/p/.ssh/config",
            "lib/payments.rs",
        ] {
            assert!(sensitive(p), "{p}");
        }
        for p in [
            "src/main.rs",
            "README.md",
            "app/src/author.ts",
            "docs/environment.md",
            "src/components/Button.tsx",
        ] {
            assert!(!sensitive(p), "{p}");
        }
    }

    #[test]
    fn what_goes_to_a_free_model_in_each_profile() {
        let calm = vec!["src/a.rs".to_string()];
        let risky = vec!["src/auth/token.rs".to_string()];
        // Claude never routes anything.
        assert!(!wants_free(
            Profile::Claude,
            &Task {
                kind: Kind::Finder,
                round: 0,
                light: false,
                files: &[],
                isolated: false,
                high_risk: false
            }
        ));
        // Finders go free in both other profiles, unless the run is high risk.
        let finder = |high_risk| Task {
            kind: Kind::Finder,
            round: 0,
            light: false,
            files: &[],
            isolated: false,
            high_risk,
        };
        assert!(
            wants_free(Profile::Mixed, &finder(false))
                && wants_free(Profile::MaxEco, &finder(false))
        );
        assert!(!wants_free(Profile::MaxEco, &finder(true)));
        // Workers: mixed only on small tasks, max-eco on all; first attempt only, isolated only.
        assert!(wants_free(Profile::Mixed, &worker(&calm, 0, true, true)));
        assert!(!wants_free(Profile::Mixed, &worker(&calm, 0, false, true)));
        assert!(wants_free(Profile::MaxEco, &worker(&calm, 0, false, true)));
        assert!(
            !wants_free(Profile::MaxEco, &worker(&calm, 1, false, true)),
            "a retry is Claude's"
        );
        assert!(
            !wants_free(Profile::MaxEco, &worker(&calm, 0, false, false)),
            "no worktree, no free worker"
        );
        assert!(
            !wants_free(Profile::MaxEco, &worker(&risky, 0, false, true)),
            "sensitive lot"
        );
    }
}
