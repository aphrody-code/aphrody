// SPDX-License-Identifier: Apache-2.0
//! Integration smoke tests for the LLM router and every whitelisted provider.
//!
//! Every test spins up a local axum server on an ephemeral port, captures the
//! exact HTTP request the provider produced (headers, URL, JSON body) and
//! asserts on the deserialised shape. No network I/O.

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;

use aphrody_router::{
    AnthropicProvider, AntigravityProvider, ChatMessage, ChatProvider, ChatRequest, GeminiProvider,
    ModelId, Provider, Role, Router, RouterError,
};

use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::{get, post};
use axum::{Json, Router as AxumRouter};
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
struct CaptureState {
    last_body: Arc<Mutex<Option<Value>>>,
    last_headers: Arc<Mutex<Option<HeaderMap>>>,
    last_path: Arc<Mutex<Option<String>>>,
}

async fn spawn_mock(router: AxumRouter) -> SocketAddr {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.expect("bind ephemeral port");
    let addr = listener.local_addr().expect("local_addr");
    tokio::spawn(async move {
        axum::serve(listener, router).await.ok();
    });
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    addr
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 1 — Provider enum rejects "openai" via serde
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn provider_enum_rejects_openai() {
    let err = serde_json::from_str::<Provider>(r#""openai""#).expect_err("must reject");
    let msg = err.to_string();
    // The reject path is rendered through RouterError::UnsupportedProvider, so
    // the lowercased rejected name appears in the error message.
    assert!(msg.contains("openai"), "error must name the rejected provider, got: {msg}");
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 2 — Provider enum rejects arbitrary unknown values
// ─────────────────────────────────────────────────────────────────────────────

#[test]
fn provider_enum_rejects_unknown() {
    for bad in ["xai", "perplexity", "groq", "together", "mistral", "cohere"] {
        let raw = format!(r#""{bad}""#);
        let err = serde_json::from_str::<Provider>(&raw)
            .expect_err(&format!("must reject {bad}"));
        assert!(
            err.to_string().contains(bad),
            "{bad}: error must name the provider, got: {err}"
        );
    }
    // And direct parse goes through the same path. `Provider::parse` now
    // returns `aphrody_providers::ProviderError` (re-exported via the
    // router's `pub use`) — the legacy `RouterError::UnsupportedProvider`
    // variant is still produced via `From` when `?`-converted inside the
    // router's own call sites (see `ModelId::parse`).
    assert!(matches!(
        Provider::parse("openai").unwrap_err(),
        aphrody_router::ProviderError::Unsupported(_)
    ));
    // And the From conversion still routes openai → RouterError::UnsupportedProvider.
    assert!(matches!(
        ModelId::parse("openai/whatever").unwrap_err(),
        RouterError::UnsupportedProvider(_)
    ));
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 3 — Anthropic provider sends x-api-key + anthropic-version
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn anthropic_provider_sends_correct_headers() {
    install_crypto_provider();
    let state = CaptureState::default();

    async fn handler(
        State(s): State<CaptureState>,
        headers: HeaderMap,
        Json(body): Json<Value>,
    ) -> Json<Value> {
        *s.last_body.lock().expect("lock") = Some(body);
        *s.last_headers.lock().expect("lock") = Some(headers);
        Json(json!({
            "id": "msg_01",
            "type": "message",
            "role": "assistant",
            "model": "claude-opus-4-7",
            "stop_reason": "end_turn",
            "content": [
                {"type": "text", "text": "hello back"}
            ],
            "usage": { "input_tokens": 12, "output_tokens": 3 }
        }))
    }

    let router = AxumRouter::new()
        .route("/v1/messages", post(handler))
        .with_state(state.clone());
    let addr = spawn_mock(router).await;

    let provider = AnthropicProvider::with_base_url("sk-test-123", format!("http://{addr}"))
        .expect("build provider");
    let req = ChatRequest::one_shot(
        ModelId::new(Provider::Anthropic, "claude-opus-4-7"),
        "hello",
    );
    let resp = provider.complete(req).await.expect("complete ok");
    assert_eq!(resp.content, "hello back");
    assert_eq!(resp.usage.prompt_tokens, 12);
    assert_eq!(resp.usage.completion_tokens, 3);

    let headers = state.last_headers.lock().expect("lock").clone().expect("headers");
    let api_key = headers.get("x-api-key").and_then(|v| v.to_str().ok()).expect("x-api-key");
    assert_eq!(api_key, "sk-test-123");
    let version = headers
        .get("anthropic-version")
        .and_then(|v| v.to_str().ok())
        .expect("anthropic-version");
    assert_eq!(version, "2023-06-01");
    let body = state.last_body.lock().expect("lock").clone().expect("body");
    assert_eq!(body["model"], "claude-opus-4-7");
    assert_eq!(body["messages"][0]["role"], "user");
    assert_eq!(body["messages"][0]["content"], "hello");
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 4 — Gemini provider hits the correct URL with x-goog-api-key
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn gemini_provider_sends_correct_url_and_key() {
    install_crypto_provider();
    let state = CaptureState::default();

    async fn handler(
        State(s): State<CaptureState>,
        Path(model_action): Path<String>,
        headers: HeaderMap,
        Json(body): Json<Value>,
    ) -> Json<Value> {
        *s.last_body.lock().expect("lock") = Some(body);
        *s.last_headers.lock().expect("lock") = Some(headers);
        *s.last_path.lock().expect("lock") = Some(model_action);
        Json(json!({
            "candidates": [
                {
                    "content": { "parts": [{"text": "pong"}], "role": "model" },
                    "finishReason": "STOP"
                }
            ],
            "modelVersion": "gemini-2.5-pro-preview",
            "usageMetadata": {
                "promptTokenCount": 5,
                "candidatesTokenCount": 1
            }
        }))
    }

    let router = AxumRouter::new()
        .route("/v1beta/models/{model_action}", post(handler))
        .with_state(state.clone());
    let addr = spawn_mock(router).await;

    let provider = GeminiProvider::with_base_url("AIza-test", format!("http://{addr}"))
        .expect("build provider");
    let req = ChatRequest::one_shot(
        ModelId::new(Provider::Gemini, "gemini-2.5-pro"),
        "ping",
    );
    let resp = provider.complete(req).await.expect("complete ok");
    assert_eq!(resp.content, "pong");
    assert_eq!(resp.usage.prompt_tokens, 5);
    assert_eq!(resp.usage.completion_tokens, 1);

    let path = state.last_path.lock().expect("lock").clone().expect("path");
    assert_eq!(path, "gemini-2.5-pro:generateContent");
    let headers = state.last_headers.lock().expect("lock").clone().expect("headers");
    let key = headers.get("x-goog-api-key").and_then(|v| v.to_str().ok()).expect("key");
    assert_eq!(key, "AIza-test");
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 5 — Antigravity provider sends Bearer auth
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn antigravity_provider_sends_bearer_auth() {
    install_crypto_provider();
    let state = CaptureState::default();

    async fn handler(
        State(s): State<CaptureState>,
        headers: HeaderMap,
        Json(body): Json<Value>,
    ) -> Json<Value> {
        *s.last_body.lock().expect("lock") = Some(body);
        *s.last_headers.lock().expect("lock") = Some(headers);
        Json(json!({
            "content": "vertex reply",
            "model": "antigravity-prime",
            "finish_reason": "stop",
            "usage": { "prompt_tokens": 4, "completion_tokens": 2 }
        }))
    }

    let router = AxumRouter::new()
        .route("/v1/messages", post(handler))
        .with_state(state.clone());
    let addr = spawn_mock(router).await;

    let provider =
        AntigravityProvider::with_base_url("ya29.token-xyz", format!("http://{addr}"))
            .expect("build provider");
    let mut req = ChatRequest::one_shot(
        ModelId::new(Provider::Antigravity, "antigravity-prime"),
        "ping",
    );
    req.messages.insert(0, ChatMessage { role: Role::System, content: "be terse".into(), tool_call_id: None });
    let resp = provider.complete(req).await.expect("complete ok");
    assert_eq!(resp.content, "vertex reply");

    let headers = state.last_headers.lock().expect("lock").clone().expect("headers");
    let auth = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .expect("auth");
    assert_eq!(auth, "Bearer ya29.token-xyz");
    let body = state.last_body.lock().expect("lock").clone().expect("body");
    assert_eq!(body["model"], "antigravity-prime");
    assert_eq!(body["messages"][0]["role"], "system");
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 6 — Router dispatches to the correct provider variant
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn router_dispatches_to_correct_provider() {
    install_crypto_provider();

    // Two mocks, two distinct hosts: anthropic-shaped + gemini-shaped.
    async fn anthropic_handler(Json(_): Json<Value>) -> Json<Value> {
        Json(json!({
            "id": "msg",
            "type": "message",
            "role": "assistant",
            "model": "claude-opus-4-7",
            "stop_reason": "end_turn",
            "content": [{"type": "text", "text": "from-anthropic"}],
            "usage": { "input_tokens": 1, "output_tokens": 1 }
        }))
    }
    let anth_router = AxumRouter::new().route("/v1/messages", post(anthropic_handler));
    let anth_addr = spawn_mock(anth_router).await;

    async fn gemini_handler(
        Path(_): Path<String>,
        Json(_): Json<Value>,
    ) -> Json<Value> {
        Json(json!({
            "candidates": [
                {"content": {"parts": [{"text": "from-gemini"}], "role": "model"}, "finishReason": "STOP"}
            ],
            "usageMetadata": {"promptTokenCount": 1, "candidatesTokenCount": 1}
        }))
    }
    let gem_router = AxumRouter::new().route("/v1beta/models/{model_action}", post(gemini_handler));
    let gem_addr = spawn_mock(gem_router).await;

    let mut router = Router::new();
    router.add_provider(Box::new(
        AnthropicProvider::with_base_url("anth-key", format!("http://{anth_addr}")).expect("anth"),
    ));
    router.add_provider(Box::new(
        GeminiProvider::with_base_url("gem-key", format!("http://{gem_addr}")).expect("gem"),
    ));

    let anth_resp = router
        .route(ChatRequest::one_shot(
            ModelId::new(Provider::Anthropic, "claude-opus-4-7"),
            "ping",
        ))
        .await
        .expect("route anthropic");
    assert_eq!(anth_resp.content, "from-anthropic");

    let gem_resp = router
        .route(ChatRequest::one_shot(
            ModelId::new(Provider::Gemini, "gemini-2.5-pro"),
            "ping",
        ))
        .await
        .expect("route gemini");
    assert_eq!(gem_resp.content, "from-gemini");

    // Unregistered provider → NoSuchProvider.
    let err = router
        .route(ChatRequest::one_shot(
            ModelId::new(Provider::Antigravity, "antigravity-prime"),
            "ping",
        ))
        .await
        .expect_err("expected NoSuchProvider");
    assert!(matches!(err, RouterError::NoSuchProvider(Provider::Antigravity)));
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 7 — 429 response surfaces RateLimit { retry_after_ms }
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn rate_limit_429_returns_retry_after() {
    install_crypto_provider();

    async fn handler() -> (StatusCode, HeaderMap, Json<Value>) {
        let mut headers = HeaderMap::new();
        headers.insert(axum::http::header::RETRY_AFTER, "42".parse().expect("hdr"));
        (
            StatusCode::TOO_MANY_REQUESTS,
            headers,
            Json(json!({"error": {"message": "slow down"}})),
        )
    }
    let router = AxumRouter::new().route("/v1/messages", post(handler));
    let addr = spawn_mock(router).await;

    let provider = AnthropicProvider::with_base_url("k", format!("http://{addr}")).expect("build");
    let err = provider
        .complete(ChatRequest::one_shot(
            ModelId::new(Provider::Anthropic, "claude-opus-4-7"),
            "x",
        ))
        .await
        .expect_err("expected rate-limit");
    match err {
        RouterError::RateLimit { retry_after_ms } => {
            assert_eq!(retry_after_ms, Some(42_000));
        }
        other => panic!("expected RateLimit, got {other:?}"),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Bonus — health endpoint exercised end-to-end (Gemini)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn gemini_health_succeeds_on_200() {
    install_crypto_provider();
    async fn handler() -> Json<Value> {
        Json(json!({ "models": [] }))
    }
    let router = AxumRouter::new().route("/v1beta/models", get(handler));
    let addr = spawn_mock(router).await;
    let provider =
        GeminiProvider::with_base_url("k", format!("http://{addr}")).expect("build");
    provider.health().await.expect("health ok");
}
