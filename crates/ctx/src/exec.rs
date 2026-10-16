//! Runs code snippets in a child process with a timeout and an output cap.
//! This isolates output from the model context; it is not a security sandbox.

use anyhow::{bail, Result};
use parking_lot::Mutex;
use std::{
    io::Read,
    path::Path,
    process::{Child, Command, Stdio},
    sync::Arc,
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

/// Reads a pipe into a shared buffer, so what was read so far is still there
/// if a grandchild keeps the pipe open and the reader never sees EOF.
fn drain(mut r: impl Read + Send + 'static) -> (Arc<Mutex<Vec<u8>>>, thread::JoinHandle<()>) {
    let buf = Arc::new(Mutex::new(Vec::new()));
    let b2 = buf.clone();
    let handle = thread::spawn(move || {
        let mut chunk = [0u8; 8192];
        loop {
            match r.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    let mut b = b2.lock();
                    let room = MAX_OUTPUT.saturating_sub(b.len());
                    b.extend_from_slice(&chunk[..n.min(room)]);
                }
            }
        }
    });
    (buf, handle)
}

/// Kills the process and everything it started. `Child::kill` alone leaves
/// grandchildren (npm, node, a dev server...) running with our pipes open.
fn kill_tree(child: &mut Child) {
    #[cfg(windows)]
    {
        let _ = Command::new("taskkill")
            .args(["/T", "/F", "/PID", &child.id().to_string()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    #[cfg(unix)]
    {
        // The child leads its own process group (see `run`).
        let _ = Command::new("kill")
            .args(["-KILL", "--", &format!("-{}", child.id())])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
    let _ = child.kill();
}

/// How long to keep reading after the process ended, for output still in the
/// pipes. A background process that inherited them is not waited for.
const DRAIN_GRACE: Duration = Duration::from_secs(2);

pub fn run(language: &str, code: &str, cwd: &Path, timeout: Duration) -> Result<Output> {
    let start = Instant::now();
    let mut cmd = command_for(language, code)?;
    cmd.current_dir(cwd);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    let mut child = cmd.spawn()?;
    let (out, out_reader) = drain(child.stdout.take().expect("stdout piped"));
    let (err, err_reader) = drain(child.stderr.take().expect("stderr piped"));

    let mut timed_out = false;
    let status = loop {
        if let Some(s) = child.try_wait()? {
            break Some(s);
        }
        if start.elapsed() > timeout {
            kill_tree(&mut child);
            timed_out = true;
            break child.wait().ok();
        }
        thread::sleep(Duration::from_millis(20));
    };
    let ended = Instant::now();
    while !(out_reader.is_finished() && err_reader.is_finished()) && ended.elapsed() < DRAIN_GRACE {
        thread::sleep(Duration::from_millis(10));
    }
    let text = |b: &Arc<Mutex<Vec<u8>>>| String::from_utf8_lossy(&b.lock()).into_owned();
    Ok(Output {
        stdout: text(&out),
        stderr: text(&err),
        code: status.and_then(|s| s.code()),
        timed_out,
        millis: start.elapsed().as_millis(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// node is used because it behaves the same on every platform.
    fn node_available() -> bool {
        Command::new("node").arg("--version").stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok()
    }

    /// A parent that starts a long-lived child sharing its stdout.
    const SPAWNS_SLEEPER: &str = "require('child_process').spawn(process.execPath, ['-e', 'setTimeout(() => {}, 30000)'], { stdio: 'inherit' }).unref(); console.log('started');";

    #[test]
    fn returns_when_a_background_child_keeps_the_pipe_open() {
        if !node_available() {
            return;
        }
        let t = Instant::now();
        let out = run("node", SPAWNS_SLEEPER, Path::new("."), Duration::from_secs(20)).unwrap();
        assert!(!out.timed_out);
        assert!(out.stdout.contains("started"));
        assert!(t.elapsed() < Duration::from_secs(10), "waited for the grandchild: {:?}", t.elapsed());
    }

    #[test]
    fn timeout_kills_the_whole_tree() {
        if !node_available() {
            return;
        }
        let code = format!("{SPAWNS_SLEEPER} setTimeout(() => {{}}, 30000);");
        let t = Instant::now();
        let out = run("node", &code, Path::new("."), Duration::from_secs(2)).unwrap();
        assert!(out.timed_out);
        assert!(out.stdout.contains("started"), "output read before the kill is kept");
        assert!(t.elapsed() < Duration::from_secs(10), "timeout not enforced: {:?}", t.elapsed());
    }
}
