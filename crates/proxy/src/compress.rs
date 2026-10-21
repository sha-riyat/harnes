//! Deterministic compression of `tool_result` blocks in a Messages request.
//!
//! Deterministic on purpose: the same history always compresses to the same
//! bytes, so Anthropic prompt caching keeps hitting across turns.

use harnes_ctx::Store;
use serde_json::Value;

/// Below this size a block is left untouched.
pub const LIGHT_MIN: usize = 2 * 1024;
/// Above this size (after light cleanup) the middle is elided to the store.
pub const ELIDE_MIN: usize = 24 * 1024;
const KEEP_HEAD: usize = 10 * 1024;
const KEEP_TAIL: usize = 6 * 1024;

#[derive(Debug, Default, Clone, Copy)]
pub struct Saved {
    pub before: usize,
    pub after: usize,
    pub elided: usize,
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&n) = chars.peek() {
                    chars.next();
                    if ('@'..='~').contains(&n) {
                        break;
                    }
                }
            }
            continue;
        }
        if c != '\r' {
            out.push(c);
        }
    }
    out
}

/// Strips ANSI, trailing spaces, collapses runs of identical lines and minifies JSON.
pub fn light(s: &str) -> String {
    let trimmed = s.trim();
    if (trimmed.starts_with('{') || trimmed.starts_with('[')) && trimmed.len() > 64 {
        if let Ok(v) = serde_json::from_str::<Value>(trimmed) {
            return v.to_string();
        }
    }
    let clean = strip_ansi(s);
    let mut out = String::with_capacity(clean.len());
    let mut prev: Option<&str> = None;
    let mut run = 0usize;
    let flush = |out: &mut String, run: usize| {
        if run > 1 {
            out.push_str(&format!(
                "[… previous line repeated {} more times]\n",
                run - 1
            ));
        }
    };
    for line in clean.lines().map(str::trim_end) {
        if Some(line) == prev {
            run += 1;
            continue;
        }
        flush(&mut out, run);
        out.push_str(line);
        out.push('\n');
        prev = Some(line);
        run = 1;
    }
    flush(&mut out, run);
    out
}

fn floor_char(s: &str, mut i: usize) -> usize {
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Compresses one text payload, storing the original when the middle is cut.
pub fn compress_text(text: &str, store: Option<&Store>) -> Option<(String, bool)> {
    if text.len() < LIGHT_MIN {
        return None;
    }
    let mut out = light(text);
    let mut elided = false;
    if out.len() > ELIDE_MIN {
        if let Some(store) = store {
            if let Ok(id) = store.put("proxy:tool_result", text) {
                let head = floor_char(&out, KEEP_HEAD);
                let tail = floor_char(&out, out.len() - KEEP_TAIL);
                let cut = tail - head;
                out = format!(
                    "{}\n[harnes: {cut} bytes elided — full output stored as id={id}; use ctx_retrieve or ctx_search]\n{}",
                    &out[..head],
                    &out[tail..]
                );
                elided = true;
            }
        }
    }
    (out.len() < text.len()).then_some((out, elided))
}

fn compress_block(block: &mut Value, store: Option<&Store>, saved: &mut Saved) {
    let text = match block.get_mut("text") {
        Some(Value::String(t)) => t,
        _ => return,
    };
    if let Some((new, elided)) = compress_text(text, store) {
        saved.before += text.len();
        saved.after += new.len();
        saved.elided += elided as usize;
        *text = new;
    }
}

/// Rewrites `tool_result` contents in place. Returns what was saved.
pub fn compress_request(body: &mut Value, store: Option<&Store>) -> Saved {
    let mut saved = Saved::default();
    let Some(messages) = body.get_mut("messages").and_then(Value::as_array_mut) else {
        return saved;
    };
    for msg in messages.iter_mut().filter(|m| m["role"] == "user") {
        let Some(blocks) = msg.get_mut("content").and_then(Value::as_array_mut) else {
            continue;
        };
        for block in blocks.iter_mut().filter(|b| b["type"] == "tool_result") {
            match block.get_mut("content") {
                Some(Value::String(s)) => {
                    if let Some((new, elided)) = compress_text(s, store) {
                        saved.before += s.len();
                        saved.after += new.len();
                        saved.elided += elided as usize;
                        *s = new;
                    }
                }
                Some(Value::Array(parts)) => {
                    for part in parts.iter_mut().filter(|p| p["type"] == "text") {
                        compress_block(part, store, &mut saved);
                    }
                }
                _ => {}
            }
        }
    }
    saved
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn collapses_repeats_and_ansi() {
        let s = "\x1b[32mok\x1b[0m\nsame\nsame\nsame\nend\n";
        assert_eq!(
            light(s),
            "ok\nsame\n[… previous line repeated 2 more times]\nend\n"
        );
    }

    #[test]
    fn elides_huge_results_deterministically() {
        let store = Store::in_memory().unwrap();
        let big: String = (0..5000)
            .map(|i| format!("row {i} value {}\n", i * 7))
            .collect();
        let mut a = json!({"messages":[{"role":"user","content":[{"type":"tool_result","tool_use_id":"x","content":big}]}]});
        let mut b = a.clone();
        let saved = compress_request(&mut a, Some(&store));
        compress_request(&mut b, Some(&store));
        assert_eq!(a, b, "must be deterministic for prompt caching");
        assert_eq!(saved.elided, 1);
        assert!(saved.after < saved.before / 3);
        let text = a["messages"][0]["content"][0]["content"].as_str().unwrap();
        assert!(text.contains("ctx_retrieve"));
    }

    #[test]
    fn leaves_small_results_alone() {
        let mut v = json!({"messages":[{"role":"user","content":[{"type":"tool_result","content":"short"}]}]});
        let before = v.clone();
        compress_request(&mut v, None);
        assert_eq!(v, before);
    }
}
