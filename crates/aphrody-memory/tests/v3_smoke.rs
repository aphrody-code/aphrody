// SPDX-License-Identifier: Apache-2.0
//! R3.8 — axum-mocked smoke tests for the v3 provider adapters.
//!
//! Network-free: the Honcho v3 / Mem0 v3 HTTP providers point at a local axum
//! router bound to an ephemeral port. Same pattern as `memory_smoke.rs`.
//!
//! These exercise the real wire contract end to end:
//! - Mem0 v3 async add returns `202 {event_id}`, then `add` polls
//!   `GET /v1/event/{id}/` (PENDING → DONE) and resolves the memory id;
//! - Mem0 v3 search routes keyword queries to `POST /v2/memories/search/` and
//!   bare owner queries to `POST /v3/memories/` (list);
//! - Honcho v3 add posts to `.../sessions/{sess}/messages/` and search drives
//!   the dialectic `.../peers/{peer}/chat` endpoint.

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use aphrody_memory::mem0_v3::{EventOutcome, Mem0V3Provider};
use aphrody_memory::honcho_v3::HonchoV3Provider;
use aphrody_memory::provider::MemoryProvider;
use aphrody_memory::types::{MemoryQuery, MemoryRecord};

use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};
use tokio::net::TcpListener;

// ─────────────────────────────────────────────────────────────────────────────
// Shared helpers
// ─────────────────────────────────────────────────────────────────────────────

fn install_crypto_provider() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

async fn spawn(router: Router) -> String {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    format!("http://{addr}")
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 1 — Mem0 v3 async add → poll → resolve memory id, then search.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Default)]
struct Mem0State {
    auth: Arc<Mutex<Option<String>>>,
    add_body: Arc<Mutex<Option<Value>>>,
    poll_count: Arc<AtomicU32>,
    last_search_path: Arc<Mutex<Option<String>>>,
}

#[tokio::test]
async fn mem0_v3_async_add_polls_then_search() {
    install_crypto_provider();
    let state = Mem0State::default();

    async fn add_handler(
        State(s): State<Mem0State>,
        headers: HeaderMap,
        Json(body): Json<Value>,
    ) -> (axum::http::StatusCode, Json<Value>) {
        *s.auth.lock().expect("lock") =
            headers.get("authorization").and_then(|v| v.to_str().ok()).map(str::to_string);
        *s.add_body.lock().expect("lock") = Some(body);
        (
            axum::http::StatusCode::ACCEPTED,
            Json(json!({
                "message": "queued",
                "status": "PENDING",
                "event_id": "evt-async-1"
            })),
        )
    }

    async fn event_handler(
        State(s): State<Mem0State>,
        Path(event_id): Path<String>,
    ) -> Json<Value> {
        // First poll: still pending. Second poll onward: done with an id.
        let n = s.poll_count.fetch_add(1, Ordering::SeqCst);
        if n == 0 {
            Json(json!({ "event_id": event_id, "status": "PENDING" }))
        } else {
            Json(json!({
                "event_id": event_id,
                "status": "completed",
                "memory_id": "mem_resolved_42"
            }))
        }
    }

    async fn search_handler(State(s): State<Mem0State>, Json(_body): Json<Value>) -> Json<Value> {
        *s.last_search_path.lock().expect("lock") = Some("/v2/memories/search/".into());
        Json(json!({
            "results": [{ "id": "mem_s1", "memory": "prefers dark mode", "score": 0.95 }],
            "count": 1
        }))
    }

    let router = Router::new()
        .route("/v3/memories/add/", post(add_handler))
        .route("/v1/event/{event_id}/", get(event_handler))
        .route("/v2/memories/search/", post(search_handler))
        .with_state(state.clone());
    let base = spawn(router).await;

    let provider = Mem0V3Provider::with_base_url("fake_key", &base).expect("build");

    // add → async POST + poll loop must resolve the server-side memory id.
    let rec = MemoryRecord::new("", "user-1", "I prefer dark mode")
        .with_tags(vec!["preference".into()]);
    let id = provider.add(rec).await.expect("add");
    assert_eq!(id, "mem_resolved_42");

    // Auth header is `Token fake_key`.
    assert_eq!(state.auth.lock().expect("lock").as_deref(), Some("Token fake_key"));

    // Add body carried the user_id + messages + categories.
    let body = state.add_body.lock().expect("lock").clone().expect("body");
    assert_eq!(body["user_id"], "user-1");
    assert_eq!(body["messages"][0]["content"], "I prefer dark mode");
    assert_eq!(body["messages"][0]["role"], "user");
    assert_eq!(body["categories"][0], "preference");

    // Polled at least twice (one PENDING, one DONE).
    assert!(state.poll_count.load(Ordering::SeqCst) >= 2);

    // Keyword search hits v2 search and returns the one row.
    let hits = provider
        .search(MemoryQuery { agent_id: "user-1".into(), q: Some("dark".into()), tags: vec![], limit: Some(5) })
        .await
        .expect("search");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, "mem_s1");
    assert_eq!(hits[0].content, "prefers dark mode");
    assert_eq!(state.last_search_path.lock().expect("lock").as_deref(), Some("/v2/memories/search/"));
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 2 — Mem0 v3 bare-owner search routes to the v3 list endpoint.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Default)]
struct RouteProbe {
    hit_list: Arc<Mutex<Option<Value>>>,
}

