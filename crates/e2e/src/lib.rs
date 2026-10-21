//! harnes-e2e: agentic end-to-end testing for harnes.
//!
//! Tests are declarative YAML specs mixing exact steps (`open`, `click`,
//! `fill`, `press`, `expect`) with natural-language `act` goals. An `act`
//! goal is resolved once by a Claude Code agent driving the real browser
//! through the MCP server; the concrete actions are cached and replayed on
//! every later run without any model call. Inspired by TesterArmy e2e
//! (Apache-2.0); this is an independent implementation.

pub mod app;
pub mod browser;
pub mod cache;
pub mod mcp;
pub mod report;
pub mod runner;
pub mod spec;

pub use report::Report;
pub use runner::{run, RunOptions};

/// Starter files for `harnes-e2e init`.
pub fn init(
    root: &std::path::Path,
    url: &str,
    command: Option<&str>,
) -> anyhow::Result<Vec<String>> {
    let mut written = vec![];
    let cfg = root.join(spec::CONFIG_FILE);
    if !cfg.exists() {
        let cmd = command
            .map(|c| format!("  command: {c}\n"))
            .unwrap_or_else(|| "  # command: npm run dev\n".into());
        std::fs::write(&cfg, format!("app:\n  url: {url}\n{cmd}timeout_ms: 5000\n"))?;
        written.push(spec::CONFIG_FILE.to_string());
    }
    let smoke = root.join("tests").join("smoke.e2e.yaml");
    if !smoke.exists() {
        std::fs::create_dir_all(smoke.parent().expect("tests dir"))?;
        std::fs::write(
            &smoke,
            "name: l'app s'ouvre\nsteps:\n  - open: /\n  - expect: { target: { css: body } }\n",
        )?;
        written.push("tests/smoke.e2e.yaml".into());
    }
    Ok(written)
}
