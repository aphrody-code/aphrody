// SPDX-License-Identifier: Apache-2.0
//! Integration smoke tests for every Tier-1 connector.

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;

use aphrody_messaging::{
    Connector, Message, MessageError, MessageId, ParseMode, SlackConnector, TelegramConnector,
};
#[cfg(not(target_arch = "wasm32"))]
use aphrody_messaging::{SmtpConnector, TlsMode};

use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::{Json, Router};
use serde_json::{Value, json};
use tokio::net::TcpListener;

/// Install the rustls ring CryptoProvider once for the whole test binary —
/// reqwest panics with "No provider set" otherwise (see CLAUDE.md §7).
fn install_crypto_provider() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

#[derive(Clone, Default)]
struct MockState {
    last_body: Arc<Mutex<Option<Value>>>,
    last_auth: Arc<Mutex<Option<String>>>,
}

async fn spawn_mock(router: Router) -> SocketAddr {
    let listener =
        TcpListener::bind(("127.0.0.1", 0)).await.expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local_addr");
    tokio::spawn(async move {
        axum::serve(listener, router).await.ok();
    });
    // Give the server a moment to come up.
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    addr
}

// ── Test 1 — Telegram serialisation ───────────────────────────────────────────

#[tokio::test]
async fn telegram_serializes_send_payload_correctly() {
    install_crypto_provider();
    let state = MockState::default();
    async fn handler(
        State(s): State<MockState>,
        Json(body): Json<Value>,
    ) -> Json<Value> {
        *s.last_body.lock().expect("lock") = Some(body);
        Json(json!({
            "ok": true,
            "result": { "message_id": 42 }
        }))
    }
    let router = Router::new()
        .route("/bot{token}/sendMessage", post(handler))
        .with_state(state.clone());
    let addr = spawn_mock(router).await;
    let base = format!("http://{addr}");

    let connector =
        TelegramConnector::with_base("test-token", base).expect("build connector");
    let msg = Message::text("@yoyo", "hello world").with_parse_mode(ParseMode::Markdown);
    let id: MessageId = connector.send(msg).await.expect("send ok");
    assert_eq!(id.0, "42");

    let captured = state.last_body.lock().expect("lock").clone().expect("body");
    assert_eq!(captured["chat_id"], "@yoyo");
    assert_eq!(captured["text"], "hello world");
    assert_eq!(captured["parse_mode"], "MarkdownV2");
}

// ── Test 2 — Telegram API error propagation ───────────────────────────────────

#[tokio::test]
async fn telegram_propagates_api_error() {
    install_crypto_provider();
    async fn handler() -> Json<Value> {
        Json(json!({
            "ok": false,
            "error_code": 400,
            "description": "Bad Request: chat not found"
        }))
    }
    let router = Router::new().route("/bot{token}/sendMessage", post(handler));
    let addr = spawn_mock(router).await;
    let base = format!("http://{addr}");

    let connector = TelegramConnector::with_base("t", base).expect("build");
    let err = connector
        .send(Message::text("bogus", "x"))
        .await
        .expect_err("must error");
    match err {
        MessageError::ApiError { code, description } => {
            assert_eq!(code, "400");
            assert!(description.contains("chat not found"), "got: {description}");
        }
        other => panic!("expected ApiError, got: {other:?}"),
    }
}

// ── Test 3 — Slack bearer auth ────────────────────────────────────────────────

