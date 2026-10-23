//! `ask_user` end to end over the real hook server: the agent's request blocks,
//! the agent shows as blocked on the user, and the answer comes back as JSON.

use harnes_crew::{ask::Answer, hooks::HookServer, AgentState};
use serde_json::{json, Value};
use std::{
    io::{Read, Write},
    sync::Arc,
    time::Duration,
};

fn post(port: u16, path: &str, body: &Value) -> String {
    let mut s = std::net::TcpStream::connect(("127.0.0.1", port)).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(20))).unwrap();
    let b = body.to_string();
    write!(s, "POST {path} HTTP/1.1\r\nHost: x\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{b}", b.len()).unwrap();
    let mut raw = String::new();
    s.read_to_string(&mut raw).unwrap();
    raw.split_once("\r\n\r\n")
        .map(|(_, p)| p.to_string())
        .unwrap_or_default()
}

#[tokio::test]
async fn an_agent_asks_blocks_and_gets_the_answer() {
    let hooks = Arc::new(HookServer::start(Arc::new(|_: &str, _| {})).await.unwrap());
    let mut rx = hooks.registry.register("17-worker-L1-r0");
    let port = hooks.port;
    let card = json!({"kind": "choice", "question": "SMTP or API?", "options": [{"id": "smtp", "label": "SMTP"}, {"id": "api", "label": "API"}]});
    let asking = tokio::task::spawn_blocking(move || post(port, "/ask/17-worker-L1-r0", &card));

    // Wait for the question to show up, then check the agent is blocked on the user.
    let id = loop {
        if let Some(p) = hooks.asks.pending().first() {
            break p.id.clone();
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    };
    assert_eq!(hooks.asks.pending()[0].agent, "17-worker-L1-r0");
    rx.changed().await.unwrap();
    assert_eq!(rx.borrow().state, AgentState::Blocked);

    hooks
        .asks
        .answer(
            &id,
            Answer {
                choice: Some("smtp".into()),
                text: None,
            },
        )
        .unwrap();
    let reply: Value = serde_json::from_str(&asking.await.unwrap()).unwrap();
    assert_eq!(
        reply,
        json!({"answered": true, "choice": "smtp", "text": null})
    );
    rx.changed().await.unwrap();
    assert_eq!(rx.borrow().state, AgentState::Working, "back to work");

    // A card the registry refuses comes back as an error the agent can read.
    let bad = tokio::task::spawn_blocking(move || {
        post(
            port,
            "/ask/17-worker-L1-r0",
            &json!({"kind": "slider", "question": "x"}),
        )
    })
    .await
    .unwrap();
    assert!(serde_json::from_str::<Value>(&bad).unwrap()["error"]
        .as_str()
        .unwrap()
        .contains("inconnu"));
    assert!(hooks.asks.pending().is_empty());
}
