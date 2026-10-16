//! Pushes dirty pane frames to the UI at most ~60 times per second.
//!
//! Each message: 8-byte ASCII pane id (space padded) + encoded Ghostty frame.

use crate::state::Shared;
use std::time::Duration;
use tauri::ipc::InvokeResponseBody;

pub fn message(id: &str, frame: &harnes_pty::Frame) -> Vec<u8> {
    let mut out = format!("{id:<8}").into_bytes();
    out.truncate(8);
    out.extend_from_slice(&frame.encode());
    out
}

pub fn start(st: Shared) {
    tauri::async_runtime::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_millis(16));
        tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tick.tick().await;
            let ids: Vec<String> = st.dirty.lock().drain().collect();
            if ids.is_empty() {
                continue;
            }
            let Some(chan) = st.frames.lock().clone() else { continue };
            for id in ids {
                if let Ok(Some(frame)) = st.panes.snapshot(&id, false) {
                    let _ = chan.send(InvokeResponseBody::Raw(message(&id, &frame)));
                }
            }
        }
    });
}
