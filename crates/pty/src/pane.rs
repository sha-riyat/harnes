use crate::{EventSink, PaneEvent};
use anyhow::{Context, Result};
use harnes_vt::{Frame, Terminal};
use parking_lot::Mutex;
use portable_pty::{native_pty_system, Child, ChildKiller, CommandBuilder, MasterPty, PtySize};
use serde::Serialize;
use std::{
    io::{Read, Write},
    path::PathBuf,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

/// What to run in a new pane.
#[derive(Debug, Clone, Default)]
pub struct SpawnSpec {
    pub title: String,
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: Vec<(String, String)>,
    pub cols: u16,
    pub rows: u16,
}

#[derive(Debug, Clone, Serialize)]
pub struct PaneInfo {
    pub id: String,
    pub title: String,
    pub program: String,
    pub cwd: String,
    pub created: u64,
    pub pid: Option<u32>,
}

pub struct Pane {
    info: PaneInfo,
    term: Arc<Mutex<Terminal>>,
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    master: Mutex<Box<dyn MasterPty + Send>>,
    killer: Mutex<Box<dyn ChildKiller + Send + Sync>>,
}

impl Pane {
    pub fn spawn(spec: SpawnSpec, sink: EventSink) -> Result<Arc<Self>> {
        let (cols, rows) = (spec.cols.max(20), spec.rows.max(5));
        let pair = native_pty_system()
            .openpty(PtySize { rows, cols, pixel_width: 0, pixel_height: 0 })
            .context("openpty")?;

        let mut cmd = CommandBuilder::new(&spec.program);
        cmd.args(&spec.args);
        if let Some(cwd) = &spec.cwd {
            cmd.cwd(cwd);
        }
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        for (k, v) in &spec.env {
            cmd.env(k, v);
        }
        let child = pair.slave.spawn_command(cmd).with_context(|| format!("spawn {}", spec.program))?;
        drop(pair.slave);

        let reader = pair.master.try_clone_reader()?;
        let writer: Arc<Mutex<Box<dyn Write + Send>>> = Arc::new(Mutex::new(pair.master.take_writer()?));

        let mut term = Terminal::new(cols, rows, 10_000).context("ghostty terminal")?;
        let reply_writer = writer.clone();
        term.on_reply(move |b| {
            let _ = reply_writer.lock().write_all(b);
        });

        let id = uuid::Uuid::new_v4().to_string()[..8].to_string();
        let created = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
        let info = PaneInfo {
            id: id.clone(),
            title: if spec.title.is_empty() { spec.program.clone() } else { spec.title.clone() },
            program: spec.program.clone(),
            cwd: spec.cwd.as_ref().map(|p| p.display().to_string()).unwrap_or_default(),
            created,
            pid: child.process_id(),
        };
        let pane = Arc::new(Self {
            info,
            term: Arc::new(Mutex::new(term)),
            writer,
            master: Mutex::new(pair.master),
            killer: Mutex::new(child.clone_killer()),
        });
        pane.start_reader(reader, sink.clone());
        pane.start_waiter(child, sink);
        Ok(pane)
    }

    fn start_reader(self: &Arc<Self>, mut reader: Box<dyn Read + Send>, sink: EventSink) {
        let term = self.term.clone();
        let id = self.info.id.clone();
        std::thread::Builder::new()
            .name(format!("pane-{id}"))
            .spawn(move || {
                let mut buf = [0u8; 16 * 1024];
                loop {
                    match reader.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            term.lock().write(&buf[..n]);
                            sink(PaneEvent::Output(id.clone(), buf[..n].to_vec()));
                            sink(PaneEvent::Dirty(id.clone()));
                        }
                    }
                }
            })
            .expect("spawn reader thread");
    }

    // ConPTY keeps the output pipe open after the child exits, so exit is
    // detected by waiting on the process rather than on reader EOF.
    fn start_waiter(&self, mut child: Box<dyn Child + Send + Sync>, sink: EventSink) {
        let id = self.info.id.clone();
        std::thread::Builder::new()
            .name(format!("wait-{id}"))
            .spawn(move || {
                let code = child.wait().ok().map(|s| s.exit_code());
                // Let the reader drain the last output before announcing exit.
                std::thread::sleep(std::time::Duration::from_millis(150));
                sink(PaneEvent::Exited(id, code));
            })
            .expect("spawn waiter thread");
    }

    pub fn info(&self) -> PaneInfo {
        self.info.clone()
    }

    pub fn write(&self, data: &[u8]) -> Result<()> {
        let mut w = self.writer.lock();
        w.write_all(data)?;
        w.flush()?;
        Ok(())
    }

    pub fn resize(&self, cols: u16, rows: u16, cell_w: u32, cell_h: u32) -> Result<()> {
        let (cols, rows) = (cols.max(2), rows.max(1));
        self.master.lock().resize(PtySize {
            rows,
            cols,
            pixel_width: (cols as u32 * cell_w) as u16,
            pixel_height: (rows as u32 * cell_h) as u16,
        })?;
        self.term.lock().resize(cols, rows, cell_w, cell_h);
        Ok(())
    }

    pub fn scroll(&self, delta: isize) {
        self.term.lock().scroll(delta);
    }

    pub fn snapshot(&self, force: bool) -> Option<Frame> {
        self.term.lock().snapshot(force)
    }

    pub fn kill(&self) {
        let _ = self.killer.lock().kill();
    }
}