#[tokio::test]
async fn slack_authenticates_with_bearer_token() {
    install_crypto_provider();
    let state = MockState::default();
    async fn handler(
        State(s): State<MockState>,
        headers: HeaderMap,
        Json(_body): Json<Value>,
    ) -> Json<Value> {
        let auth =
            headers.get(axum::http::header::AUTHORIZATION).and_then(|v| v.to_str().ok());
        *s.last_auth.lock().expect("lock") = auth.map(str::to_owned);
        Json(json!({
            "ok": true,
            "ts": "1234.567",
            "channel": "C09ABC"
        }))
    }
    let router = Router::new()
        .route("/chat.postMessage", post(handler))
        .with_state(state.clone());
    let addr = spawn_mock(router).await;
    let base = format!("http://{addr}");

    let connector =
        SlackConnector::with_base("xoxb-test", base).expect("build slack");
    let id = connector
        .send(Message::text("C09ABC", "ping"))
        .await
        .expect("send ok");
    assert_eq!(id.0, "1234.567");

    let captured = state.last_auth.lock().expect("lock").clone().expect("auth");
    assert_eq!(captured, "Bearer xoxb-test");
}

// ── Test 4 — Slack rate limit ─────────────────────────────────────────────────

#[tokio::test]
async fn slack_propagates_rate_limit() {
    install_crypto_provider();
    async fn handler() -> (StatusCode, HeaderMap, Json<Value>) {
        let mut headers = HeaderMap::new();
        headers.insert(axum::http::header::RETRY_AFTER, "30".parse().expect("hdr"));
        (
            StatusCode::TOO_MANY_REQUESTS,
            headers,
            Json(json!({ "ok": false, "error": "ratelimited" })),
        )
    }
    let router = Router::new().route("/chat.postMessage", post(handler));
    let addr = spawn_mock(router).await;
    let base = format!("http://{addr}");

    let connector = SlackConnector::with_base("xoxb-x", base).expect("build");
    let err = connector
        .send(Message::text("C0", "x"))
        .await
        .expect_err("expected rate-limit error");
    match err {
        MessageError::RateLimit { retry_after_secs } => {
            assert_eq!(retry_after_secs, 30);
        }
        other => panic!("expected RateLimit, got: {other:?}"),
    }
}

// ── Test 5 — SMTP Message-ID format (non-wasm only) ───────────────────────────

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn email_constructs_message_id_per_rfc5322() {
    let c = SmtpConnector::new("smtp.example.com", 587, "user@x", "pass", TlsMode::Plain)
        .with_local_hostname("aphrody.test");
    let id = c.make_message_id();
    // RFC 5322 §3.6.4: msg-id = "<" id-left "@" id-right ">"
    assert!(id.starts_with('<') && id.ends_with('>'), "envelope: {id}");
    let inner = &id[1..id.len() - 1];
    let (left, right) = inner.split_once('@').expect("split @");
    assert_eq!(left.len(), 36, "UUID v4 left part");
    assert_eq!(right, "aphrody.test");
    // Idempotent format: two consecutive ids share envelope + right, differ on UUID.
    let id2 = c.make_message_id();
    assert_ne!(id, id2);
    assert!(id2.ends_with("@aphrody.test>"));
}

// ── Test 6 — dyn dispatch over Connector ──────────────────────────────────────

#[tokio::test]
async fn connector_dyn_dispatch_works() {
    install_crypto_provider();
    // Both connectors must be storable behind `Box<dyn Connector>`.
    let telegram: Box<dyn Connector> =
        Box::new(TelegramConnector::new("tok").expect("tg"));
    let slack: Box<dyn Connector> =
        Box::new(SlackConnector::new("xoxb").expect("slack"));

    let mut connectors: Vec<Box<dyn Connector>> = vec![telegram, slack];
    #[cfg(not(target_arch = "wasm32"))]
    {
        let smtp: Box<dyn Connector> = Box::new(SmtpConnector::new(
            "smtp.example.com",
            587,
            "u@x",
            "p",
            TlsMode::Starttls,
        ));
        connectors.push(smtp);
    }

    let ids: Vec<&'static str> = connectors.iter().map(|c| c.id()).collect();
    assert!(ids.contains(&"telegram"));
    assert!(ids.contains(&"slack"));
    #[cfg(not(target_arch = "wasm32"))]
    assert!(ids.contains(&"smtp"));
}
