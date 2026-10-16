use serde::Serialize;
use serde_json::Value;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

#[derive(Default)]
pub struct Stats {
    pub requests: AtomicU64,
    pub errors: AtomicU64,
    pub bytes_before: AtomicU64,
    pub bytes_after: AtomicU64,
    pub elided_blocks: AtomicU64,
    pub input_tokens: AtomicU64,
    pub output_tokens: AtomicU64,
    pub cache_read_tokens: AtomicU64,
    pub cache_write_tokens: AtomicU64,
}

#[derive(Debug, Clone, Serialize)]
pub struct StatsSnapshot {
    pub requests: u64,
    pub errors: u64,
    pub bytes_saved: u64,
    pub est_tokens_saved: u64,
    pub elided_blocks: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
}

impl Stats {
    pub fn snapshot(&self) -> StatsSnapshot {
        let saved = self.bytes_before.load(Relaxed).saturating_sub(self.bytes_after.load(Relaxed));
        StatsSnapshot {
            requests: self.requests.load(Relaxed),
            errors: self.errors.load(Relaxed),
            bytes_saved: saved,
            est_tokens_saved: saved / 4,
            elided_blocks: self.elided_blocks.load(Relaxed),
            input_tokens: self.input_tokens.load(Relaxed),
            output_tokens: self.output_tokens.load(Relaxed),
            cache_read_tokens: self.cache_read_tokens.load(Relaxed),
            cache_write_tokens: self.cache_write_tokens.load(Relaxed),
        }
    }

    fn add_usage(&self, u: &Value) {
        let get = |k: &str| u.get(k).and_then(Value::as_u64).unwrap_or(0);
        self.input_tokens.fetch_add(get("input_tokens"), Relaxed);
        self.output_tokens.fetch_add(get("output_tokens"), Relaxed);
        self.cache_read_tokens.fetch_add(get("cache_read_input_tokens"), Relaxed);
        self.cache_write_tokens.fetch_add(get("cache_creation_input_tokens"), Relaxed);
    }
}

/// Reads token usage out of an SSE stream, chunk by chunk.
#[derive(Default)]
pub struct UsageTap {
    line: Vec<u8>,
}

impl UsageTap {
    pub fn feed(&mut self, chunk: &[u8], stats: &Stats) {
        for &b in chunk {
            if b == b'\n' {
                self.line_done(stats);
            } else if self.line.len() < 64 * 1024 {
                self.line.push(b);
            }
        }
    }

    fn line_done(&mut self, stats: &Stats) {
        if let Some(data) = self.line.strip_prefix(b"data: ") {
            if let Ok(v) = serde_json::from_slice::<Value>(data) {
                match v["type"].as_str() {
                    // message_start carries input/cache usage; message_delta the final output count.
                    Some("message_start") => stats.add_usage(&v["message"]["usage"]),
                    Some("message_delta") => {
                        let out = v["usage"]["output_tokens"].as_u64().unwrap_or(0);
                        stats.output_tokens.fetch_add(out, Relaxed);
                    }
                    _ => {}
                }
            }
        }
        self.line.clear();
    }
}
