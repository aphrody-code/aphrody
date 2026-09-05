// SPDX-License-Identifier: Apache-2.0
//! Offline, deterministic tests for the multi-agent control plane.
//!
//! No `reqwest::Client` / `GeminiClient` is ever constructed (the
//! [`MockModelClient`] is a scripted in-memory stream), so the rustls
//! "No provider set" panic (CLAUDE.md §7) cannot occur. Every event drain is
//! wrapped in a short [`tokio::time::timeout`] so a regression can never hang
//! the test binary.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use aphrody_agent_proto::{Event, EventMsg};
use aphrody_engine::{EngineConfig, Session};
use aphrody_model_client::{
    ChatTurn, ModelClient, ModelError, ModelStream, ModelStreamEvent,
};
use aphrody_toolcall::ToolRegistry;
use async_trait::async_trait;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::time::timeout;

use crate::{AgentId, Supervisor, SupervisorError};

/// Hard ceiling on any single `recv().await` in a drain loop.
const RECV_TIMEOUT: Duration = Duration::from_secs(5);

/// A model client that replays one pre-scripted turn per `stream` call.
struct MockModelClient {
    turns: Mutex<VecDeque<Vec<ModelStreamEvent>>>,
}

impl MockModelClient {
    /// Build a client that emits `events` (a single turn) on its first stream
    /// call, then a bare `Completed` afterwards.
    fn single_turn(events: Vec<ModelStreamEvent>) -> Arc<dyn ModelClient> {
        Arc::new(Self {
            turns: Mutex::new(VecDeque::from([events])),
        })
    }
}

#[async_trait]
impl ModelClient for MockModelClient {
    async fn stream(&self, _turn: ChatTurn) -> Result<ModelStream, ModelError> {
        let events = self
            .turns
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .pop_front()
            .unwrap_or_else(|| {
                vec![ModelStreamEvent::Completed {
                    usage: None,
                    finish_reason: Some("STOP".to_string()),
                }]
            });
        let stream = futures::stream::iter(events.into_iter().map(Ok));
        Ok(Box::pin(stream))
    }

    fn model_id(&self) -> &str {
        "mock-model"
    }
}

/// A scripted text turn: one delta plus a STOP completion.
fn text_turn(text: &str) -> Arc<dyn ModelClient> {
    MockModelClient::single_turn(vec![
        ModelStreamEvent::TextDelta(text.to_string()),
        ModelStreamEvent::Completed {
            usage: None,
            finish_reason: Some("STOP".to_string()),
        },
    ])
}

fn empty_tools() -> Arc<ToolRegistry> {
    Arc::new(ToolRegistry::new())
}

fn session() -> Session {
    Session::new(EngineConfig::new("mock-model"))
}

/// Drain the fan-in stream until every id in `expected` has emitted a
/// `TurnComplete`, collecting each `(AgentId, msg-kind)` tag seen. Bounded by a
/// per-recv timeout so it can never hang.
async fn drain_until_complete(
    rx: &mut UnboundedReceiver<(AgentId, Event)>,
    mut remaining: usize,
) -> Vec<(AgentId, EventMsg)> {
    let mut seen = Vec::new();
    while remaining > 0 {
        let next = timeout(RECV_TIMEOUT, rx.recv())
            .await
            .expect("fan-in recv timed out");
        let Some((id, event)) = next else {
            break; // channel closed
        };
        let is_complete = matches!(event.msg, EventMsg::TurnComplete { .. });
        seen.push((id, event.msg));
        if is_complete {
            remaining -= 1;
        }
    }
    seen
}

#[tokio::test]
async fn fan_in_tags_events_from_both_agents() {
    let mut sup = Supervisor::new();
    let mut events = sup.take_events().expect("events receiver taken once");

    sup.spawn_agent("alpha", session(), text_turn("from alpha"), empty_tools(), None)
        .expect("spawn alpha");
    sup.spawn_agent("beta", session(), text_turn("from beta"), empty_tools(), None)
        .expect("spawn beta");

    sup.send_user_text("alpha", "hi alpha").expect("route alpha");
    sup.send_user_text("beta", "hi beta").expect("route beta");

    // Two agents -> wait for two TurnComplete events.
    let seen = drain_until_complete(&mut events, 2).await;

    let alpha = AgentId::from("alpha");
    let beta = AgentId::from("beta");

    // Both agents reached completion.
    let completes: Vec<&AgentId> = seen
        .iter()
        .filter(|(_, msg)| matches!(msg, EventMsg::TurnComplete { .. }))
        .map(|(id, _)| id)
        .collect();
    assert!(completes.contains(&&alpha), "alpha must complete: {seen:?}");
    assert!(completes.contains(&&beta), "beta must complete: {seen:?}");

    // Each agent's stream is correctly tagged: every event tagged `alpha`
    // belongs to alpha and likewise for beta (no cross-talk).
    let alpha_starts = seen
        .iter()
        .any(|(id, msg)| *id == alpha && matches!(msg, EventMsg::TurnStarted));
    let beta_starts = seen
        .iter()
        .any(|(id, msg)| *id == beta && matches!(msg, EventMsg::TurnStarted));
    assert!(alpha_starts, "alpha must start a turn: {seen:?}");
    assert!(beta_starts, "beta must start a turn: {seen:?}");

    // The agent-visible text routed to the right tag.
    let alpha_msg = seen.iter().any(|(id, msg)| {
        *id == alpha && matches!(msg, EventMsg::AgentMessage { text } if text == "from alpha")
    });
    let beta_msg = seen.iter().any(|(id, msg)| {
        *id == beta && matches!(msg, EventMsg::AgentMessage { text } if text == "from beta")
    });
    assert!(alpha_msg, "alpha message mis-tagged: {seen:?}");
    assert!(beta_msg, "beta message mis-tagged: {seen:?}");

    sup.shutdown_all();
}

