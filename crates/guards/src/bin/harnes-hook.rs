//! `harnes-hook`: a small PreToolUse guard for Claude Code and other CLIs.
//!
//! Reads one hook payload on stdin and writes a Claude-compatible decision on
//! stdout. Root comes from `--root`, `HARNES_PROJECT_ROOT`, else the current
//! directory. `--forward <url>` reports the payload to the cockpit, best effort.
//! Exit code 0 always: a guard failure must not brick the agent session.

fn main() {
    std::process::exit(harnes_guards::hook_main());
}
