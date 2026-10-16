//! `harnes-ctx [--root DIR] [--db FILE]` — MCP server on stdio.

use harnes_ctx::{mcp, store::Store, tools::Ctx};
use std::{env, io, path::PathBuf};

fn main() -> anyhow::Result<()> {
    let mut root = env::current_dir()?;
    let mut db: Option<PathBuf> = None;
    let mut args = env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--root" => root = args.next().map(PathBuf::from).unwrap_or(root),
            "--db" => db = args.next().map(PathBuf::from),
            "--version" => {
                println!("harnes-ctx {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            _ => {}
        }
    }
    let db = db.unwrap_or_else(|| root.join(".harnes").join("ctx.db"));
    let threads = harnes_threads::Threads::open(&root.join(".harnes").join("threads.db")).ok();
    let ctx = Ctx { store: Store::open(&db)?, root, threads };
    mcp::serve(&ctx, io::stdin().lock(), io::stdout().lock())
}
