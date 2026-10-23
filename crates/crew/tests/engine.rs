//! Full crew loop against a fake host that plays the agents.

use harnes_crew::{
    budget::{Mode, Tune, OPUS_MEDIUM, SONNET_HIGH},
    engine::{AgentHost, Engine},
    hooks::{HookServer, Registry},
    AgentState, CrewEvent, Phase, Role,
};
use parking_lot::Mutex;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

struct FakeHost {
    root: PathBuf,
    registry: Arc<Registry>,
    reviews: Mutex<u32>,
    spawned: Mutex<Vec<String>>,
    tunes: Mutex<Vec<Option<Tune>>>,
}

/// First `...json` path between backticks in the prompt.
fn out_path(prompt: &str) -> String {
    prompt
        .split('`')
        .filter(|s| s.ends_with(".json"))
        .last()
        .unwrap()
        .to_string()
}

impl AgentHost for FakeHost {
    fn spawn(
        &self,
        key: &str,
        role: Role,
        prompt: &str,
        settings: &Path,
        tune: Option<Tune>,
    ) -> anyhow::Result<String> {
        assert!(settings.exists());
        self.spawned.lock().push(key.to_string());
        self.tunes.lock().push(tune);
        let out = self.root.join(out_path(prompt));
        let body = match role {
            Role::Planner => r#"{"context":"test","lots":[
                {"id":"L1","title":"a","instructions":"x","files":["a.rs"]},
                {"id":"L2","title":"b","instructions":"y","files":["b.rs"]}],
                "waves":[["L1","L2"]]}"#
                .to_string(),
            Role::Teacher => {
                if prompt.contains("Calibrage") {
                    r#"{"criteria":[{"id":"C1","lot":"L1","description":"a","evidence":"review"}],"notify_user_when":"C1 ok"}"#.to_string()
                } else {
                    r#"{"ready":true,"met":["C1"],"summary_for_user":"tout est prêt"}"#.to_string()
                }
            }
            Role::Worker => format!(
                r#"{{"lot":"{}","status":"ok"}}"#,
                if prompt.contains("\"L2\"") {
                    "L2"
                } else {
                    "L1"
                }
            ),
            Role::Reviewer => {
                let mut n = self.reviews.lock();
                *n += 1;
                if *n == 1 {
                    r#"{"verdict":"CHANGES","fixes":[{"lot":"L1","where":"a.rs:1","problem":"bug","fix":"fix it"}]}"#.into()
                } else {
                    r#"{"verdict":"PASS","commits":[{"lot":"L1","message":"Add a","files":["a.rs"]}]}"#.into()
                }
            }
        };
        let pane = format!("pane-{key}");
        let (reg, key) = (self.registry.clone(), key.to_string());
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(30)).await;
            reg.set(&key, AgentState::Working, false);
            std::fs::write(&out, body).unwrap();
            reg.set(&key, AgentState::Done, true);
        });
        Ok(pane)
    }
    fn close(&self, _pane: &str) {}
}

