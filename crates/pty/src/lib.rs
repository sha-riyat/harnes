//! Terminal panes: a child process on a PTY whose output feeds a Ghostty VT.

mod pane;

pub use harnes_vt::Frame;
pub use pane::{Pane, PaneInfo, SpawnSpec};

use anyhow::{anyhow, Result};
use parking_lot::RwLock;
use std::{collections::HashMap, sync::Arc};

/// Events emitted by panes. Delivered from reader threads.
#[derive(Debug, Clone)]
pub enum PaneEvent {
    /// New output arrived; a fresh frame can be pulled.
    Dirty(String),
    /// Raw output bytes, for observers (logs, detection).
    Output(String, Vec<u8>),
    /// The child process exited.
    Exited(String, Option<u32>),
}

pub type EventSink = Arc<dyn Fn(PaneEvent) + Send + Sync>;

/// Owns every live pane.
pub struct PaneManager {
    panes: RwLock<HashMap<String, Arc<Pane>>>,
    sink: EventSink,
}

impl PaneManager {
    pub fn new(sink: EventSink) -> Self {
        Self {
            panes: RwLock::new(HashMap::new()),
            sink,
        }
    }

    pub fn spawn(&self, spec: SpawnSpec) -> Result<PaneInfo> {
        let pane = Pane::spawn(spec, self.sink.clone())?;
        let info = pane.info();
        self.panes.write().insert(info.id.clone(), pane);
        Ok(info)
    }

    pub fn get(&self, id: &str) -> Result<Arc<Pane>> {
        self.panes
            .read()
            .get(id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown pane {id}"))
    }

    pub fn list(&self) -> Vec<PaneInfo> {
        let mut v: Vec<_> = self.panes.read().values().map(|p| p.info()).collect();
        v.sort_by(|a, b| a.created.cmp(&b.created));
        v
    }

    pub fn write(&self, id: &str, data: &[u8]) -> Result<()> {
        self.get(id)?.write(data)
    }

    pub fn resize(&self, id: &str, cols: u16, rows: u16, cell_w: u32, cell_h: u32) -> Result<()> {
        self.get(id)?.resize(cols, rows, cell_w, cell_h)
    }

    pub fn snapshot(&self, id: &str, force: bool) -> Result<Option<Frame>> {
        Ok(self.get(id)?.snapshot(force))
    }

    pub fn close(&self, id: &str) -> Result<()> {
        if let Some(p) = self.panes.write().remove(id) {
            p.kill();
        }
        Ok(())
    }

    pub fn close_all(&self) {
        for (_, p) in self.panes.write().drain() {
            p.kill();
        }
    }
}

#[cfg(test)]
mod tests;
