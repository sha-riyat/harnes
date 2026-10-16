//! Runs test specs: app lifecycle, steps, act replay, report.

use crate::{
    app::AppProcess,
    browser::{find_browser, Session},
    cache,
    report::{self, Report, Status, TestResult, Unresolved},
    spec::{self, Config, Step, TestSpec},
};
use anyhow::Result;
use std::{
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

pub struct RunOptions {
    pub files: Vec<PathBuf>,
    pub headed: bool,
}

fn rel(root: &Path, p: &Path) -> String {
    p.strip_prefix(root).unwrap_or(p).display().to_string().replace('\\', "/")
}

/// Runs everything and writes `.harnes/e2e/report.json`. Never panics on test errors.
pub async fn run(root: &Path, opts: RunOptions) -> Report {
    let report = match run_inner(root, &opts).await {
        Ok(r) => r,
        Err((msg, env)) => Report::build(vec![], vec![], vec![msg], env),
    };
    let d = report::dir(root);
    let _ = std::fs::create_dir_all(&d);
    let _ = std::fs::write(d.join("report.json"), serde_json::to_string_pretty(&report).unwrap_or_default());
    report
}

async fn run_inner(root: &Path, opts: &RunOptions) -> Result<Report, (String, bool)> {
    let cfg = spec::load_config(root).map_err(|e| (format!("{e:#}"), false))?;
    let files = if opts.files.is_empty() { spec::discover(root) } else { opts.files.iter().map(|f| root.join(f)).collect() };
    if files.is_empty() {
        return Err(("aucun test trouvé dans tests/**/*.e2e.yaml".into(), false));
    }
    let exe = find_browser(cfg.browser.as_deref()).map_err(|e| (format!("{e:#}"), true))?;
    let mut app = AppProcess::start(&cfg.app, root).map_err(|e| (format!("app : {e:#}"), true))?;

    let mut results = Vec::new();
    let mut unresolved = Vec::new();
    let mut errors = Vec::new();
    for file in &files {
        let rel_file = rel(root, file);
        let spec = match spec::load_test(file) {
            Ok(s) => s,
            Err(e) => {
                errors.push(format!("{e:#}"));
                continue;
            }
        };
        // Fresh browser per test: no shared cookies or storage.
        let session = Session::launch(&exe, &cfg.app.url, opts.headed || cfg.headed, Duration::from_millis(cfg.timeout_ms))
            .await
            .map_err(|e| (format!("navigateur : {e:#}"), true))?;
        let (res, unres) = run_test(root, &session, &rel_file, &spec).await;
        results.push(res);
        unresolved.extend(unres);
        session.close().await;
    }
    app.stop();
    Ok(Report::build(results, unresolved, errors, false))
}

/// Expands a step into what actually runs: `act` becomes its cached actions.
pub fn expand(root: &Path, file: &str, spec: &TestSpec, index: usize) -> Option<Vec<Step>> {
    match &spec.steps[index] {
        Step::Act(a) => cache::get(root, &cache::key(file, &spec.name, index, a)).map(|r| r.actions),
        other => Some(vec![other.clone()]),
    }
}

/// Runs steps `[0, upto)` (all when `None`). Used by the runner and by the
/// MCP server to bring the app to the state just before an `act` step.
pub async fn run_steps(root: &Path, s: &Session, file: &str, spec: &TestSpec, upto: Option<usize>) -> Result<usize, (usize, String, bool)> {
    let mut replayed = 0;
    for (i, step) in spec.steps.iter().enumerate().take(upto.unwrap_or(spec.steps.len())) {
        let Some(actions) = expand(root, file, spec, i) else {
            return Err((i, "étape `act` pas encore résolue".into(), true));
        };
        let is_act = matches!(step, Step::Act(_));
        replayed += is_act as usize;
        for a in &actions {
            let r = match a {
                Step::Expect(e) => s.check(e).await,
                other => s.perform(other).await,
            };
            if let Err(e) = r {
                // A broken replay means the app changed: needs a new resolution.
                return Err((i, format!("{e:#}"), is_act));
            }
        }
    }
    Ok(replayed)
}

async fn run_test(root: &Path, s: &Session, file: &str, spec: &TestSpec) -> (TestResult, Vec<Unresolved>) {
    let start = Instant::now();
    let mut res = TestResult {
        file: file.to_string(),
        name: spec.name.clone(),
        status: Status::Passed,
        millis: 0,
        failed_step: None,
        error: None,
        trace: None,
        replayed: 0,
    };
    let mut unresolved = vec![];
    match run_steps(root, s, file, spec, None).await {
        Ok(n) => res.replayed = n,
        Err((i, msg, act_problem)) => {
            res.failed_step = Some(i);
            res.error = Some(msg.clone());
            if let (true, Step::Act(a)) = (act_problem, &spec.steps[i]) {
                res.status = Status::Unresolved;
                unresolved.push(Unresolved { file: file.into(), test: spec.name.clone(), step: i, act: a.clone(), reason: msg });
            } else {
                res.status = Status::Failed;
            }
            let screen = s.snapshot().await.map(|sn| sn.render()).unwrap_or_default();
            let shot = report::dir(root).join("results").join(format!("{}.png", report::slug(&format!("{file}-{}", spec.name))));
            let shot = s.screenshot(&shot).await.ok().map(|_| shot);
            let desc = serde_json::to_string(&spec.steps[i]).unwrap_or_default();
            res.trace = report::write_trace(root, &res, &desc, &screen, shot.as_deref());
        }
    }
    res.millis = start.elapsed().as_millis();
    (res, unresolved)
}

/// Loads config + spec for the MCP server.
pub fn load(root: &Path, file: &str) -> Result<(Config, TestSpec)> {
    Ok((spec::load_config(root)?, spec::load_test(&root.join(file))?))
}
