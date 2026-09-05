// SPDX-License-Identifier: Apache-2.0
//! Tier-1 MemoryProvider smoke tests — SqliteLocal + axum-mocked Mem0/Honcho.
//!
//! Network-free: HTTP providers point at a local axum router bound to an
//! ephemeral port. Same pattern as `crates/aphrody-messaging/tests/connector_smoke.rs`.

use std::sync::Arc;
use std::sync::Mutex;

use aphrody_memory::honcho::HonchoProvider;
use aphrody_memory::mem0::Mem0Provider;
use aphrody_memory::provider::MemoryProvider;
use aphrody_memory::sqlite_local::SqliteLocalProvider;
use aphrody_memory::types::{MemoryQuery, MemoryRecord, ProviderKind};

use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde_json::{Value, json};
use tokio::net::TcpListener;

// ─────────────────────────────────────────────────────────────────────────────
// Test helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Install the rustls ring CryptoProvider once for the whole test binary.
fn install_crypto_provider() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

#[derive(Clone, Default)]
struct Captured {
    auth: Arc<Mutex<Option<String>>>,
    body: Arc<Mutex<Option<Value>>>,
    last_path: Arc<Mutex<Option<String>>>,
}

async fn spawn(router: Router) -> String {
    let listener = TcpListener::bind(("127.0.0.1", 0)).await.expect("bind");
    let addr = listener.local_addr().expect("local_addr");
    tokio::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    // Tiny wait for the listener to start accepting.
    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    format!("http://{addr}")
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 1 — sqlite_local
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn sqlite_local() {
    let provider = SqliteLocalProvider::open_in_memory().expect("open in-memory");

    // Add 3 records for agent-A, one for agent-B (noise).
    let rec1 = MemoryRecord::new("", "agent-A", "loves coffee")
        .with_tags(vec!["preference".into()]);
    let rec2 = MemoryRecord::new("", "agent-A", "lives in Paris")
        .with_tags(vec!["fact".into()]);
    let rec3 = MemoryRecord::new("", "agent-A", "buy milk tomorrow")
        .with_tags(vec!["todo".into()]);
    let rec4 = MemoryRecord::new("", "agent-B", "unrelated noise");

    let id1 = provider.add(rec1).await.expect("add 1");
    let id2 = provider.add(rec2).await.expect("add 2");
    let id3 = provider.add(rec3).await.expect("add 3");
    let _id4 = provider.add(rec4).await.expect("add 4 noise");

    assert!(!id1.is_empty() && !id2.is_empty() && !id3.is_empty());

    // Get one — must round-trip content + tags.
    let got = provider.get(&id2).await.expect("get").expect("present");
    assert_eq!(got.id, id2);
    assert_eq!(got.agent_id, "agent-A");
    assert_eq!(got.content, "lives in Paris");
    assert_eq!(got.tags, vec!["fact".to_string()]);

    // Search by agent_id — must return exactly 3 (agent-B excluded).
    let hits = provider
        .search(MemoryQuery::for_agent("agent-A"))
        .await
        .expect("search agent");
    assert_eq!(hits.len(), 3, "expected 3 records for agent-A, got {}", hits.len());

    // Search with keyword filter.
    let hits = provider
        .search(MemoryQuery {
            agent_id: "agent-A".into(),
            q: Some("Paris".into()),
            tags: vec![],
            limit: None,
        })
        .await
        .expect("search Paris");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, id2);

    // Search with tag filter.
    let hits = provider
        .search(MemoryQuery {
            agent_id: "agent-A".into(),
            q: None,
            tags: vec!["todo".into()],
            limit: None,
        })
        .await
        .expect("search tag");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].content, "buy milk tomorrow");

    // Delete one — must drop count from 3 to 2.
    provider.delete(&id1).await.expect("delete");
    let hits = provider
        .search(MemoryQuery::for_agent("agent-A"))
        .await
        .expect("re-search");
    assert_eq!(hits.len(), 2);

    // Health must be Ok.
    provider.health().await.expect("health");
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 2 — mem0_smoke (axum mock)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn mem0_smoke() {
    install_crypto_provider();
    let captured = Captured::default();

    async fn add_handler(
        State(s): State<Captured>,
        headers: HeaderMap,
        Json(body): Json<Value>,
    ) -> Json<Value> {
        *s.auth.lock().expect("lock") =
            headers.get("authorization").and_then(|v| v.to_str().ok()).map(str::to_string);
        *s.body.lock().expect("lock") = Some(body);
        Json(json!({ "id": "mem_123" }))
    }

    async fn search_handler(
        State(s): State<Captured>,
        headers: HeaderMap,
        Query(params): Query<Vec<(String, String)>>,
    ) -> Json<Value> {
        *s.auth.lock().expect("lock") =
            headers.get("authorization").and_then(|v| v.to_str().ok()).map(str::to_string);
        let user_id = params
            .iter()
            .find(|(k, _)| k == "user_id")
            .map(|(_, v)| v.clone())
            .unwrap_or_default();
        Json(json!([{
            "id": "mem_456",
            "user_id": user_id,
            "memory": "echo back match",
            "categories": ["fact"],
            "created_at": "2026-05-19T12:00:00Z",
            "metadata": {}
        }]))
    }

    let router = Router::new()
        .route("/v1/memories/", post(add_handler))
        .route("/v1/memories/search", get(search_handler))
        .with_state(captured.clone());
    let base = spawn(router).await;

    let provider =
        Mem0Provider::with_base_url("fake_key", &base).expect("build mem0");

    // Add → must return server-assigned id "mem_123".
    let rec = MemoryRecord::new("", "user-1", "remember this fact")
        .with_tags(vec!["fact".into()]);
    let id = provider.add(rec).await.expect("add");
    assert_eq!(id, "mem_123");

    // Auth header must be `Token fake_key`.
    let auth = captured.auth.lock().expect("lock").clone();
    assert_eq!(auth.as_deref(), Some("Token fake_key"));

    // Body must include user_id + messages array.
    let body = captured.body.lock().expect("lock").clone().expect("body");
    assert_eq!(body["user_id"], "user-1");
    assert!(body["messages"].is_array());
    assert_eq!(body["messages"][0]["content"], "remember this fact");

    // Search → must return exactly 1 record.
    let hits = provider
        .search(MemoryQuery::for_agent("user-1"))
        .await
        .expect("search");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, "mem_456");
    assert_eq!(hits[0].content, "echo back match");
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 3 — honcho_smoke (axum mock)
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn honcho_smoke() {
    install_crypto_provider();
    let captured = Captured::default();

    async fn post_msg(
        State(s): State<Captured>,
        Path((app_id, user_id, session_id)): Path<(String, String, String)>,
        headers: HeaderMap,
        Json(body): Json<Value>,
    ) -> Json<Value> {
        *s.auth.lock().expect("lock") =
            headers.get("authorization").and_then(|v| v.to_str().ok()).map(str::to_string);
        *s.body.lock().expect("lock") = Some(body);
        *s.last_path.lock().expect("lock") =
            Some(format!("/v1/apps/{app_id}/users/{user_id}/sessions/{session_id}/messages"));
        Json(json!({ "id": "hmsg_999" }))
    }

    async fn list_msg(
        State(s): State<Captured>,
        Path((_app, user, _sess)): Path<(String, String, String)>,
        headers: HeaderMap,
    ) -> Json<Value> {
        *s.auth.lock().expect("lock") =
            headers.get("authorization").and_then(|v| v.to_str().ok()).map(str::to_string);
        Json(json!({
            "items": [{
                "id": "hmsg_111",
                "content": format!("echo for {user}"),
                "created_at": "2026-05-19T12:00:00Z",
                "metadata": { "tags": ["fact"] }
            }]
        }))
    }

    let router = Router::new()
        .route(
            "/v1/apps/{app_id}/users/{user_id}/sessions/{session_id}/messages",
            post(post_msg).get(list_msg),
        )
        .with_state(captured.clone());
    let base = spawn(router).await;

    let provider =
        HonchoProvider::with_base_url("fake_key", &base, "default").expect("build honcho");

    // Add a record.
    let rec = MemoryRecord::new("", "user-X", "honcho remember")
        .with_tags(vec!["fact".into()]);
    let id = provider.add(rec).await.expect("add");
    assert_eq!(id, "hmsg_999");

    // Auth header must be `Bearer fake_key`.
    let auth = captured.auth.lock().expect("lock").clone();
    assert_eq!(auth.as_deref(), Some("Bearer fake_key"));

    // Body must include `content` and `is_user: true`.
    let body = captured.body.lock().expect("lock").clone().expect("body");
    assert_eq!(body["content"], "honcho remember");
    assert_eq!(body["is_user"], true);

    // Search must return exactly 1 record with the fact tag round-tripped.
    let hits = provider
        .search(MemoryQuery::for_agent("user-X"))
        .await
        .expect("search");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].id, "hmsg_111");
    assert_eq!(hits[0].tags, vec!["fact".to_string()]);
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 4 — dyn_dispatch_works
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn dyn_dispatch_works() {
    let provider: Box<dyn MemoryProvider> =
        Box::new(SqliteLocalProvider::open_in_memory().expect("open"));

    let rec = MemoryRecord::new("", "agent-dyn", "dispatched record")
        .with_tags(vec!["dyn".into()]);
    let id = provider.add(rec).await.expect("dyn add");
    assert!(!id.is_empty());

    let hits = provider
        .search(MemoryQuery::for_agent("agent-dyn"))
        .await
        .expect("dyn search");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].content, "dispatched record");
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 5 — provider_kind_correct
// ─────────────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn provider_kind_correct() {
    install_crypto_provider();

    let sqlite = SqliteLocalProvider::open_in_memory().expect("open");
    assert_eq!(sqlite.provider_kind(), ProviderKind::SqliteLocal);

    let mem0 = Mem0Provider::with_base_url("k", "http://localhost:1").expect("mem0");
    assert_eq!(mem0.provider_kind(), ProviderKind::Mem0);

    let honcho = HonchoProvider::with_base_url("k", "http://localhost:1", "default")
        .expect("honcho");
    assert_eq!(honcho.provider_kind(), ProviderKind::Honcho);
}
