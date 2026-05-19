// SPDX-License-Identifier: Apache-2.0
//! End-to-end smoke tests for the aphrody-session crate.
//!
//! Covers:
//!
//! * Aggregation invariants (`total_tokens`, `total_cost_usd`).
//! * JSONL round-trip (`Session::to_jsonl` ↔ `Session::from_jsonl`).
//! * Filesystem store round-trip (`save_session` / `load_session` / `list`
//!   / `delete`) via a [`tempfile::TempDir`] so tests are hermetic and run
//!   concurrently without stepping on each other.
//! * Deterministic [`replay`] walk preserves insertion order.
//! * Corrupt JSONL input surfaces a typed [`SessionError::InvalidJsonl`].

use std::sync::{Arc, Mutex};

use aphrody_session::{
    JsonlFileStore, Session, SessionError, SessionEvent, SessionId, SessionStore, ToolCall, Turn,
    TurnRole, replay,
};
use chrono::{TimeZone, Utc};
use tempfile::TempDir;

/// Build a populated session: 1 start (auto) + 2 turns + 1 tool call + 1 end.
fn fixture_session() -> Session {
    let id = SessionId::new("smoke-test-session-001").unwrap();
    let mut s = Session::new(id, serde_json::json!({"model": "claude-opus-4-7"}));
    s.add_turn(Turn {
        id: "t1".into(),
        role: TurnRole::User,
        content: "hello".into(),
        ts: Utc.with_ymd_and_hms(2026, 5, 19, 12, 0, 0).unwrap(),
        prompt_tokens: 0,
        completion_tokens: 0,
        cost_usd: 0.0,
    });
    s.add_turn(Turn {
        id: "t2".into(),
        role: TurnRole::Assistant,
        content: "hi back".into(),
        ts: Utc.with_ymd_and_hms(2026, 5, 19, 12, 0, 5).unwrap(),
        prompt_tokens: 120,
        completion_tokens: 30,
        cost_usd: 0.0042,
    });
    s.add_tool_call(ToolCall {
        id: "call_abc".into(),
        name: "Read".into(),
        args: serde_json::json!({"path": "/tmp/foo"}),
        result: Some(serde_json::json!({"ok": true})),
        error: None,
        ts: Utc.with_ymd_and_hms(2026, 5, 19, 12, 0, 7).unwrap(),
        duration_ms: 42,
    });
    s.end();
    s
}

#[test]
fn add_turn_increments_total_tokens() {
    let mut s = Session::new(SessionId::generate(), serde_json::json!({}));
    assert_eq!(s.total_tokens(), 0);
    s.add_turn(Turn {
        id: "t1".into(),
        role: TurnRole::Assistant,
        content: "x".into(),
        ts: Utc::now(),
        prompt_tokens: 100,
        completion_tokens: 50,
        cost_usd: 0.0,
    });
    assert_eq!(s.total_tokens(), 150);
    s.add_turn(Turn {
        id: "t2".into(),
        role: TurnRole::Assistant,
        content: "y".into(),
        ts: Utc::now(),
        prompt_tokens: 7,
        completion_tokens: 3,
        cost_usd: 0.0,
    });
    assert_eq!(s.total_tokens(), 160);
}

#[test]
fn total_cost_aggregates_turn_costs() {
    let mut s = Session::new(SessionId::generate(), serde_json::json!({}));
    for (p, c, cost) in [(10u32, 5u32, 0.01_f64), (20, 10, 0.02), (30, 15, 0.03)] {
        s.add_turn(Turn {
            id: format!("turn-{p}"),
            role: TurnRole::Assistant,
            content: String::new(),
            ts: Utc::now(),
            prompt_tokens: p,
            completion_tokens: c,
            cost_usd: cost,
        });
    }
    // Tool calls must NOT contribute to cost.
    s.add_tool_call(ToolCall {
        id: "nope".into(),
        name: "Read".into(),
        args: serde_json::json!({}),
        result: None,
        error: None,
        ts: Utc::now(),
        duration_ms: 0,
    });
    assert!((s.total_cost_usd() - 0.06).abs() < 1e-9);
}

#[test]
fn to_jsonl_roundtrip() {
    let s = fixture_session();
    let jsonl = s.to_jsonl().expect("serialize");
    // Expect exactly 5 lines: start + 2 turns + 1 tool call + end.
    let line_count = jsonl.lines().count();
    assert_eq!(line_count, 5, "got jsonl:\n{jsonl}");
    let back = Session::from_jsonl(&jsonl).expect("parse");
    assert_eq!(back.id, s.id);
    assert_eq!(back.events.len(), s.events.len());
    assert_eq!(back.total_tokens(), s.total_tokens());
    assert!((back.total_cost_usd() - s.total_cost_usd()).abs() < 1e-12);
    // Spot-check round-trip equality for the structured tool call.
    let original_call = s
        .events
        .iter()
        .find_map(|e| if let SessionEvent::ToolCall(c) = e { Some(c) } else { None })
        .unwrap();
    let parsed_call = back
        .events
        .iter()
        .find_map(|e| if let SessionEvent::ToolCall(c) = e { Some(c) } else { None })
        .unwrap();
    assert_eq!(original_call, parsed_call);
}

