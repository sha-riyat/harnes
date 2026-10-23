//! harnes-ctx: a context sandbox for coding agents.
//!
//! Large tool outputs (command runs, file reads, logs) are stored and indexed
//! in SQLite FTS5 instead of being pasted into the model context. The agent
//! gets a short summary plus a handle it can search or page through.

pub mod ask;
pub mod digest;
pub mod exec;
pub mod mcp;
pub mod memory;
pub mod os;
pub mod store;
pub mod summary;
pub mod threads;
pub mod tools;

pub use store::{Hit, Stats, Store};

/// Outputs at or below this size are returned inline.
pub const INLINE_LIMIT: usize = 4 * 1024;
