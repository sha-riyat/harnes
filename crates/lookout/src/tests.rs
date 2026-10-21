use super::*;
use crate::model::{Session, Status};
use std::sync::{Arc, Mutex};

struct Fake(Arc<Mutex<Vec<Session>>>);

impl Adapter for Fake {
    fn id(&self) -> &'static str {
        "fake"
    }
    fn poll(&self) -> anyhow::Result<Option<Vec<Session>>> {
        Ok(Some(self.0.lock().unwrap().clone()))
    }
}

fn s(id: &str, st: Status) -> Session {
    let mut s = Session::new("fake", "claude", id, "C:/work/proj");
    s.name = id.into();
    s.status = st;
    s
}

#[test]
fn diffs_into_events_and_sorts_needs_you_first() {
    let data = Arc::new(Mutex::new(vec![
        s("a", Status::Working),
        s("b", Status::Idle),
    ]));
    let mut c = Collector::new(vec![Box::new(Fake(data.clone()))]);
    let ev = c.poll(vec![]);
    assert_eq!(
        ev.iter().filter(|e| e.kind == EventKind::Appeared).count(),
        2
    );

    *data.lock().unwrap() = vec![s("a", Status::NeedsYou)];
    let ev = c.poll(vec![s("harnes-x", Status::Working)]);
    assert!(ev
        .iter()
        .any(|e| e.kind == EventKind::StatusChanged && e.to == Some(Status::NeedsYou)));
    assert!(ev
        .iter()
        .any(|e| e.kind == EventKind::Ended && e.session_id == "fake:b"));
    assert!(ev
        .iter()
        .any(|e| e.kind == EventKind::Appeared && e.session_id == "fake:harnes-x"));

    let snap = c.snapshot();
    assert_eq!(snap.sessions[0].status, Status::NeedsYou);
    assert_eq!(snap.sessions[0].project, "proj");
    assert_eq!(c.history().last().unwrap().needs_you, 1);
}

#[test]
fn real_sources_do_not_panic() {
    let mut c = Collector::default();
    c.poll(vec![]);
    assert_eq!(c.snapshot().sources.len(), 4);
}