#[tokio::test]
async fn jsonl_store_save_load_roundtrip() {
    let dir = TempDir::new().unwrap();
    let store = JsonlFileStore::new(dir.path());
    let s = fixture_session();
    store.save_session(&s).await.expect("save");
    // Verify the file landed on disk with the expected name.
    let expected = dir.path().join(format!("{}.jsonl", s.id.as_str()));
    assert!(expected.is_file(), "expected {expected:?} to exist");
    let back = store.load_session(&s.id).await.expect("load");
    assert_eq!(back.id, s.id);
    assert_eq!(back.events.len(), s.events.len());
    assert_eq!(back.total_tokens(), s.total_tokens());
}

#[tokio::test]
async fn jsonl_store_list_returns_all_session_ids() {
    let dir = TempDir::new().unwrap();
    let store = JsonlFileStore::new(dir.path());
    let ids: Vec<SessionId> = (0..3)
        .map(|i| SessionId::new(format!("list-test-{i:02}")).unwrap())
        .collect();
    for id in &ids {
        let mut s = Session::new(id.clone(), serde_json::json!({}));
        s.add_turn(Turn {
            id: "t1".into(),
            role: TurnRole::User,
            content: "x".into(),
            ts: Utc::now(),
            prompt_tokens: 0,
            completion_tokens: 0,
            cost_usd: 0.0,
        });
        store.save_session(&s).await.unwrap();
    }
    // Drop a non-jsonl file alongside the sessions to ensure the lister
    // ignores it.
    tokio::fs::write(dir.path().join("README.md"), b"ignore me").await.unwrap();
    let listed = store.list_sessions().await.expect("list");
    // Sorted lexicographically by `JsonlFileStore::list_sessions`.
    let mut expected = ids.clone();
    expected.sort_by(|a, b| a.as_str().cmp(b.as_str()));
    assert_eq!(listed, expected);
}

#[tokio::test]
async fn jsonl_store_delete_removes_file() {
    let dir = TempDir::new().unwrap();
    let store = JsonlFileStore::new(dir.path());
    let s = fixture_session();
    store.save_session(&s).await.unwrap();
    let path = dir.path().join(format!("{}.jsonl", s.id.as_str()));
    assert!(path.is_file());
    store.delete_session(&s.id).await.expect("delete");
    assert!(!path.exists());
    // Second delete must surface NotFound.
    let err = store.delete_session(&s.id).await.unwrap_err();
    assert!(matches!(err, SessionError::NotFound(_)));
}

#[tokio::test]
async fn jsonl_store_load_missing_returns_not_found() {
    let dir = TempDir::new().unwrap();
    let store = JsonlFileStore::new(dir.path());
    let missing = SessionId::new("does-not-exist").unwrap();
    let err = store.load_session(&missing).await.unwrap_err();
    assert!(matches!(err, SessionError::NotFound(_)));
}

#[test]
fn replay_visits_all_events_in_order() {
    let s = fixture_session();
    let seen = Arc::new(Mutex::new(Vec::<&'static str>::new()));
    let seen_for_cb = Arc::clone(&seen);
    let count = replay(&s, move |ev| {
        let tag: &'static str = match ev {
            SessionEvent::SessionStart { .. } => "start",
            SessionEvent::SessionEnd { .. } => "end",
            SessionEvent::Turn(_) => "turn",
            SessionEvent::ToolCall(_) => "tool",
        };
        seen_for_cb.lock().unwrap().push(tag);
    });
    assert_eq!(count, s.events.len());
    assert_eq!(
        *seen.lock().unwrap(),
        vec!["start", "turn", "turn", "tool", "end"],
    );
}

#[test]
fn corrupt_jsonl_line_returns_invalid_jsonl_error() {
    // Build a valid prelude line (SessionStart) so the empty-events branch
    // doesn't fire, then drop a non-JSON line that must trip the parser.
    let mut s = Session::new(SessionId::new("corrupt").unwrap(), serde_json::json!({}));
    s.add_turn(Turn {
        id: "t1".into(),
        role: TurnRole::User,
        content: "x".into(),
        ts: Utc::now(),
        prompt_tokens: 0,
        completion_tokens: 0,
        cost_usd: 0.0,
    });
    let mut jsonl = s.to_jsonl().unwrap();
    jsonl.push_str("{not valid json\n");
    let err = Session::from_jsonl(&jsonl).unwrap_err();
    match err {
        SessionError::InvalidJsonl(msg) => {
            assert!(msg.contains("line"), "expected 'line N' in error, got: {msg}");
        }
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn empty_jsonl_returns_empty_session_error() {
    let err = Session::from_jsonl("").unwrap_err();
    assert!(matches!(err, SessionError::EmptySession));
}
