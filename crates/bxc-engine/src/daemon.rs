// SPDX-License-Identifier: Apache-2.0
//! HTTP daemon — axum server exposing the bxc JSON API contract.
//!
//! Routes (exact match with `packages/bxc/src/cli/api.ts` so the existing
//! `crates/cli/src/scrape.rs::ScrapeClient` is a drop-in consumer):
//!
//! | Method | Path             | Body / Query                | Handler         |
//! |--------|------------------|-----------------------------|-----------------|
//! | GET    | `/healthz`       | —                           | [`healthz`]     |
//! | GET    | `/api/recon`     | `?url=`                     | [`recon_get`]   |
//! | POST   | `/api/recon`     | `{ "url": "…" }`            | [`recon_post`]  |
//! | GET    | `/api/detect`    | `?url=`                     | [`detect_get`]  |
//! | POST   | `/api/detect`    | `{ "url": "…" }`            | [`detect_post`] |
//! | GET    | `/api/scrape`    | `?url=&selector=&max=`      | [`scrape_get`]  |
//! | POST   | `/api/scrape`    | `{ url, selector, max? }`   | [`scrape_post`] |
//! | GET    | `/api/tokens`    | `?url=`                     | [`tokens_get`]  |
//!
//! All responses carry an `x-bxc-version` header. Errors are JSON
//! `{ "error": "…", "reason": "…?" }` with the appropriate HTTP status.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{Query, State};
use axum::http::{header, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use serde::Deserialize;
use serde_json::json;

use crate::scraper;
use crate::VERSION;

/// Shared application state — one HTTP client reused across requests so
/// keep-alive pools survive.
#[derive(Clone)]
pub struct AppState {
    pub http: reqwest::Client,
}

impl AppState {
    pub fn new() -> anyhow::Result<Self> {
        Ok(Self { http: scraper::http_client()? })
    }
}

impl Default for AppState {
    fn default() -> Self {
        // Best-effort fallback: build a client with workspace defaults.
        // Construction can only fail if no rustls provider is installed —
        // tests/binaries install one via `rustls::crypto::ring`.
        Self::new().unwrap_or_else(|_| AppState {
            http: reqwest::Client::new(),
        })
    }
}

/// Construct the full router. Public so integration tests can mount it
/// onto an ephemeral port without going through [`serve`].
pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/api/recon", get(recon_get).post(recon_post))
        .route("/api/detect", get(detect_get).post(detect_post))
        .route("/api/scrape", get(scrape_get).post(scrape_post))
        .route("/api/tokens", get(tokens_get))
        .with_state(Arc::new(state))
}