#[tokio::test]
async fn mem0_v3_owner_only_search_uses_list_endpoint() {
    install_crypto_provider();
    let probe = RouteProbe::default();

    async fn list_handler(State(s): State<RouteProbe>, Json(body): Json<Value>) -> Json<Value> {
        *s.hit_list.lock().expect("lock") = Some(body);
        Json(json!({
            "count": 1,
            "next": null,
            "previous": null,
            "results": [{ "id": "mem_l1", "memory": "lives in Paris", "user_id": "user-2" }]
        }))
    }

    let router = Router::new()
        .route("/v3/memories/", post(list_handler))
        .with_state(probe.clone());
    let base = spawn(router).await;

    let provider = Mem0V3Provider::with_base_url("k", &base).expect("build");

    // No keyword → list path.
    let hits = provider.search(MemoryQuery::for_agent("user-2")).await.expect("list");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].content, "lives in Paris");

    // The list body filtered by the owner id.
    let body = probe.hit_list.lock().expect("lock").clone().expect("body");
    assert_eq!(body["filters"]["user_id"], "user-2");
    assert_eq!(body["page"], 1);
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 3 — Mem0 v3 failed event surfaces an error.
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn mem0_v3_failed_event_is_error() {
    install_crypto_provider();

    async fn add_handler() -> (axum::http::StatusCode, Json<Value>) {
        (
            axum::http::StatusCode::ACCEPTED,
            Json(json!({ "status": "PENDING", "event_id": "evt-bad" })),
        )
    }
    async fn event_handler(Path(_e): Path<String>) -> Json<Value> {
        Json(json!({ "status": "FAILED" }))
    }

    let router = Router::new()
        .route("/v3/memories/add/", post(add_handler))
        .route("/v1/event/{event_id}/", get(event_handler));
    let base = spawn(router).await;

    let provider = Mem0V3Provider::with_base_url("k", &base).expect("build");
    let rec = MemoryRecord::new("", "user-3", "doomed write");
    let err = provider.add(rec).await.expect_err("must fail");
    let msg = err.to_string();
    assert!(msg.contains("failed"), "unexpected error: {msg}");

    // The public poll helper classifies the same event as Failed.
    let outcome = provider.poll_event("evt-bad").await.expect("poll");
    assert!(matches!(outcome, EventOutcome::Failed { .. }));
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 4 — Honcho v3 add (messages) + dialectic search + health.
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Clone, Default)]
struct HonchoState {
    auth: Arc<Mutex<Option<String>>>,
    msg_body: Arc<Mutex<Option<Value>>>,
    chat_body: Arc<Mutex<Option<Value>>>,
    msg_path: Arc<Mutex<Option<String>>>,
}

