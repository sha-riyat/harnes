use serde::Serialize;
use serde_json::Value;
use std::sync::{
    atomic::{AtomicU64, Ordering::Relaxed},
    Arc,
};

/// One usage observation from an SSE stream, emitted to the telemetry sink.
#[derive(Debug, Clone, Serialize)]
pub struct UsageEvent {
    pub at: u64,
    pub phase: String,
    pub model: Option<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
}

pub type UsageSink = Arc<dyn Fn(UsageEvent) + Send + Sync>;

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
        let get = |k: &str| {
            h.get(format!("anthropic-ratelimit-unified-{k}"))
                .and_then(|v| v.to_str().ok())
                .map(str::to_string)
        };
        let num = |k: &str| get(k).and_then(|v| v.parse::<f64>().ok());
        let five = num("5h-utilization");
        let week = num("7d-utilization");
        if five.is_none() && week.is_none() {
            return None;
        }
        let secs = |k: &str| num(k).map(|v| v as u64).unwrap_or(0);
        let seen = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
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
        let saved = self
            .bytes_before
            .load(Relaxed)
            .saturating_sub(self.bytes_after.load(Relaxed));
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

    /// `message_start` reports input/cache usage and a provisional output count;
    /// `message_delta` reports the cumulative final output. Only count output on
    /// the delta, or every message would be counted twice.
    fn add_usage(&self, u: &Value, output: bool) {
        let get = |k: &str| u.get(k).and_then(Value::as_u64).unwrap_or(0);
        self.input_tokens.fetch_add(get("input_tokens"), Relaxed);
        if output {
            self.output_tokens.fetch_add(get("output_tokens"), Relaxed);
        }
        self.cache_read_tokens
            .fetch_add(get("cache_read_input_tokens"), Relaxed);
        self.cache_write_tokens
            .fetch_add(get("cache_creation_input_tokens"), Relaxed);
    }
}

fn usage_event(phase: &str, model: &Option<String>, u: &Value, output: bool) -> UsageEvent {
    let get = |k: &str| u.get(k).and_then(Value::as_u64).unwrap_or(0);
    UsageEvent {
        at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0),
        phase: phase.into(),
        model: model.clone(),
        input_tokens: get("input_tokens"),
        output_tokens: if output { get("output_tokens") } else { 0 },
        cache_read_tokens: get("cache_read_input_tokens"),
        cache_write_tokens: get("cache_creation_input_tokens"),
    }
}

/// Reads token usage out of an SSE stream, chunk by chunk.
#[derive(Default)]
pub struct UsageTap {
    line: Vec<u8>,
    model: Option<String>,
    sink: Option<UsageSink>,
}

impl UsageTap {
    pub fn new(model: Option<String>, sink: Option<UsageSink>) -> Self {
        Self {
            model,
            sink,
            ..Default::default()
        }
    }

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
                    Some("message_start") => {
                        let usage = v["message"]["usage"].clone();
                        stats.add_usage(&usage, false);
                        if let Some(sink) = &self.sink {
                            sink(usage_event("message_start", &self.model, &usage, false));
                        }
                    }
                    Some("message_delta") => {
                        stats.add_usage(&v["usage"], true);
                        if let Some(sink) = &self.sink {
                            sink(usage_event("message_delta", &self.model, &v["usage"], true));
                        }
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
    fn counts_stream_usage_once() {
        let stats = Stats::default();
        let seen = Arc::new(std::sync::Mutex::new(Vec::new()));
        let sink = {
            let seen = seen.clone();
            Arc::new(move |e: UsageEvent| seen.lock().unwrap().push(e)) as UsageSink
        };
        let mut tap = UsageTap::new(Some("claude-sonnet".into()), Some(sink));
        tap.feed(
            b"data: {\"type\":\"message_start\",\"message\":{\"usage\":{\"input_tokens\":120,\"cache_read_input_tokens\":80,\"cache_creation_input_tokens\":7,\"output_tokens\":1}}}\n",
            &stats,
        );
        tap.feed(
            b"data: {\"type\":\"message_delta\",\"usage\":{\"output_tokens\":42}}\n",
            &stats,
        );
        let s = stats.snapshot();
        assert_eq!(
            (
                s.input_tokens,
                s.output_tokens,
                s.cache_read_tokens,
                s.cache_write_tokens
            ),
            (120, 42, 80, 7)
        );
        let seen = seen.lock().unwrap();
        assert_eq!(seen.len(), 2);
        assert_eq!(seen.iter().map(|e| e.output_tokens).sum::<u64>(), 42);
        assert_eq!(seen[0].output_tokens, 0);
        assert_eq!(seen[1].output_tokens, 42);
    }

    #[test]
    fn reads_subscription_limits() {
        let mut h = reqwest::header::HeaderMap::new();
        for (k, v) in [
            ("5h-utilization", "0.27"),
            ("5h-reset", "1792261200"),
            ("7d-utilization", "0.65"),
            ("7d-reset", "1792699200"),
            ("status", "allowed"),
        ] {
            h.insert(
                reqwest::header::HeaderName::from_bytes(
                    format!("anthropic-ratelimit-unified-{k}").as_bytes(),
                )
                .unwrap(),
                v.parse().unwrap(),
            );
        }
        let l = Limits::from_headers(&h).unwrap();
        assert_eq!(
            (
                l.five_hour,
                l.seven_day,
                l.seven_day_reset,
                l.status.as_str()
            ),
            (0.27, 0.65, 1792699200, "allowed")
        );
        assert!(Limits::from_headers(&reqwest::header::HeaderMap::new()).is_none());
    }
}
