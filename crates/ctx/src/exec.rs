//! Runs code snippets in a child process with a timeout and an output cap.
//! This isolates output from the model context; it is not a security sandbox.

use anyhow::{bail, Result};
use std::{
    io::Read,
    path::Path,
    process::{Command, Stdio},
    thread,
    time::{Duration, Instant},
};

const MAX_OUTPUT: usize = 32 * 1024 * 1024;

#[derive(Debug, Default)]
pub struct Output {
    pub stdout: String,
    pub stderr: String,
    pub code: Option<i32>,
    pub timed_out: bool,
    pub millis: u128,
}

impl Output {
    pub fn combined(&self) -> String {
        match (self.stdout.is_empty(), self.stderr.is_empty()) {
            (_, true) => self.stdout.clone(),
            (true, false) => self.stderr.clone(),
            (false, false) => format!("{}\n--- stderr ---\n{}", self.stdout, self.stderr),
        }
    }
}

fn command_for(language: &str, code: &str) -> Result<Command> {
    let mut c = match language {
        "shell" | "bash" | "sh" => {
            if cfg!(windows) {
                let mut c = Command::new("bash");
                c.args(["-lc", code]);
                c
            } else {
                let mut c = Command::new("sh");
                c.args(["-c", code]);
                c
            }
        }
        "powershell" | "pwsh" => {
            let mut c = Command::new(if cfg!(windows) { "powershell" } else { "pwsh" });
            c.args(["-NoProfile", "-NonInteractive", "-Command", code]);
            c
        }
        "cmd" => {
            let mut c = Command::new("cmd");
            c.args(["/C", code]);
            c
        }
        "python" => {
            let mut c = Command::new("python");
            c.args(["-c", code]);
            c
        }
        "node" | "javascript" | "js" => {
            let mut c = Command::new("node");
            c.args(["-e", code]);
            c
        }
        other => bail!("unsupported language: {other} (use shell, powershell, cmd, python or node)"),
    };
    c.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    Ok(c)
}

fn drain(mut r: impl Read + Send + 'static) -> thread::JoinHandle<String> {
    thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = r.by_ref().take(MAX_OUTPUT as u64).read_to_end(&mut buf);
        String::from_utf8_lossy(&buf).into_owned()
    })
}

pub fn run(language: &str, code: &str, cwd: &Path, timeout: Duration) -> Result<Output> {
    let start = Instant::now();
    let mut child = command_for(language, code)?.current_dir(cwd).spawn()?;
    let out = drain(child.stdout.take().expect("stdout piped"));
    let err = drain(child.stderr.take().expect("stderr piped"));

    let mut timed_out = false;
    let status = loop {
        if let Some(s) = child.try_wait()? {
            break Some(s);
        }
        if start.elapsed() > timeout {
            let _ = child.kill();
            timed_out = true;
            break child.wait().ok();
        }
        thread::sleep(Duration::from_millis(20));
    };
    Ok(Output {
        stdout: out.join().unwrap_or_default(),
        stderr: err.join().unwrap_or_default(),
        code: status.and_then(|s| s.code()),
        timed_out,
        millis: start.elapsed().as_millis(),
    })
}