#[tokio::test]
async fn honcho_v3_messages_add_and_dialectic_search() {
    install_crypto_provider();
    let state = HonchoState::default();

    async fn post_msg(
        State(s): State<HonchoState>,
        Path((ws, sess)): Path<(String, String)>,
        headers: HeaderMap,
        Json(body): Json<Value>,
    ) -> Json<Value> {
        *s.auth.lock().expect("lock") =
            headers.get("authorization").and_then(|v| v.to_str().ok()).map(str::to_string);
        *s.msg_body.lock().expect("lock") = Some(body);
        *s.msg_path.lock().expect("lock") =
            Some(format!("/v3/workspaces/{ws}/sessions/{sess}/messages/"));
        Json(json!({ "messages": [{ "id": "hmsg_v3_1" }] }))
    }

    async fn chat(
        State(s): State<HonchoState>,
        Path((_ws, _peer)): Path<(String, String)>,
        Json(body): Json<Value>,
    ) -> Json<Value> {
        *s.chat_body.lock().expect("lock") = Some(body);
        Json(json!({ "content": "This peer is an expert in Rust systems programming." }))
    }

    async fn workspace_root(Path(_ws): Path<String>) -> Json<Value> {
        Json(json!({ "id": "default" }))
    }

    let router = Router::new()
        .route(
            "/v3/workspaces/{workspace_id}/sessions/{session_id}/messages/",
            post(post_msg),
        )
        .route("/v3/workspaces/{workspace_id}/peers/{peer_id}/chat", post(chat))
        .route("/v3/workspaces/{workspace_id}", get(workspace_root))
        .with_state(state.clone());
    let base = spawn(router).await;

    let provider = HonchoV3Provider::with_base_url("fake_key", &base, "default").expect("build");

    // add → posts a message tagged with its peer, parses the id out of the
    // `{messages:[{id}]}` envelope.
    let rec = MemoryRecord::new("", "peer-X", "knows rust deeply")
        .with_tags(vec!["skill".into()]);
    let id = provider.add(rec).await.expect("add");
    assert_eq!(id, "hmsg_v3_1");

    assert_eq!(state.auth.lock().expect("lock").as_deref(), Some("Bearer fake_key"));
    let body = state.msg_body.lock().expect("lock").clone().expect("body");
    assert_eq!(body["messages"][0]["content"], "knows rust deeply");
    assert_eq!(body["messages"][0]["peer_id"], "peer-X");
    assert_eq!(body["messages"][0]["metadata"]["tags"][0], "skill");
    assert_eq!(
        state.msg_path.lock().expect("lock").as_deref(),
        Some("/v3/workspaces/default/sessions/default/messages/")
    );

    // search → dialectic chat; one synthesised record carrying the answer.
    let hits = provider
        .search(MemoryQuery { agent_id: "peer-X".into(), q: Some("rust?".into()), tags: vec![], limit: None })
        .await
        .expect("search");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].content, "This peer is an expert in Rust systems programming.");
    assert_eq!(hits[0].agent_id, "peer-X");
    let chat_body = state.chat_body.lock().expect("lock").clone().expect("chat body");
    assert_eq!(chat_body["query"], "rust?");
    assert_eq!(chat_body["stream"], false);
    assert_eq!(chat_body["session_id"], "default");

    // health → GET workspace root, 200 is healthy.
    provider.health().await.expect("health");
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 5 — Honcho v3 empty dialectic answer yields no rows.
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn honcho_v3_empty_dialectic_is_empty() {
    install_crypto_provider();

    async fn chat(Path((_ws, _peer)): Path<(String, String)>, Json(_b): Json<Value>) -> Json<Value> {
        Json(json!({ "content": null }))
    }

    let router = Router::new()
        .route("/v3/workspaces/{workspace_id}/peers/{peer_id}/chat", post(chat));
    let base = spawn(router).await;

    let provider = HonchoV3Provider::with_base_url("k", &base, "default").expect("build");
    let hits = provider.search(MemoryQuery::for_agent("peer-Y")).await.expect("search");
    assert!(hits.is_empty());
}
