use super::*;
use std::{sync::mpsc, time::Duration};

#[test]
fn runs_a_command_and_renders_output() {
    let (tx, rx) = mpsc::channel();
    let tx = std::sync::Mutex::new(tx);
    let mgr = PaneManager::new(Arc::new(move |e| {
        if let PaneEvent::Exited(..) = e {
            let _ = tx.lock().unwrap().send(());
        }
    }));
    let (program, args) = if cfg!(windows) {
        (
            "cmd.exe",
            vec!["/c".to_string(), "echo harnes-ok".to_string()],
        )
    } else {
        ("sh", vec!["-c".to_string(), "echo harnes-ok".to_string()])
    };
    let info = mgr
        .spawn(SpawnSpec {
            program: program.into(),
            args,
            cols: 80,
            rows: 10,
            ..Default::default()
        })
        .expect("spawn");
    rx.recv_timeout(Duration::from_secs(15))
        .expect("child exit");
    let frame = mgr.snapshot(&info.id, true).unwrap().unwrap();
    assert!(frame.text().contains("harnes-ok"), "got:\n{}", frame.text());
    mgr.close_all();
}
