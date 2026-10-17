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
    /// The account's usage limits, as the last response reported them.
    pub limits: std::sync::Mutex<Option<Limits>>,
}

/// Claude subscription limits from `anthropic-ratelimit-unified-*` headers:
/// the 5-hour and 7-day windows, each used 0..1, and when they reset (epoch s).
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct Limits {
    pub five_hour: f64,
    pub five_hour_reset: u64,
    pub seven_day: f64,
    pub seven_day_reset: u64,
    /// allowed, allowed_warning or rejected.
    pub status: String,
    /// When these were read (epoch s).
    pub seen: u64,
}

impl Limits {
    /// None when the response carries no subscription limits (API keys, errors).
    pub fn from_headers(h: &reqwest::header::HeaderMap) -> Option<Limits> {
        let get = |k: &str| h.get(format!("anthropic-ratelimit-unified-{k}")).and_then(|v| v.to_str().ok()).map(str::to_string);
        let num = |k: &str| get(k).and_then(|v| v.parse::<f64>().ok());
        let five = num("5h-utilization");
        let week = num("7d-utilization");
        if five.is_none() && week.is_none() {
            return None;
        }
        let secs = |k: &str| num(k).map(|v| v as u64).unwrap_or(0);
        let seen = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        Some(Limits {
            five_hour: five.unwrap_or(0.0),
            five_hour_reset: secs("5h-reset"),
            seven_day: week.unwrap_or(0.0),
            seven_day_reset: secs("7d-reset"),
            status: get("status").unwrap_or_default(),
            seen,
        })
    }
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
    pub limits: Option<Limits>,
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
            limits: self.limits.lock().ok().and_then(|l| l.clone()),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_subscription_limits() {
        let mut h = reqwest::header::HeaderMap::new();
        for (k, v) in [("5h-utilization", "0.27"), ("5h-reset", "1792261200"), ("7d-utilization", "0.65"), ("7d-reset", "1792699200"), ("status", "allowed")] {
            h.insert(reqwest::header::HeaderName::from_bytes(format!("anthropic-ratelimit-unified-{k}").as_bytes()).unwrap(), v.parse().unwrap());
        }
        let l = Limits::from_headers(&h).unwrap();
        assert_eq!((l.five_hour, l.seven_day, l.seven_day_reset, l.status.as_str()), (0.27, 0.65, 1792699200, "allowed"));
        assert!(Limits::from_headers(&reqwest::header::HeaderMap::new()).is_none());
    }
}