/// Bind and serve forever on `addr`. Returns the bound address (useful
/// when port=0). Spawns the listener on the current tokio runtime.
pub async fn serve(addr: SocketAddr) -> anyhow::Result<()> {
    let state = AppState::new()?;
    let app = router(state);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let local = listener.local_addr()?;
    tracing::info!("bxc daemon listening on http://{local}");
    axum::serve(listener, app)
        .await
        .map_err(|e| anyhow::anyhow!("axum serve failed: {e}"))?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Header helper
// ─────────────────────────────────────────────────────────────────────────────

fn version_header() -> (header::HeaderName, HeaderValue) {
    (header::HeaderName::from_static("x-bxc-version"),
     HeaderValue::from_static(VERSION))
}

fn json_response(status: StatusCode, body: serde_json::Value) -> Response {
    let mut resp = (status, Json(body)).into_response();
    let (k, v) = version_header();
    resp.headers_mut().insert(k, v);
    resp
}

// ─────────────────────────────────────────────────────────────────────────────
// Handlers
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct UrlQuery {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct UrlBody {
    pub url: String,
}

#[derive(Debug, Deserialize)]
pub struct ScrapeQuery {
    pub url: String,
    pub selector: String,
    #[serde(default = "default_max")]
    pub max: usize,
}

fn default_max() -> usize {
    50
}

#[derive(Debug, Deserialize)]
pub struct ScrapeBody {
    pub url: String,
    pub selector: String,
    #[serde(default)]
    pub max: Option<usize>,
}

pub async fn healthz(State(_): State<Arc<AppState>>) -> Response {
    json_response(
        StatusCode::OK,
        json!({ "ok": true, "version": VERSION }),
    )
}

pub async fn recon_get(
    State(s): State<Arc<AppState>>,
    Query(q): Query<UrlQuery>,
) -> Response {
    match scraper::recon(&s.http, &q.url).await {
        Ok(r) => json_response(StatusCode::OK, serde_json::to_value(r).unwrap_or(json!({}))),
        Err(e) => json_response(
            StatusCode::BAD_GATEWAY,
            json!({ "error": "recon failed", "reason": e.to_string() }),
        ),
    }
}

pub async fn recon_post(
    State(s): State<Arc<AppState>>,
    Json(body): Json<UrlBody>,
) -> Response {
    match scraper::recon(&s.http, &body.url).await {
        Ok(r) => json_response(StatusCode::OK, serde_json::to_value(r).unwrap_or(json!({}))),
        Err(e) => json_response(
            StatusCode::BAD_GATEWAY,
            json!({ "error": "recon failed", "reason": e.to_string() }),
        ),
    }
}

pub async fn detect_get(
    State(s): State<Arc<AppState>>,
    Query(q): Query<UrlQuery>,
) -> Response {
    match scraper::detect(&s.http, &q.url).await {
        Ok(r) => json_response(StatusCode::OK, serde_json::to_value(r).unwrap_or(json!({}))),
        Err(e) => json_response(
            StatusCode::BAD_GATEWAY,
            json!({ "error": "detect failed", "reason": e.to_string() }),
        ),
    }
}

pub async fn detect_post(
    State(s): State<Arc<AppState>>,
    Json(body): Json<UrlBody>,
) -> Response {
    match scraper::detect(&s.http, &body.url).await {
        Ok(r) => json_response(StatusCode::OK, serde_json::to_value(r).unwrap_or(json!({}))),
        Err(e) => json_response(
            StatusCode::BAD_GATEWAY,
            json!({ "error": "detect failed", "reason": e.to_string() }),
        ),
    }
}

pub async fn scrape_get(
    State(s): State<Arc<AppState>>,
    Query(q): Query<ScrapeQuery>,
) -> Response {
    match scraper::scrape(&s.http, &q.url, &q.selector, q.max).await {
        Ok(matches) => json_response(
            StatusCode::OK,
            serde_json::to_value(matches).unwrap_or(json!([])),
        ),
        Err(e) => json_response(
            StatusCode::BAD_GATEWAY,
            json!({ "error": "scrape failed", "reason": e.to_string() }),
        ),
    }
}

pub async fn scrape_post(
    State(s): State<Arc<AppState>>,
    Json(body): Json<ScrapeBody>,
) -> Response {
    let max = body.max.unwrap_or(50);
    match scraper::scrape(&s.http, &body.url, &body.selector, max).await {
        Ok(matches) => json_response(
            StatusCode::OK,
            serde_json::to_value(matches).unwrap_or(json!([])),
        ),
        Err(e) => json_response(
            StatusCode::BAD_GATEWAY,
            json!({ "error": "scrape failed", "reason": e.to_string() }),
        ),
    }
}

pub async fn tokens_get(
    State(s): State<Arc<AppState>>,
    Query(q): Query<UrlQuery>,
) -> Response {
    match scraper::tokens_m3(&s.http, &q.url).await {
        Ok(m) => json_response(StatusCode::OK, serde_json::to_value(m).unwrap_or(json!({}))),
        Err(e) => json_response(
            StatusCode::BAD_GATEWAY,
            json!({ "error": "tokens failed", "reason": e.to_string() }),
        ),
    }
}
