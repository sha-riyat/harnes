use axum::{body::Bytes, routing::post, Router};
use harnes_proxy::{start, ProxyConfig};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};

/// Fake Anthropic: records the request body and streams back usage events.
async fn fake_upstream(seen: Arc<Mutex<Option<Value>>>) -> u16 {
    let app = Router::new().route(
        "/v1/messages",
        post(move |body: Bytes| {
            let seen = seen.clone();
            async move {
                *seen.lock().unwrap() = serde_json::from_slice(&body).ok();
                let sse = concat!(
                    "event: message_start\n",
                    "data: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":120,\"cache_read_input_tokens\":80,\"output_tokens\":1}}}\n\n",
                    "event: message_delta\n",
                    "data: {\"type\":\"message_delta\",\"usage\":{\"output_tokens\":42}}\n\n"
                );
                ([("content-type", "text/event-stream")], sse)
            }
        }),
    );
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = l.local_addr().unwrap().port();
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    port
}

#[tokio::test]
async fn compresses_and_counts_usage() {
    let seen = Arc::new(Mutex::new(None));
    let up = fake_upstream(seen.clone()).await;
    let store = Arc::new(harnes_ctx::Store::in_memory().unwrap());
    let proxy = start(ProxyConfig {
        port: 0,
        upstream: format!("http://127.0.0.1:{up}"),
        compress: true,
        store: Some(store.clone()),
        events: None,
    })
    .await
    .unwrap();

    let big: String = (0..4000).map(|i| format!("log line {i}\n")).collect();
    let req = json!({"model":"x","messages":[{"role":"user","content":[{"type":"tool_result","tool_use_id":"t","content":big}]}]});
    let resp = reqwest::Client::new()
        .post(format!("{}/v1/messages", proxy.url()))
        .header("x-api-key", "test")
        .json(&req)
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());
    let text = resp.text().await.unwrap();
    assert!(text.contains("message_delta"), "stream passed through");

    let got = seen.lock().unwrap().clone().expect("upstream saw request");
    let sent = got["messages"][0]["content"][0]["content"]
        .as_str()
        .unwrap();
    assert!(
        sent.len() < big.len() / 2,
        "tool_result compressed: {} -> {}",
        big.len(),
        sent.len()
    );

    let s = proxy.snapshot();
    assert_eq!(s.requests, 1);
    assert_eq!(s.elided_blocks, 1);
    assert_eq!(s.input_tokens, 120);
    assert_eq!(s.cache_read_tokens, 80);
    assert_eq!(s.output_tokens, 42);
    assert_eq!(store.stats().unwrap().blobs, 1);
}