#[tokio::test]
async fn plan_work_review_fix_pass() {
    let root = std::env::temp_dir().join(format!("harnes-engine-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();

    let hooks = Arc::new(HookServer::start(Arc::new(|_: &str, _| {})).await.unwrap());
    let host = Arc::new(FakeHost {
        root: root.clone(),
        registry: hooks.registry.clone(),
        reviews: Mutex::new(0),
        spawned: Mutex::new(vec![]),
        tunes: Mutex::new(vec![]),
    });
    let events: Arc<Mutex<Vec<CrewEvent>>> = Arc::default();
    let ev = events.clone();
    let (_tx, cancel) = tokio::sync::watch::channel(false);
    let threads = std::sync::Arc::new(harnes_threads::Threads::in_memory().unwrap());
    let engine = Engine {
        host: host.clone(),
        hooks,
        project: "test".into(),
        root: root.clone(),
        emit: Arc::new(move |e| ev.lock().push(e)),
        cancel,
        max_rounds: 3,
        agent_timeout: Duration::from_secs(10),
        run_e2e: true,
        threads: harnes_crew::threading::RunThreads(Some(threads.clone())),
        mode: Mode::Full,
        eco: false,
        telemetry: None,
        memory: None,
        worktrees: None,
    };

    let out = engine.run("do the thing").await.expect("run passes");
    assert!(out.review.passed() && out.gate.ready);
    assert_eq!(out.review.commits[0].message, "Add a");

    let spawned = host.spawned.lock().clone();
    let roles: Vec<&str> = spawned
        .iter()
        .map(|k| k.split('-').nth(1).unwrap())
        .collect();
    assert_eq!(
        roles,
        ["planner", "teacher", "worker", "worker", "reviewer", "worker", "reviewer", "teacher"]
    );
    let tunes = host.tunes.lock().clone();
    assert_eq!(tunes[0], Some(OPUS_MEDIUM), "only the Planner gets Opus");
    assert!(
        tunes[1..].iter().all(|t| *t == Some(SONNET_HIGH)),
        "every other role gets Sonnet high: {tunes:?}"
    );
    assert!(
        spawned[5].ends_with("worker-L1-r1"),
        "fix goes to L1: {spawned:?}"
    );
    assert_eq!(events.lock().last().unwrap().phase, Phase::Done);
    let all = threads.list("all", None, 50).unwrap();
    assert_eq!(all.len(), 3, "run thread + one per lot");
    assert!(
        all.iter().all(|t| t.status == "closed"),
        "closed after the gate"
    );
    let l1 = all.iter().find(|t| t.id.ends_with("-L1")).unwrap();
    let msgs = threads.read(&l1.id, 0, 50).unwrap();
    assert!(
        msgs.iter()
            .any(|m| m.body.starts_with("Corrections demandées")),
        "review fixes posted to the lot thread"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn resume_skips_finished_steps() {
    let root = std::env::temp_dir().join(format!("harnes-resume-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join(".harnes/runs/42");
    std::fs::create_dir_all(dir.join("reports")).unwrap();
    std::fs::write(
        dir.join("plan.json"),
        r#"{"lots":[{"id":"L1","title":"a","instructions":"x","files":["a.rs"]}]}"#,
    )
    .unwrap();
    std::fs::write(dir.join("bar.json"), r#"{"criteria":[]}"#).unwrap();

    let hooks = Arc::new(HookServer::start(Arc::new(|_: &str, _| {})).await.unwrap());
    let host = Arc::new(FakeHost {
        root: root.clone(),
        registry: hooks.registry.clone(),
        reviews: Mutex::new(1),
        spawned: Mutex::new(vec![]),
        tunes: Mutex::new(vec![]),
    });
    let (_tx, cancel) = tokio::sync::watch::channel(false);
    let engine = Engine {
        host: host.clone(),
        hooks,
        project: "test".into(),
        root: root.clone(),
        emit: Arc::new(|_| {}),
        cancel,
        max_rounds: 3,
        agent_timeout: Duration::from_secs(10),
        run_e2e: false,
        threads: Default::default(),
        mode: Mode::Full,
        eco: false,
        telemetry: None,
        memory: None,
        worktrees: None,
    };
    let out = engine
        .run_resumable("t", Some("42".into()))
        .await
        .expect("resumed run passes");
    assert!(out.gate.ready);
    let roles: Vec<String> = host
        .spawned
        .lock()
        .iter()
        .map(|k| k.split('-').nth(1).unwrap().to_string())
        .collect();
    assert_eq!(
        roles,
        ["worker", "reviewer", "teacher"],
        "planner and teacher-bar were skipped"
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// A crew engine on a fake host, for one mode; the reviewer passes at once.
async fn quick(name: &str, mode: Mode) -> (Arc<FakeHost>, PathBuf, harnes_crew::engine::Outcome) {
    let root = std::env::temp_dir().join(format!("harnes-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let hooks = Arc::new(HookServer::start(Arc::new(|_: &str, _| {})).await.unwrap());
    let host = Arc::new(FakeHost {
        root: root.clone(),
        registry: hooks.registry.clone(),
        reviews: Mutex::new(1),
        spawned: Mutex::new(vec![]),
        tunes: Mutex::new(vec![]),
    });
    let (_tx, cancel) = tokio::sync::watch::channel(false);
    let engine = Engine {
        host: host.clone(),
        hooks,
        project: "test".into(),
        root: root.clone(),
        emit: Arc::new(|_| {}),
        cancel,
        max_rounds: 3,
        agent_timeout: Duration::from_secs(10),
        run_e2e: false,
        threads: Default::default(),
        mode,
        eco: false,
        telemetry: None,
        memory: None,
        worktrees: None,
    };
    let out = engine
        .run_resumable("fix the typo", Some("7".into()))
        .await
        .expect("run passes");
    (host, root, out)
}

#[tokio::test]
async fn light_run_is_one_worker_and_a_sonnet_high_review() {
    let (host, root, out) = quick("light", Mode::Light).await;
    assert!(out.gate.ready);
    let roles: Vec<String> = host
        .spawned
        .lock()
        .iter()
        .map(|k| k.split('-').nth(1).unwrap().to_string())
        .collect();
    assert_eq!(roles, ["worker", "reviewer"], "no planner, no Teacher");
    assert!(host.tunes.lock().iter().all(|t| *t == Some(SONNET_HIGH)));
    let metrics: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join(".harnes/runs/7/metrics.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(
        (metrics["rounds"].as_u64(), metrics["passed"].as_bool()),
        (Some(1), Some(true))
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn deep_review_has_finders_then_a_verifier() {
    let (host, root, _) = quick("deep", Mode::Deep).await;
    let spawned = host.spawned.lock().clone();
    let reviewers: Vec<&String> = spawned
        .iter()
        .filter(|k| k.contains("-reviewer-"))
        .collect();
    assert_eq!(
        reviewers.len(),
        4,
        "three finders and a verifier: {spawned:?}"
    );
    assert!(reviewers.last().unwrap().ends_with("reviewer-0-verify"));
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn history_sums_the_runs() {
    let (_, root, _) = quick("history", Mode::Light).await;
    let h = harnes_crew::review::history(&root);
    assert_eq!((h.runs, h.passed, h.first_pass), (1, 1, 1));
    let _ = std::fs::remove_dir_all(&root);
}

/// An agent that starts, says it works, and is never heard from again.
struct StuckHost {
    registry: Arc<Registry>,
    closed: Mutex<u32>,
}

impl AgentHost for StuckHost {
    fn spawn(
        &self,
        key: &str,
        _role: Role,
        _prompt: &str,
        _settings: &Path,
        _tune: Option<Tune>,
    ) -> anyhow::Result<String> {
        self.registry.set(key, AgentState::Working, false);
        Ok(format!("pane-{key}"))
    }
    fn close(&self, _pane: &str) {
        *self.closed.lock() += 1;
    }
}

#[tokio::test]
async fn a_silent_agent_is_stopped_long_before_the_timeout() {
    let root = std::env::temp_dir().join(format!("harnes-stuck-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let hooks = Arc::new(HookServer::start(Arc::new(|_: &str, _| {})).await.unwrap());
    let host = Arc::new(StuckHost {
        registry: hooks.registry.clone(),
        closed: Mutex::new(0),
    });
    let (_tx, cancel) = tokio::sync::watch::channel(false);
    let engine = Engine {
        host: host.clone(),
        hooks,
        project: "test".into(),
        root: root.clone(),
        emit: Arc::new(|_| {}),
        cancel,
        max_rounds: 1,
        // The stall limit is a third of this: one second, against a minute.
        agent_timeout: Duration::from_secs(3),
        run_e2e: false,
        threads: Default::default(),
        mode: Mode::Light,
        eco: false,
        telemetry: None,
        memory: None,
        worktrees: None,
    };
    let started = std::time::Instant::now();
    let err = engine
        .run_resumable("t", Some("9".into()))
        .await
        .expect_err("nothing was ever written");
    assert!(format!("{err:#}").contains("figé"), "{err:#}");
    assert!(*host.closed.lock() >= 1, "the stuck pane is closed");
    assert!(started.elapsed() < Duration::from_secs(20));
    let metrics = std::fs::read_to_string(root.join(".harnes/runs/9/metrics.json")).unwrap();
    assert!(metrics.contains("\"passed\": false"));
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_run_records_its_harness_and_flags_a_change_on_resume() {
    let (_, root, _) = quick("fingerprint", Mode::Light).await;
    let first: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join(".harnes/runs/7/metrics.json")).unwrap(),
    )
    .unwrap();
    let fp = first["harness"].as_str().unwrap().to_string();
    assert_eq!(fp.len(), 12);
    assert_eq!(
        std::fs::read_to_string(root.join(".harnes/runs/7/harness.fp")).unwrap(),
        fp
    );
    assert_eq!(first["harness_changed"], false);
    // Someone edits a prompt, then the same run is resumed.
    std::fs::create_dir_all(root.join(".claude/agents")).unwrap();
    std::fs::write(root.join(".claude/agents/worker.md"), "edited").unwrap();
    let hooks = Arc::new(HookServer::start(Arc::new(|_: &str, _| {})).await.unwrap());
    let host = Arc::new(FakeHost {
        root: root.clone(),
        registry: hooks.registry.clone(),
        reviews: Mutex::new(1),
        spawned: Mutex::new(vec![]),
        tunes: Mutex::new(vec![]),
    });
    let (_tx, cancel) = tokio::sync::watch::channel(false);
    let engine = Engine {
        host,
        hooks,
        project: "test".into(),
        root: root.clone(),
        emit: Arc::new(|_| {}),
        cancel,
        max_rounds: 3,
        agent_timeout: Duration::from_secs(10),
        run_e2e: false,
        threads: Default::default(),
        mode: Mode::Light,
        eco: false,
        telemetry: None,
        memory: None,
        worktrees: None,
    };
    engine
        .run_resumable("fix the typo", Some("7".into()))
        .await
        .unwrap();
    let again: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(root.join(".harnes/runs/7/metrics.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(again["harness_changed"], true);
    assert_ne!(again["harness"].as_str().unwrap(), fp);
    let _ = std::fs::remove_dir_all(&root);
}

// ---- free-model routing ---------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum Free {
    /// The free model reviews and passes.
    Passes,
    /// OpenCode cannot be started.
    Missing,
    /// The free reviewer edits a project file while it is supposed to only read.
    Edits,
}

struct FreeHost {
    inner: FakeHost,
    mode: Free,
    free_calls: Mutex<Vec<(String, String)>>,
}

impl AgentHost for FreeHost {
    fn spawn(
        &self,
        key: &str,
        role: Role,
        prompt: &str,
        settings: &Path,
        tune: Option<Tune>,
    ) -> anyhow::Result<String> {
        self.inner.spawn(key, role, prompt, settings, tune)
    }
    fn spawn_free(
        &self,
        key: &str,
        _role: Role,
        prompt_file: &Path,
        _settings: &Path,
        model: &str,
        _cwd: Option<&Path>,
    ) -> anyhow::Result<String> {
        if self.mode == Free::Missing {
            anyhow::bail!("opencode ne démarre pas");
        }
        self.free_calls
            .lock()
            .push((key.to_string(), model.to_string()));
        let prompt = std::fs::read_to_string(prompt_file).unwrap();
        let out = self.inner.root.join(out_path(&prompt));
        let edits = self.mode == Free::Edits;
        let root = self.inner.root.clone();
        let pane = format!("free-{key}");
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(30)).await;
            if edits {
                std::fs::write(root.join("a.txt"), "changed by a reviewer").unwrap();
            }
            std::fs::write(
                &out,
                r#"{"verdict":"PASS","summary":"0 problème bloquant"}"#,
            )
            .unwrap();
            // OpenCode sends no hooks: the engine finds the file by polling.
        });
        Ok(pane)
    }
    fn has_hooks(&self, role: Role) -> bool {
        self.inner.has_hooks(role)
    }
    fn close(&self, _pane: &str) {}
}

/// A stand-in `opencode` that offers one free model.
fn fake_opencode(dir: &Path) -> PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let p = dir.join(if cfg!(windows) {
        "opencode.cmd"
    } else {
        "opencode"
    });
    let body = if cfg!(windows) {
        "@echo off\r\necho opencode/glm-5.3-flash-free\r\n"
    } else {
        "#!/bin/sh\necho opencode/glm-5.3-flash-free\n"
    };
    std::fs::write(&p, body).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    p
}

/// A deep run in a git repository, with the Mixed profile and a fake OpenCode.
async fn deep_with_free(
    name: &str,
    mode: Free,
    profile: &str,
) -> (Arc<FreeHost>, PathBuf, harnes_crew::engine::Outcome) {
    // One at a time: they share the process environment and OpenCode's model cache.
    static ONE_AT_A_TIME: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _turn = ONE_AT_A_TIME.lock().await;
    let root = std::env::temp_dir().join(format!("harnes-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".harnes")).unwrap();
    std::fs::write(
        root.join(".harnes/profile.json"),
        format!(r#"{{"profile":"{profile}"}}"#),
    )
    .unwrap();
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
    std::fs::write(root.join(".gitignore"), ".harnes/\n").unwrap();
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
    std::env::set_var("HARNES_OPENCODE_BIN", fake_opencode(&root.join("bin")));
    let hooks = Arc::new(HookServer::start(Arc::new(|_: &str, _| {})).await.unwrap());
    let host = Arc::new(FreeHost {
        inner: FakeHost {
            root: root.clone(),
            registry: hooks.registry.clone(),
            reviews: Mutex::new(1),
            spawned: Mutex::new(vec![]),
            tunes: Mutex::new(vec![]),
        },
        mode,
        free_calls: Mutex::new(vec![]),
    });
    let (_tx, cancel) = tokio::sync::watch::channel(false);
    let engine = Engine {
        host: host.clone(),
        hooks,
        project: "test".into(),
        root: root.clone(),
        emit: Arc::new(|_| {}),
        cancel,
        max_rounds: 3,
        agent_timeout: Duration::from_secs(20),
        run_e2e: false,
        threads: Default::default(),
        mode: Mode::Deep,
        eco: false,
        telemetry: None,
        memory: None,
        worktrees: None,
    };
    let out = engine
        .run_resumable("fix the typo", Some("11".into()))
        .await
        .expect("run passes");
    std::env::remove_var("HARNES_OPENCODE_BIN");
    (host, root, out)
}

#[tokio::test]
async fn mixed_profile_gives_the_finders_to_a_free_model_and_a_claude_verifier_checks() {
    let (host, root, out) = deep_with_free("free-ok", Free::Passes, "mixed").await;
    assert!(out.gate.ready);
    let free = host.free_calls.lock().clone();
    assert_eq!(free.len(), 3, "three finders on the free model: {free:?}");
    assert!(free
        .iter()
        .all(|(k, m)| k.contains("-reviewer-0-f") && m == "opencode/glm-5.3-flash-free"));
    let claude: Vec<String> = host.inner.spawned.lock().clone();
    assert!(
        claude.iter().any(|k| k.ends_with("reviewer-0-verify")),
        "the verifier stays on Claude: {claude:?}"
    );
    assert!(
        !claude.iter().any(|k| k.contains("-f0")),
        "no finder on Claude"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn claude_profile_never_routes_to_a_free_model() {
    let (host, root, _) = deep_with_free("free-off", Free::Passes, "claude").await;
    assert!(host.free_calls.lock().is_empty());
    assert_eq!(
        host.inner
            .spawned
            .lock()
            .iter()
            .filter(|k| k.contains("-reviewer-0-f"))
            .count(),
        3
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_missing_opencode_falls_back_to_claude_without_losing_the_run() {
    let (host, root, out) = deep_with_free("free-missing", Free::Missing, "mixed").await;
    assert!(out.gate.ready);
    assert_eq!(
        host.inner
            .spawned
            .lock()
            .iter()
            .filter(|k| k.contains("-reviewer-0-f"))
            .count(),
        3,
        "all three finders reran on Claude"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[tokio::test]
async fn a_free_reviewer_that_edits_files_is_discarded_and_redone_on_claude() {
    let (host, root, out) = deep_with_free("free-edits", Free::Edits, "mixed").await;
    assert!(out.gate.ready);
    assert!(host.free_calls.lock().len() >= 1);
    assert!(
        host.inner
            .spawned
            .lock()
            .iter()
            .filter(|k| k.contains("-reviewer-0-f"))
            .count()
            >= 1,
        "the discarded finder reran on Claude"
    );
    let _ = std::fs::remove_dir_all(&root);
}
