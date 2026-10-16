use super::*;
use std::sync::{Arc, Mutex};

#[test]
fn renders_text_and_styles() {
    let mut t = Terminal::new(20, 3, 1000).expect("terminal");
    t.write(b"Hello, \x1b[1;32mworld\x1b[0m!\r\nline2");
    let f = t.snapshot(false).expect("dirty frame");
    assert_eq!((f.cols, f.rows), (20, 3));
    assert_eq!(f.text().lines().next(), Some("Hello, world!"));
    let w = &f.cells[7];
    assert_eq!(char::from_u32(w.cp), Some('w'));
    assert!(w.flags & BOLD != 0);
    assert_eq!((f.cursor.x, f.cursor.y), (5, 1));
    assert!(t.snapshot(false).is_none(), "clean after snapshot");
}

#[test]
fn resizes() {
    let mut t = Terminal::new(10, 2, 100).unwrap();
    assert!(t.resize(40, 10, 8, 16));
    let f = t.snapshot(true).unwrap();
    assert_eq!((f.cols, f.rows), (40, 10));
    assert_eq!(f.encode().len(), Frame::HEADER + 400 * 14);
}

#[test]
fn replies_to_device_attributes() {
    let mut t = Terminal::new(10, 2, 100).unwrap();
    let got = Arc::new(Mutex::new(Vec::new()));
    let sink = got.clone();
    t.on_reply(move |b| sink.lock().unwrap().extend_from_slice(b));
    t.write(b"\x1b[6n"); // cursor position report
    assert!(!got.lock().unwrap().is_empty(), "expected a DSR reply");
}
