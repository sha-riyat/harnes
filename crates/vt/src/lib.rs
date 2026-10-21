//! Safe wrapper around libghostty-vt (through the `hv` C shim).
//!
//! A [`Terminal`] parses VT bytes from a PTY and produces [`Frame`]s:
//! a flat grid of [`Cell`]s plus cursor state, ready to draw.

use std::ffi::c_void;

mod ffi;

pub use ffi::{Cell, CursorState};

pub const BOLD: u16 = 1;
pub const ITALIC: u16 = 2;
pub const UNDERLINE: u16 = 4;
pub const STRIKE: u16 = 8;
pub const FAINT: u16 = 16;
pub const INVISIBLE: u16 = 32;

/// A rendered snapshot of the visible viewport.
#[derive(Debug, Clone, Default)]
pub struct Frame {
    pub cols: u16,
    pub rows: u16,
    pub cursor: CursorState,
    pub default_bg: u32,
    pub cells: Vec<Cell>,
}

impl Frame {
    /// Binary layout sent to the UI, little-endian: a 16-byte header
    /// (cols u16, rows u16, cursor x u16, y u16, visible u8, style u8, default_bg u32, pad u16)
    /// then 14 bytes per cell (cp u32, fg rgb 3B, bg rgb 3B, flags u16, pad u16).
    pub const HEADER: usize = 16;

    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(Self::HEADER + self.cells.len() * 14);
        out.extend_from_slice(&self.cols.to_le_bytes());
        out.extend_from_slice(&self.rows.to_le_bytes());
        out.extend_from_slice(&self.cursor.x.to_le_bytes());
        out.extend_from_slice(&self.cursor.y.to_le_bytes());
        out.push(self.cursor.visible);
        out.push(self.cursor.style);
        out.extend_from_slice(&self.default_bg.to_le_bytes());
        out.extend_from_slice(&[0, 0]);
        for c in &self.cells {
            out.extend_from_slice(&c.cp.to_le_bytes());
            out.extend_from_slice(&c.fg.to_le_bytes()[..3]);
            out.extend_from_slice(&c.bg.to_le_bytes()[..3]);
            out.extend_from_slice(&c.flags.to_le_bytes()[..2]);
            out.extend_from_slice(&[0, 0]);
        }
        out
    }

    /// Plain text of the frame, one line per row (trailing spaces trimmed).
    pub fn text(&self) -> String {
        let mut s = String::new();
        for row in self.cells.chunks(self.cols.max(1) as usize) {
            let line: String = row
                .iter()
                .map(|c| char::from_u32(c.cp).filter(|_| c.cp != 0).unwrap_or(' '))
                .collect();
            s.push_str(line.trim_end());
            s.push('\n');
        }
        s
    }
}

type Writer = Box<dyn FnMut(&[u8]) + Send>;

/// A VT terminal state machine backed by Ghostty.
pub struct Terminal {
    raw: *mut ffi::HvTerm,
    // Boxed twice so the pointer handed to C stays stable.
    writer: Box<Option<Writer>>,
    buf: Vec<Cell>,
}

// The terminal is only ever touched through `&mut self`.
unsafe impl Send for Terminal {}

extern "C" fn write_trampoline(ud: *mut c_void, data: *const u8, len: usize) {
    // SAFETY: `ud` points at the `Option<Writer>` owned by the Terminal, alive while the terminal is.
    let writer = unsafe { &mut *(ud as *mut Option<Writer>) };
    if let Some(w) = writer.as_mut() {
        w(unsafe { std::slice::from_raw_parts(data, len) });
    }
}

impl Terminal {
    pub fn new(cols: u16, rows: u16, max_scrollback: usize) -> Option<Self> {
        let raw = unsafe { ffi::hv_new(cols, rows, max_scrollback) };
        if raw.is_null() {
            return None;
        }
        let mut writer: Box<Option<Writer>> = Box::new(None);
        let ud = writer.as_mut() as *mut Option<Writer> as *mut c_void;
        unsafe { ffi::hv_set_writer(raw, write_trampoline, ud) };
        Some(Self {
            raw,
            writer,
            buf: Vec::new(),
        })
    }

    /// Bytes the terminal must send back to the PTY (query replies).
    pub fn on_reply(&mut self, f: impl FnMut(&[u8]) + Send + 'static) {
        *self.writer = Some(Box::new(f));
    }

    pub fn write(&mut self, data: &[u8]) {
        unsafe { ffi::hv_write(self.raw, data.as_ptr(), data.len()) }
    }

    pub fn resize(&mut self, cols: u16, rows: u16, cell_w: u32, cell_h: u32) -> bool {
        unsafe { ffi::hv_resize(self.raw, cols.max(1), rows.max(1), cell_w, cell_h) == 0 }
    }

    /// Negative scrolls back into history, 0 returns to the live bottom.
    pub fn scroll(&mut self, delta: isize) {
        unsafe { ffi::hv_scroll(self.raw, delta) }
    }

    /// Returns a frame when something changed (or always with `force`).
    pub fn snapshot(&mut self, force: bool) -> Option<Frame> {
        let (mut cols, mut rows, mut bg) = (0u16, 0u16, 0u32);
        let mut cursor = CursorState::default();
        loop {
            let cap = self.buf.capacity();
            let r = unsafe {
                ffi::hv_snapshot(
                    self.raw,
                    self.buf.as_mut_ptr(),
                    cap,
                    &mut cols,
                    &mut rows,
                    &mut cursor,
                    &mut bg,
                    force as i32,
                )
            };
            match r {
                1 => {
                    let n = cols as usize * rows as usize;
                    // SAFETY: the shim wrote exactly cols*rows cells.
                    unsafe { self.buf.set_len(n) };
                    return Some(Frame {
                        cols,
                        rows,
                        cursor,
                        default_bg: bg,
                        cells: self.buf.clone(),
                    });
                }
                -2 => self.buf.reserve(cols as usize * rows as usize),
                _ => return None,
            }
        }
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        unsafe { ffi::hv_free(self.raw) }
    }
}

#[cfg(test)]
mod tests;
