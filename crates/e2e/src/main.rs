//! harnes-e2e CLI.
//!
//!   harnes-e2e run [files…] [--root DIR] [--headed] [--json]
//!   harnes-e2e mcp [--root DIR]
//!   harnes-e2e init --url URL [--command CMD] [--root DIR]

use std::{env, path::PathBuf, process::ExitCode};

#[tokio::main]
async fn main() -> ExitCode {
    let mut args: Vec<String> = env::args().skip(1).collect();
    let cmd = if args.is_empty() { "help".to_string() } else { args.remove(0) };
    let mut root = env::current_dir().unwrap_or_default();
    let (mut headed, mut json, mut url, mut command) = (false, false, None, None);
    let mut files = vec![];
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        match a.as_str() {
            "--root" => root = it.next().map(PathBuf::from).unwrap_or(root),
            "--headed" => headed = true,
            "--json" => json = true,
            "--url" => url = it.next(),
            "--command" => command = it.next(),
            f => files.push(PathBuf::from(f)),
        }
    }
    match cmd.as_str() {
        "run" => {
            let r = harnes_e2e::run(&root, harnes_e2e::RunOptions { files, headed }).await;
            if json {
                println!("{}", serde_json::to_string_pretty(&r).unwrap_or_default());
            } else {
                println!("{}", r.describe());
                for u in &r.unresolved {
                    println!("à résoudre : {} étape {} « {} »", u.file, u.step, u.act);
                }
            }
            ExitCode::from(r.exit_code as u8)
        }
        "mcp" => match harnes_e2e::mcp::Server::new(root).serve().await {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("{e:#}");
                ExitCode::from(4)
            }
        },
        "init" => match harnes_e2e::init(&root, url.as_deref().unwrap_or("http://localhost:3000"), command.as_deref()) {
            Ok(w) => {
                println!("écrit : {}", if w.is_empty() { "rien (déjà présent)".into() } else { w.join(", ") });
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("{e:#}");
                ExitCode::from(2)
            }
        },
        "--version" | "version" => {
            println!("harnes-e2e {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        _ => {
            println!("harnes-e2e run [files…] [--root DIR] [--headed] [--json]\nharnes-e2e mcp [--root DIR]\nharnes-e2e init --url URL [--command CMD]");
            ExitCode::from(2)
        }
    }
}
