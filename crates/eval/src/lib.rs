//! Evaluation suites for harnes.
//!
//! A suite is a YAML file in `.harnes/evals/`: a list of tasks, each with
//! deterministic checks (command, exit code, expected output). The same file
//! can gate a release, compare two branches, or score harness variants.
//! The runner is deliberately small and reproducible: no model call, no hidden
//! judge, a ledger of time and exit codes.

pub mod passk;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::Instant,
};
use wait_timeout::ChildExt;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suite {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tasks: Vec<Task>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    #[serde(default)]
    pub title: String,
    /// Prompt an agent would receive. Kept for harness experiments.
    #[serde(default)]
    pub prompt: String,
    #[serde(default)]
    pub checks: Vec<Check>,
    /// Soft cap for the whole task, in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
}

fn default_timeout() -> u64 {
    300
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Check {
    pub name: String,
    pub command: String,
    #[serde(default)]
    pub expect_exit: i32,
    #[serde(default)]
    pub contains: String,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default = "default_check_timeout")]
    pub timeout_secs: u64,
}

fn default_check_timeout() -> u64 {
    180
}

#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    pub name: String,
    pub command: String,
    pub exit: Option<i32>,
    pub passed: bool,
    pub timed_out: bool,
    pub duration_ms: u64,
    pub stdout_tail: String,
    pub stderr_tail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskResult {
    pub id: String,
    pub title: String,
    pub passed: bool,
    pub checks: Vec<CheckResult>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SuiteResult {
    pub schema: String,
    pub suite: String,
    pub started_at: u64,
    pub duration_ms: u64,
    pub passed: bool,
    pub passed_tasks: usize,
    pub total_tasks: usize,
    pub tasks: Vec<TaskResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Ledger {
    pub runs: u64,
    pub tasks: u64,
    pub checks: u64,
    pub passed: u64,
    pub failed: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct Variant {
    pub id: String,
    pub label: String,
    pub model: String,
    pub effort: String,
    pub skills: Vec<String>,
    pub context_tokens: u64,
    pub reviewer_rounds: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct TunePlan {
    pub component: String,
    pub suite: String,
    pub baseline: String,
    pub variants: Vec<Variant>,
    pub created_at: u64,
}

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn evals_dir(root: &Path) -> PathBuf {
    root.join(".harnes").join("evals")
}

pub fn list(root: &Path) -> Result<Vec<Suite>> {
    let dir = evals_dir(root);
    let mut out = Vec::new();
    for entry in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
        let p = entry.path();
        if p.extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e == "yaml" || e == "yml")
        {
            let text = std::fs::read_to_string(&p)?;
            let s: Suite =
                serde_yaml::from_str(&text).with_context(|| format!("{} invalide", p.display()))?;
            out.push(s);
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(out)
}

pub fn load(root: &Path, name: &str) -> Result<Suite> {
    list(root)?
        .into_iter()
        .find(|s| s.name == name)
        .with_context(|| format!("suite inconnue : {name}"))
}

pub fn ensure_defaults(root: &Path) -> Result<Option<PathBuf>> {
    let dir = evals_dir(root);
    std::fs::create_dir_all(&dir)?;
    if list(root)?.is_empty() {
        let path = dir.join("smoke.yaml");
        let checks = if root.join("Cargo.toml").exists() {
            vec![Check {
                name: "cargo check".into(),
                command: "cargo check --workspace --all-targets".into(),
                expect_exit: 0,
                contains: String::new(),
                cwd: None,
                timeout_secs: 300,
            }]
        } else if root.join("package.json").exists() {
            vec![Check {
                name: "npm build".into(),
                command: "npm run build".into(),
                expect_exit: 0,
                contains: String::new(),
                cwd: None,
                timeout_secs: 300,
            }]
        } else {
            // Keep the generated suite runnable even for projects with no
            // obvious build command; an empty task would always fail validate.
            vec![Check {
                name: "smoke".into(),
                command: "echo 'harnes eval ok'".into(),
                expect_exit: 0,
                contains: "harnes eval ok".into(),
                cwd: None,
                timeout_secs: 30,
            }]
        };
        let suite = Suite {
            name: "smoke".into(),
            description: "Baseline générée par harnes : build détecté, sinon smoke.".into(),
            tasks: vec![Task {
                id: "build".into(),
                title: "Baseline".into(),
                prompt: "La baseline du projet passe.".into(),
                checks,
                timeout_secs: 360,
            }],
        };
        std::fs::write(&path, serde_yaml::to_string(&suite)?)?;
        return Ok(Some(path));
    }
    Ok(None)
}

pub fn run_suite(root: &Path, suite: &Suite) -> SuiteResult {
    let started = now();
    let clock = Instant::now();
    let mut tasks = Vec::new();
    for task in &suite.tasks {
        let mut checks = Vec::new();
        let task_start = Instant::now();
        for check in &task.checks {
            let elapsed = task_start.elapsed().as_secs();
            let remaining = task.timeout_secs.saturating_sub(elapsed);
            checks.push(run_check(root, check, remaining));
        }
        let passed = !checks.is_empty() && checks.iter().all(|c| c.passed);
        tasks.push(TaskResult {
            id: task.id.clone(),
            title: task.title.clone(),
            passed,
            checks,
        });
    }
    let passed_tasks = tasks.iter().filter(|t| t.passed).count();
    SuiteResult {
        schema: "harnes.eval.v1".into(),
        suite: suite.name.clone(),
        started_at: started,
        duration_ms: clock.elapsed().as_millis() as u64,
        passed: !tasks.is_empty() && passed_tasks == tasks.len(),
        passed_tasks,
        total_tasks: tasks.len(),
        tasks,
    }
}

fn run_check(root: &Path, check: &Check, budget_secs: u64) -> CheckResult {
    let clock = Instant::now();
    let cwd = check
        .cwd
        .as_ref()
        .map(|c| root.join(c))
        .unwrap_or_else(|| root.to_path_buf());
    let limit = check.timeout_secs.max(1).min(budget_secs.max(1));
    let (program, args) = if cfg!(windows) {
        (
            "powershell.exe",
            vec!["-NoLogo", "-NoProfile", "-Command", &check.command],
        )
    } else {
        ("sh", vec!["-c", &check.command])
    };
    let child = Command::new(program)
        .args(args)
        .current_dir(&cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut timed_out = false;
    let (exit, stdout, stderr) = match child {
        Ok(mut child) => {
            // Drain both pipes while the child runs. Waiting for exit first
            // would deadlock as soon as a chatty build fills the pipe buffer.
            let reader = |pipe: Option<std::process::ChildStdout>| {
                pipe.map(|mut pipe| {
                    std::thread::spawn(move || {
                        let mut bytes = Vec::new();
                        let _ = pipe.read_to_end(&mut bytes);
                        bytes
                    })
                })
            };
            let stderr_reader = child.stderr.take().map(|mut pipe| {
                std::thread::spawn(move || {
                    let mut bytes = Vec::new();
                    let _ = pipe.read_to_end(&mut bytes);
                    bytes
                })
            });
            let stdout_reader = reader(child.stdout.take());
            let status = match child.wait_timeout(std::time::Duration::from_secs(limit)) {
                Ok(Some(s)) => Some(s),
                Ok(None) => {
                    timed_out = true;
                    let _ = child.kill();
                    let _ = child.wait();
                    None
                }
                Err(_) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    None
                }
            };
            let stdout = stdout_reader
                .and_then(|t| t.join().ok())
                .map(|b| String::from_utf8_lossy(&b).to_string())
                .unwrap_or_default();
            let stderr = stderr_reader
                .and_then(|t| t.join().ok())
                .map(|b| String::from_utf8_lossy(&b).to_string())
                .unwrap_or_default();
            (status.and_then(|s| s.code()), stdout, stderr)
        }
        Err(e) => (None, String::new(), e.to_string()),
    };
    let mut passed = !timed_out && exit == Some(check.expect_exit);
    if passed && !check.contains.is_empty() {
        passed = stdout.contains(&check.contains) || stderr.contains(&check.contains);
    }
    CheckResult {
        name: check.name.clone(),
        command: check.command.clone(),
        exit,
        passed,
        timed_out,
        duration_ms: clock.elapsed().as_millis() as u64,
        stdout_tail: tail(&stdout, 4000),
        stderr_tail: tail(&stderr, 4000),
    }
}

fn tail(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        let mut start = s.len() - max;
        while !s.is_char_boundary(start) {
            start += 1;
        }
        format!("…{}", &s[start..])
    }
}

pub fn compare(baseline: &SuiteResult, candidate: &SuiteResult) -> serde_json::Value {
    let better = candidate.passed_tasks as i64 - baseline.passed_tasks as i64;
    serde_json::json!({
        "baseline": {"suite": baseline.suite, "passed": baseline.passed_tasks, "total": baseline.total_tasks, "ms": baseline.duration_ms},
        "candidate": {"suite": candidate.suite, "passed": candidate.passed_tasks, "total": candidate.total_tasks, "ms": candidate.duration_ms},
        "delta_tasks": better,
        "verdict": if better > 0 { "better" } else if better < 0 { "worse" } else { "same" }
    })
}

/// A small, deterministic sweep. It varies one component at a time so a
/// difference can be attributed; no model is called here.
pub fn tune_plan(component: &str, suite: &str, model: &str, eco: bool) -> TunePlan {
    let base_effort = if eco { "low" } else { "medium" };
    let variants = vec![
        Variant {
            id: "baseline".into(),
            label: "base".into(),
            model: model.into(),
            effort: base_effort.into(),
            skills: vec![],
            context_tokens: 12_000,
            reviewer_rounds: 2,
        },
        Variant {
            id: "effort-high".into(),
            label: "effort high".into(),
            model: model.into(),
            effort: "high".into(),
            skills: vec![],
            context_tokens: 12_000,
            reviewer_rounds: 2,
        },
        Variant {
            id: "context-tight".into(),
            label: "contexte serré".into(),
            model: model.into(),
            effort: base_effort.into(),
            skills: vec![],
            context_tokens: 8_000,
            reviewer_rounds: 2,
        },
        Variant {
            id: "review-deep".into(),
            label: "revue profonde".into(),
            model: model.into(),
            effort: base_effort.into(),
            skills: vec![],
            context_tokens: 12_000,
            reviewer_rounds: 3,
        },
    ];
    TunePlan {
        component: component.into(),
        suite: suite.into(),
        baseline: "baseline".into(),
        variants,
        created_at: now(),
    }
}

pub fn ledger(root: &Path, result: &SuiteResult) -> Result<Ledger> {
    let path = root.join(".harnes").join("eval-ledger.json");
    let mut l: Ledger = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    l.runs += 1;
    l.tasks += result.total_tasks as u64;
    l.checks += result.tasks.iter().map(|t| t.checks.len()).sum::<usize>() as u64;
    l.passed += result.passed_tasks as u64;
    l.failed += (result.total_tasks - result.passed_tasks) as u64;
    l.duration_ms += result.duration_ms;
    std::fs::write(&path, serde_json::to_string_pretty(&l)?)?;
    Ok(l)
}

pub fn save_result(root: &Path, result: &SuiteResult) -> Result<PathBuf> {
    let dir = root.join(".harnes").join("evals").join("results");
    std::fs::create_dir_all(&dir)?;
    let safe: String = result
        .suite
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let path = dir.join(format!("{}-{}.json", safe, result.started_at));
    std::fs::write(&path, serde_json::to_string_pretty(result)?)?;
    Ok(path)
}

pub fn validate(suite: &Suite) -> Result<()> {
    if suite.name.trim().is_empty() {
        bail!("suite sans nom");
    }
    if suite.tasks.is_empty() {
        bail!("suite {} sans tâche", suite.name);
    }
    for t in &suite.tasks {
        if t.checks.is_empty() {
            bail!("tâche {} sans check", t.id);
        }
        for check in &t.checks {
            if check.command.trim().is_empty() {
                bail!("check {} sans commande", check.name);
            }
            if let Some(cwd) = &check.cwd {
                let path = Path::new(cwd);
                if path.is_absolute()
                    || path
                        .components()
                        .any(|c| matches!(c, std::path::Component::ParentDir))
                {
                    bail!("check {} : cwd doit rester dans le projet", check.name);
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn suite(cmds: &[(&str, &str)]) -> Suite {
        Suite {
            name: "t".into(),
            description: String::new(),
            tasks: vec![Task {
                id: "one".into(),
                title: "One".into(),
                prompt: String::new(),
                timeout_secs: 20,
                checks: cmds
                    .iter()
                    .map(|(n, c)| Check {
                        name: n.to_string(),
                        command: c.to_string(),
                        expect_exit: 0,
                        contains: String::new(),
                        cwd: None,
                        timeout_secs: 20,
                    })
                    .collect(),
            }],
        }
    }

    #[test]
    fn runs_checks_and_fails_on_exit() {
        let root = std::env::temp_dir();
        let ok = run_suite(&root, &suite(&[("echo", "echo harnes")]));
        assert!(ok.passed && ok.tasks[0].checks[0].stdout_tail.contains("harnes"));
        let bad = run_suite(&root, &suite(&[("false", "exit 3")]));
        assert!(!bad.passed && bad.tasks[0].checks[0].exit == Some(3));
    }

    #[test]
    fn drains_pipes_larger_than_the_os_buffer() {
        let command = if cfg!(windows) {
            "1..5000 | ForEach-Object { 'x' * 80 }"
        } else {
            "yes x | head -c 200000"
        };
        let result = run_suite(&std::env::temp_dir(), &suite(&[("large", command)]));
        assert!(result.passed, "{}", result.tasks[0].checks[0].stderr_tail);
        assert!(!result.tasks[0].checks[0].timed_out);
    }

    #[test]
    fn writes_defaults_and_validates() {
        let root = std::env::temp_dir().join(format!("harnes-eval-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("Cargo.toml"), "[workspace]").unwrap();
        assert!(ensure_defaults(&root).unwrap().is_some());
        let s = load(&root, "smoke").unwrap();
        validate(&s).unwrap();
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn fallback_smoke_is_runnable() {
        let root = std::env::temp_dir().join(format!("harnes-eval-empty-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        assert!(ensure_defaults(&root).unwrap().is_some());
        let s = load(&root, "smoke").unwrap();
        validate(&s).unwrap();
        let result = run_suite(&root, &s);
        assert!(result.passed, "{result:?}");
        let _ = std::fs::remove_dir_all(&root);
    }
}
