//! Starts the app under test (if needed) and stops it with its process tree.

use crate::spec::AppConfig;
use anyhow::{bail, Result};
use std::{
    net::{TcpStream, ToSocketAddrs},
    path::Path,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

pub struct AppProcess {
    child: Option<Child>,
}

/// `host:port` of an http(s) URL.
fn host_port(url: &str) -> Option<String> {
    let rest = url.split("://").nth(1)?;
    let authority = rest.split('/').next()?;
    if authority.contains(':') {
        Some(authority.to_string())
    } else {
        Some(format!(
            "{authority}:{}",
            if url.starts_with("https") { 443 } else { 80 }
        ))
    }
}

pub fn reachable(url: &str) -> bool {
    let Some(hp) = host_port(url) else {
        return false;
    };
    let Ok(mut addrs) = hp.to_socket_addrs() else {
        return false;
    };
    addrs.any(|a| TcpStream::connect_timeout(&a, Duration::from_millis(300)).is_ok())
}

impl AppProcess {
    pub fn start(cfg: &AppConfig, root: &Path) -> Result<Self> {
        if reachable(&cfg.url) {
            return Ok(Self { child: None });
        }
        let Some(cmd) = cfg.command.as_deref() else {
            bail!(
                "{} ne répond pas et aucune `app.command` n'est configurée",
                cfg.url
            );
        };
        let mut c = if cfg!(windows) {
            let mut c = Command::new("cmd");
            c.args(["/C", cmd]);
            c
        } else {
            let mut c = Command::new("sh");
            c.args(["-c", cmd]);
            c
        };
        let mut child = c
            .current_dir(root)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;
        let start = Instant::now();
        while !reachable(&cfg.url) {
            if let Some(status) = child.try_wait()? {
                bail!(
                    "`{cmd}` s'est arrêté ({status}) avant que {} réponde",
                    cfg.url
                );
            }
            if start.elapsed() > Duration::from_millis(cfg.ready_timeout_ms) {
                let _ = child.kill();
                bail!(
                    "{} ne répond pas après {} ms",
                    cfg.url,
                    cfg.ready_timeout_ms
                );
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        Ok(Self { child: Some(child) })
    }

    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            if cfg!(windows) {
                // Kill the whole tree: cmd.exe -> node/npm -> ...
                let _ = Command::new("taskkill")
                    .args(["/T", "/F", "/PID", &child.id().to_string()])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status();
            }
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for AppProcess {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_host_port() {
        assert_eq!(
            host_port("http://localhost:4173/x").as_deref(),
            Some("localhost:4173")
        );
        assert_eq!(
            host_port("https://example.com").as_deref(),
            Some("example.com:443")
        );
    }
}