#[tokio::test]
async fn duplicate_id_is_rejected() {
    let mut sup = Supervisor::new();
    sup.spawn_agent("dup", session(), text_turn("a"), empty_tools(), None)
        .expect("first spawn");

    let err = sup
        .spawn_agent("dup", session(), text_turn("b"), empty_tools(), None)
        .expect_err("duplicate id must be rejected");

    match err {
        SupervisorError::DuplicateId(id) => assert_eq!(id, AgentId::from("dup")),
        other => panic!("expected DuplicateId, got {other:?}"),
    }
    // The original agent is untouched.
    assert_eq!(sup.count(), 1);

    sup.shutdown_all();
}

#[tokio::test]
async fn routing_to_unknown_agent_errors() {
    let sup = Supervisor::new();

    for err in [
        sup.send_user_text("ghost", "hi").unwrap_err(),
        sup.interrupt("ghost").unwrap_err(),
        sup.steer_text("ghost", "be concise").unwrap_err(),
    ] {
        match err {
            SupervisorError::UnknownAgent(id) => assert_eq!(id, AgentId::from("ghost")),
            other => panic!("expected UnknownAgent, got {other:?}"),
        }
    }

    // Mutating lifecycle calls on an unknown id also error.
    let mut sup = sup;
    match sup.shutdown("ghost").unwrap_err() {
        SupervisorError::UnknownAgent(id) => assert_eq!(id, AgentId::from("ghost")),
        other => panic!("expected UnknownAgent from shutdown, got {other:?}"),
    }
    match sup.join("ghost").await.unwrap_err() {
        SupervisorError::UnknownAgent(id) => assert_eq!(id, AgentId::from("ghost")),
        other => panic!("expected UnknownAgent from join, got {other:?}"),
    }
}

#[tokio::test]
async fn steer_routes_to_live_agent() {
    let mut sup = Supervisor::new();
    sup.spawn_agent("worker", session(), text_turn("ok"), empty_tools(), None)
        .expect("spawn");

    // Routing a steer to a live agent succeeds via both the ergonomic text
    // wrapper and the explicit-items form. (The engine's mid-turn folding is
    // covered deterministically by the aphrody-engine tests; here we only assert
    // the control-plane routing.)
    sup.steer_text("worker", "be concise").expect("steer_text routes");
    sup.steer(
        "worker",
        vec![aphrody_agent_proto::InputItem::Text {
            text: "and fast".to_string(),
        }],
    )
    .expect("steer routes");

    sup.shutdown_all();
}

#[tokio::test]
async fn shutdown_removes_the_agent() {
    let mut sup = Supervisor::new();
    sup.spawn_agent("worker", session(), text_turn("hi"), empty_tools(), None)
        .expect("spawn");

    assert_eq!(sup.count(), 1);
    assert!(sup.contains("worker"));

    sup.shutdown("worker").expect("shutdown");

    assert_eq!(sup.count(), 0);
    assert!(!sup.contains("worker"));
    assert!(sup.is_empty());

    // A second shutdown of the now-removed agent is an UnknownAgent error.
    match sup.shutdown("worker").unwrap_err() {
        SupervisorError::UnknownAgent(_) => {}
        other => panic!("expected UnknownAgent, got {other:?}"),
    }
}

#[tokio::test]
async fn list_reflects_live_agents() {
    let mut sup = Supervisor::new();
    sup.spawn_agent("one", session(), text_turn("1"), empty_tools(), None)
        .expect("spawn one");
    sup.spawn_agent("two", session(), text_turn("2"), empty_tools(), None)
        .expect("spawn two");

    let listed = sup.list();
    assert_eq!(listed.len(), 2);
    // Sorted by id for determinism.
    assert_eq!(listed[0].id, AgentId::from("one"));
    assert_eq!(listed[1].id, AgentId::from("two"));
    // FullAuto is the default autonomy.
    assert!(listed.iter().all(|s| s.full_auto));
    assert!(listed.iter().all(|s| s.model == "mock-model"));

    sup.shutdown("one").expect("shutdown one");
    let listed = sup.list();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, AgentId::from("two"));

    sup.shutdown_all();
}

#[tokio::test]
async fn take_events_is_callable_once() {
    let mut sup = Supervisor::new();
    assert!(sup.take_events().is_some());
    assert!(sup.take_events().is_none());
}

#[tokio::test]
async fn join_recovers_session_after_shutdown() {
    let mut sup = Supervisor::new();
    // Drain events so the actor is not blocked on a full channel (unbounded
    // anyway, but this models a real consumer).
    let mut events = sup.take_events().expect("events");

    sup.spawn_agent("solo", session(), text_turn("done"), empty_tools(), None)
        .expect("spawn");
    sup.send_user_text("solo", "go").expect("route");

    // Let the turn complete, then ask the actor to wind down.
    let _ = drain_until_complete(&mut events, 1).await;
    sup.submit("solo", aphrody_agent_proto::Op::Shutdown)
        .expect("submit shutdown");

    let session = timeout(RECV_TIMEOUT, sup.join("solo"))
        .await
        .expect("join did not hang")
        .expect("join recovers session");
    // History holds the user turn plus the model's text reply.
    assert!(!session.is_empty(), "joined session retains history");
    assert!(!sup.contains("solo"));
}
