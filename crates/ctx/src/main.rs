//! `harnes-ctx [--root DIR] [--db FILE]` — MCP server on stdio.
//! `harnes-ctx os-import PATH` — import OS packs (zip, folder of zips, folders).

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
            "os-import" => {
                let src = args
                    .next()
                    .map(PathBuf::from)
                    .ok_or_else(|| anyhow::anyhow!("usage: harnes-ctx os-import PATH"))?;
                let lib = harnes_os::Library::open_default();
                let ids = harnes_os::import(&lib, &src)?;
                println!(
                    "{} pack(s) in {}: {}",
                    ids.len(),
                    lib.dir.display(),
                    ids.join(", ")
                );
                return Ok(());
            }
            "--version" => {
                println!("harnes-ctx {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            _ => {}
        }
    }
    let db = db.unwrap_or_else(|| root.join(".harnes").join("ctx.db"));
    let threads = harnes_threads::Threads::open(&root.join(".harnes").join("threads.db")).ok();
    let project = root
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project")
        .to_string();
    let memory = harnes_memory::Memory::open(&root, &project).ok();
    let ctx = Ctx {
        store: Store::open(&db)?,
        root,
        threads,
        os: harnes_os::Library::open_default(),
        memory,
    };
    mcp::serve(&ctx, io::stdin().lock(), io::stdout().lock())
}
