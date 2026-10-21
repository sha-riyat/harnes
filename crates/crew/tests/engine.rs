//! Full crew loop against a fake host that plays the agents.

use harnes_crew::{
    budget::{Mode, Tune},
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
async fn light_run_is_one_worker_and_a_sonnet_review() {
    let (host, root, out) = quick("light", Mode::Light).await;
    assert!(out.gate.ready);
    let roles: Vec<String> = host
        .spawned
        .lock()
        .iter()
        .map(|k| k.split('-').nth(1).unwrap().to_string())
        .collect();
    assert_eq!(roles, ["worker", "reviewer"], "no planner, no Teacher");
    assert!(host
        .tunes
        .lock()
        .iter()
        .all(|t| t.map(|t| t.model) == Some("sonnet")));
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
