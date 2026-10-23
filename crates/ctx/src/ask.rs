//! `ask_user`: a typed question to the person using harnes.
//!
//! The agent states what it needs as JSON (a choice, a yes or no, a free
//! answer); the cockpit draws it and sends the answer back. The model never
//! describes the interface, and a kind that is not registered is refused. The
//! call blocks until the user answers, declines, or the time is up.

use anyhow::{anyhow, bail, Result};
use serde_json::{json, Value};
use std::io::{Read, Write};

pub fn available() -> bool {
    std::env::var_os("HARNES_API").is_some() && std::env::var_os("HARNES_AGENT").is_some()
}

pub fn definition() -> Value {
    json!({
        "name": "ask_user",
        "description": "Ask the user one question and wait for the answer. Use it only when you are blocked on a decision that is theirs (a product choice, a risky action, a missing fact) and cannot be settled from the code or the plan; otherwise decide and say so in your report. One question at a time. Kinds: `choice` (2 to 8 options, each {id, label, detail?}), `confirm` (yes or no), `text` (a free answer). Give a `default` that stands if they do not answer in time. The result is {answered, choice, text} or {answered: false, reason, default}.",
        "inputSchema": {"type": "object", "additionalProperties": false, "properties": {
            "kind": {"type": "string", "enum": ["choice", "confirm", "text"]},
            "question": {"type": "string", "maxLength": 500},
            "options": {"type": "array", "maxItems": 8, "items": {"type": "object", "additionalProperties": false,
                "properties": {"id": {"type": "string"}, "label": {"type": "string", "maxLength": 80}, "detail": {"type": "string", "maxLength": 200}},
                "required": ["id", "label"]}},
            "default": {"type": "string", "description": "An option id, yes or no, or the text that stands if there is no answer."},
            "timeout_secs": {"type": "integer", "default": 300, "maximum": 540}
        }, "required": ["kind", "question"]}
    })
}

/// Blocking POST to the harnes hook server for this agent.
fn post(base: &str, path: &str, body: &Value) -> Result<String> {
    let host = base.trim_start_matches("http://").trim_end_matches('/');
    let mut stream = std::net::TcpStream::connect(host)?;
    // The server waits at most 540 s for the user; leave it a margin.
    stream.set_read_timeout(Some(std::time::Duration::from_secs(600)))?;
    let b = body.to_string();
    write!(stream, "POST {path} HTTP/1.1\r\nHost: {host}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{b}", b.len())?;
    let mut raw = String::new();
    stream.read_to_string(&mut raw)?;
    let (head, payload) = raw.split_once("\r\n\r\n").unwrap_or((&raw, ""));
    if !head.starts_with("HTTP/1.1 200") {
        bail!("harnes a répondu : {}", head.lines().next().unwrap_or("?"));
    }
    Ok(payload.to_string())
}

pub fn call(args: &Value) -> Result<String> {
    let (Ok(base), Ok(agent)) = (std::env::var("HARNES_API"), std::env::var("HARNES_AGENT")) else {
        bail!("ask_user est réservé aux agents lancés par harnes");
    };
    ask(&base, &agent, args)
}

fn ask(base: &str, agent: &str, args: &Value) -> Result<String> {
    if !agent
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        bail!("identité d'agent invalide");
    }
    let reply = post(base, &format!("/ask/{agent}"), args)?;
    let v: Value =
        serde_json::from_str(&reply).map_err(|_| anyhow!("réponse illisible de harnes"))?;
    // A refused card is an answer the agent should read and fix, not a crash.
    if let Some(e) = v.get("error").and_then(Value::as_str) {
        return Ok(format!("question refusée : {e}"));
    }
    Ok(serde_json::to_string_pretty(&v)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stand-in harnes: reads one request, replies with `reply`, returns what it received.
    fn server(reply: &'static str) -> (String, std::thread::JoinHandle<String>) {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap().to_string();
        let h = std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            // Read the whole request: answering early would reset the connection.
            let mut got = Vec::new();
            let mut chunk = [0u8; 1024];
            loop {
                let n = s.read(&mut chunk).unwrap();
                got.extend_from_slice(&chunk[..n]);
                let text = String::from_utf8_lossy(&got).to_string();
                if let Some((head, body)) = text.split_once("\r\n\r\n") {
                    let len = head
                        .lines()
                        .find_map(|l| l.strip_prefix("Content-Length: "))
                        .and_then(|v| v.trim().parse::<usize>().ok())
                        .unwrap_or(0);
                    if body.len() >= len {
                        break;
                    }
                }
                if n == 0 {
                    break;
                }
            }
            let got = String::from_utf8_lossy(&got).to_string();
            write!(
                s,
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                reply.len()
            )
            .unwrap();
            got
        });
        (format!("http://{addr}"), h)
    }

    #[test]
    fn the_question_goes_to_this_agent_and_the_answer_comes_back() {
        let (url, got) = server(r#"{"answered":true,"choice":"b","text":null}"#);
        let out = ask(&url, "17-worker-L1-r0", &json!({"kind": "choice", "question": "Which?", "options": [{"id": "a", "label": "A"}, {"id": "b", "label": "B"}]})).unwrap();
        assert!(out.contains("\"choice\": \"b\""), "{out}");
        let request = got.join().unwrap();
        assert!(
            request.starts_with("POST /ask/17-worker-L1-r0 HTTP/1.1"),
            "{request}"
        );
        assert!(request.contains("\"question\":\"Which?\""));
        // A card the registry refuses is told to the agent, in words.
        let (url, got) = server(r#"{"error":"kind `slider` inconnu"}"#);
        let out = ask(
            &url,
            "17-worker-L1-r0",
            &json!({"kind": "slider", "question": "x"}),
        )
        .unwrap();
        assert!(out.starts_with("question refusée"), "{out}");
        let _ = got.join();
        // An identity that could change the path is refused.
        assert!(ask("http://127.0.0.1:1", "a/../b", &json!({})).is_err());
    }
}
