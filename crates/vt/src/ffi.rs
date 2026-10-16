//! Raw bindings to `shim/hv.h`. Keep in sync with the C header.

use std::ffi::c_void;

#[repr(C)]
pub struct HvTerm {
    _private: [u8; 0],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Cell {
    /// First codepoint, 0 = empty.
    pub cp: u32,
    /// 0x00RRGGBB.
    pub fg: u32,
    pub bg: u32,
    pub flags: u16,
    _pad: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CursorState {
    pub x: u16,
    pub y: u16,
    pub visible: u8,
    /// 0 block, 1 bar, 2 underline, 3 hollow.
    pub style: u8,
}

pub type HvWritePty = extern "C" fn(*mut c_void, *const u8, usize);

extern "C" {
    pub fn hv_new(cols: u16, rows: u16, max_scrollback: usize) -> *mut HvTerm;
    pub fn hv_free(t: *mut HvTerm);
    pub fn hv_set_writer(t: *mut HvTerm, cb: HvWritePty, ud: *mut c_void);
    pub fn hv_write(t: *mut HvTerm, data: *const u8, len: usize);
    pub fn hv_resize(t: *mut HvTerm, cols: u16, rows: u16, cell_w: u32, cell_h: u32) -> i32;
    pub fn hv_scroll(t: *mut HvTerm, delta: isize);
    pub fn hv_snapshot(
        t: *mut HvTerm,
        out: *mut Cell,
        cap: usize,
        cols: *mut u16,
        rows: *mut u16,
        cursor: *mut CursorState,
        default_bg: *mut u32,
        force: i32,
    ) -> i32;
}
