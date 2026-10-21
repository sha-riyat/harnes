//! End-to-end gate backed by `harnes-e2e` (native component, no API key).
//!
//! Agents write `tests/**/*.e2e.yaml` and resolve natural-language `act`
//! steps through the harnes-e2e MCP server. Before each review the engine
//! runs the whole suite itself so the Reviewer judges real results.

use harnes_e2e::{spec::CONFIG_FILE, Report, RunOptions};
use std::path::Path;

/// `harnes-e2e.yaml` present: the project has an e2e suite.
pub fn configured(root: &Path) -> bool {
    root.join(CONFIG_FILE).exists()
}

/// Runs every spec. Exit codes: 0 pass, 1 failure, 2 config, 3 environment, 5 act steps to resolve.
pub async fn run(root: &Path) -> Report {
    harnes_e2e::run(
        root,
        RunOptions {
            files: vec![],
            headed: false,
        },
    )
    .await
}
