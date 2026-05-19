// SPDX-License-Identifier: Apache-2.0
//! End-to-end smoke test for the bxc-engine HTTP daemon.
//!
//! Spins up the axum router on an ephemeral port (port 0), then issues
//! one request per route via `reqwest` and asserts the JSON shape.
//!
//! Network-free: every route is exercised with the daemon talking to a
//! sibling axum mock instead of the real internet so the test is
//! deterministic and sandbox-friendly.

use std::net::SocketAddr;
use std::time::Duration;

use axum::{routing::get, Router};
use bxc_engine::daemon::{router, AppState};
use serde_json::Value;
use tokio::net::TcpListener;

/// Install the rustls crypto provider once per test process.
fn install_crypto() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

/// Spawn a tiny HTML server returning a Next.js-flavoured page with a
/// CSS selector match + an M3 design token.
async fn spawn_mock() -> (String, tokio::task::JoinHandle<()>) {
    let body = r#"<!doctype html>
<html><head>
  <title>Mock</title>
  <style>:root{--md-sys-color-primary:#6750A4;--md-sys-color-on-primary:#fff;}</style>
  <link rel="stylesheet" href="/static/app.css" />
  <script src="/_next/static/chunks/main.js"></script>
</head>
<body>
  <main><h1 class="title">Hello World</h1><h1 class="title">Second</h1></main>
  <script id="__NEXT_DATA__">{"props":{"pageProps":{}}}</script>
</body></html>"#;
    let app = Router::new().route("/", get(move || async move { axum::response::Html(body) }));
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind mock");
    let addr = listener.local_addr().expect("local_addr");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    (format!("http://{addr}/"), handle)
}

/// Spawn the bxc daemon on an ephemeral port and return its base URL.
async fn spawn_daemon() -> (String, tokio::task::JoinHandle<()>) {
    let state = AppState::new().expect("AppState::new must succeed");
    let app = router(state);
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind daemon");
    let addr = listener.local_addr().expect("local_addr");
    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.ok();
    });
    (format!("http://{addr}"), handle)
}

fn http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .expect("client")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn healthz_returns_ok() {
    install_crypto();
    let (base, _h) = spawn_daemon().await;
    let client = http_client();
    let resp = client.get(format!("{base}/healthz")).send().await.expect("send");
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers().get("x-bxc-version").map(|v| v.to_str().unwrap_or("")),
        Some(bxc_engine::VERSION)
    );
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["ok"], Value::Bool(true));
    assert_eq!(body["version"], Value::String(bxc_engine::VERSION.to_string()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn recon_returns_v1_schema_against_mock() {
    install_crypto();
    let (mock_url, _m) = spawn_mock().await;
    let (base, _d) = spawn_daemon().await;
    let client = http_client();

    let resp = client
        .get(format!("{base}/api/recon"))
        .query(&[("url", &mock_url)])
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status(), 200, "recon must be 200, got {}", resp.status());
    let body: Value = resp.json().await.expect("json");
    assert_eq!(body["$schema"], Value::String("bxc-recon-v1".into()));
    assert_eq!(body["profile"], Value::String("http".into()));
    assert!(body["bytes"].as_u64().unwrap_or(0) > 0);
    assert!(!body["bodyHash"].as_str().unwrap_or("").is_empty());
    // Asset extraction must have found at least the linked stylesheet + script.
    let assets = body["assets"].as_array().expect("assets array");
    assert!(assets.iter().any(|a| a["type"] == "stylesheet"));
    assert!(assets.iter().any(|a| a["type"] == "script"));
    // Framework sniffer must catch Next.js via __NEXT_DATA__.
    let fws = body["frameworks"].as_array().expect("frameworks array");
    assert!(
        fws.iter().any(|f| f["name"] == "Next.js"),
        "frameworks must include Next.js: {fws:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn detect_post_returns_multi_bucket_against_mock() {
    install_crypto();
    let (mock_url, _m) = spawn_mock().await;
    let (base, _d) = spawn_daemon().await;
    let client = http_client();

    let resp = client
        .post(format!("{base}/api/detect"))
        .json(&serde_json::json!({ "url": mock_url }))
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.expect("json");
    assert!(body["frontend"].is_array());
    assert!(body["backend"].is_array());
    assert!(body["cdn"].is_array());
    assert!(body["library"].is_array());
    // Next.js marker → frontend bucket; Material Web → library bucket.
    let frontend = body["frontend"].as_array().unwrap();
    assert!(
        frontend.iter().any(|e| e["name"] == "Next.js"),
        "frontend must include Next.js: {frontend:?}"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn scrape_selector_returns_matches_against_mock() {
    install_crypto();
    let (mock_url, _m) = spawn_mock().await;
    let (base, _d) = spawn_daemon().await;
    let client = http_client();

    let resp = client
        .get(format!("{base}/api/scrape"))
        .query(&[("url", mock_url.as_str()), ("selector", "h1.title")])
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.expect("json");
    let arr = body.as_array().expect("scrape returns array");
    assert_eq!(arr.len(), 2, "two h1.title elements, got {arr:?}");
    assert_eq!(arr[0]["index"], 0);
    assert_eq!(arr[0]["text"], "Hello World");
    assert_eq!(arr[1]["text"], "Second");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn tokens_endpoint_extracts_m3_custom_properties() {
    install_crypto();
    let (mock_url, _m) = spawn_mock().await;
    let (base, _d) = spawn_daemon().await;
    let client = http_client();

    let resp = client
        .get(format!("{base}/api/tokens"))
        .query(&[("url", &mock_url)])
        .send()
        .await
        .expect("send");
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.expect("json");
    let tokens = body["tokens"].as_object().expect("tokens object");
    assert!(
        tokens.contains_key("--md-sys-color-primary"),
        "must extract --md-sys-color-primary: {tokens:?}"
    );
    assert_eq!(
        tokens["--md-sys-color-primary"],
        Value::String("#6750A4".into())
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn recon_400_on_invalid_url_schema() {
    install_crypto();
    let (base, _d) = spawn_daemon().await;
    let client = http_client();
    // No `url` query param → axum's `Query<UrlQuery>` rejects with 400.
    let resp = client.get(format!("{base}/api/recon")).send().await.expect("send");
    assert!(
        resp.status() == 400 || resp.status() == 422,
        "expected 400/422 for missing url, got {}",
        resp.status()
    );
}

// Compile-time sanity: bxc_engine re-exports the router so external smoke
// drivers can mount the daemon without forking the SocketAddr API.
#[test]
fn router_constructible_without_panic() {
    install_crypto();
    let state = AppState::new().expect("state");
    let _r = router(state);
    let _addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
}
