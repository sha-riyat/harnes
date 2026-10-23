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

/// Whether a process whose command line contains `needle` is running.
#[cfg(windows)]
fn running(needle: &str) -> bool {
    let script = format!("(Get-CimInstance Win32_Process | Where-Object {{ $_.CommandLine -like '*{needle}*' -and $_.Name -eq 'PING.EXE' }} | Measure-Object).Count");
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim() != "0"
}

#[cfg(windows)]
#[test]
fn closing_a_pane_kills_what_it_started() {
    let mgr = PaneManager::new(Arc::new(|_| {}));
    let info = mgr
        .spawn(SpawnSpec {
            program: "cmd.exe".into(),
            args: vec!["/c".into(), "ping -n 91 127.0.0.1 > nul".into()],
            cols: 80,
            rows: 10,
            ..Default::default()
        })
        .expect("spawn");
    let mut up = false;
    for _ in 0..20 {
        std::thread::sleep(Duration::from_millis(500));
        if running("-n 91") {
            up = true;
            break;
        }
    }
    assert!(up, "the grandchild never started");
    mgr.close(&info.id).unwrap();
    let mut gone = false;
    for _ in 0..20 {
        std::thread::sleep(Duration::from_millis(500));
        if !running("-n 91") {
            gone = true;
            break;
        }
    }
    assert!(gone, "the grandchild outlived its pane");
}
