//! Local proxy for the Anthropic API. Point `ANTHROPIC_BASE_URL` at it.
//!
//! Requests to `/v1/messages` get their `tool_result` blocks compressed
//! (see [`compress`]); everything else, including streaming responses, is
//! passed through unchanged. Token usage is read from the SSE stream.

pub mod compress;
pub mod stats;

use anyhow::Result;
use axum::{
    body::{Body, Bytes},
    extract::State,
    http::{HeaderMap, Method, StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::{any, get},
    Json, Router,
};
use futures_util::StreamExt;
use harnes_ctx::Store;
use stats::{Stats, StatsSnapshot, UsageTap};
use std::sync::{atomic::Ordering::Relaxed, Arc};

pub const DEFAULT_UPSTREAM: &str = "https://api.anthropic.com";

#[derive(Clone)]
pub struct ProxyConfig {
    pub port: u16,
    pub upstream: String,
    pub compress: bool,
    pub store: Option<Arc<Store>>,
}

struct Shared {
    cfg: ProxyConfig,
    client: reqwest::Client,
    stats: Arc<Stats>,
}

pub struct ProxyHandle {
    pub port: u16,
    pub stats: Arc<Stats>,
    task: tokio::task::JoinHandle<()>,
}

impl ProxyHandle {
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
    pub fn snapshot(&self) -> StatsSnapshot {
        self.stats.snapshot()
    }
    pub fn stop(&self) {
        self.task.abort();
    }
}

/// Binds on 127.0.0.1 (port 0 picks a free port) and serves in the background.
pub async fn start(cfg: ProxyConfig) -> Result<ProxyHandle> {
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", cfg.port)).await?;
    let port = listener.local_addr()?.port();
    let stats = Arc::new(Stats::default());
    let shared = Arc::new(Shared { client: reqwest::Client::builder().build()?, cfg, stats: stats.clone() });
    let app = Router::new()
        .route("/harnes/stats", get(stats_route))
        .fallback(any(forward))
        .with_state(shared);
    let task = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            tracing::error!("proxy stopped: {e}");
        }
    });
    Ok(ProxyHandle { port, stats, task })
}

async fn stats_route(State(s): State<Arc<Shared>>) -> Json<StatsSnapshot> {
    Json(s.stats.snapshot())
}

const HOP: [&str; 6] = ["host", "content-length", "connection", "transfer-encoding", "keep-alive", "accept-encoding"];

async fn forward(State(s): State<Arc<Shared>>, method: Method, uri: Uri, headers: HeaderMap, body: Bytes) -> Response {
    s.stats.requests.fetch_add(1, Relaxed);
    let path = uri.path_and_query().map(|p| p.as_str()).unwrap_or("/");
    let url = format!("{}{}", s.cfg.upstream.trim_end_matches('/'), path);

    let mut body = body.to_vec();
    if s.cfg.compress && method == Method::POST && uri.path().ends_with("/v1/messages") {
        if let Ok(mut json) = serde_json::from_slice::<serde_json::Value>(&body) {
            let saved = compress::compress_request(&mut json, s.cfg.store.as_deref());
            if saved.before > 0 {
                s.stats.bytes_before.fetch_add(saved.before as u64, Relaxed);
                s.stats.bytes_after.fetch_add(saved.after as u64, Relaxed);
                s.stats.elided_blocks.fetch_add(saved.elided as u64, Relaxed);
                body = serde_json::to_vec(&json).unwrap_or(body);
            }
        }
    }

    let mut req = s.client.request(method, &url).body(body);
    for (k, v) in headers.iter() {
        if !HOP.contains(&k.as_str()) {
            req = req.header(k, v);
        }
    }
    let upstream = match req.send().await {
        Ok(r) => r,
        Err(e) => {
            s.stats.errors.fetch_add(1, Relaxed);
            return (StatusCode::BAD_GATEWAY, format!("harnes proxy: {e}")).into_response();
        }
    };

    let status = upstream.status();
    if !status.is_success() {
        s.stats.errors.fetch_add(1, Relaxed);
    }
    let mut builder = Response::builder().status(status.as_u16());
    for (k, v) in upstream.headers().iter() {
        if !HOP.contains(&k.as_str()) {
            builder = builder.header(k, v);
        }
    }
    let stats = s.stats.clone();
    let mut tap = UsageTap::default();
    let stream = upstream.bytes_stream().map(move |chunk| {
        if let Ok(c) = &chunk {
            tap.feed(c, &stats);
        }
        chunk
    });
    builder.body(Body::from_stream(stream)).unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}
