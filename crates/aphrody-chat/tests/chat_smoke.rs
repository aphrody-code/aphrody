// SPDX-License-Identifier: Apache-2.0
//! Integration smoke tests for the `aphrody-chat` orchestrator.
//!
//! These tests exercise the full composition root (memory ⨉ context ⨉ tools
//! ⨉ permissions ⨉ hooks ⨉ session ⨉ backend) end-to-end against the
//! deterministic [`StubBackend`]. They do not hit the network, do not spawn
//! subprocesses, and do not touch the filesystem unless a test explicitly
//! opts into it via `tempfile::tempdir`.

use std::sync::Arc;

use aphrody_chat::backend::{ModelBackend, StubBackend};
use aphrody_chat::{ChatConfig, ChatError, ChatLoop, Turn};
use aphrody_skills::permissions::{PermissionConfig, PermissionEngine};
use aphrody_tools::{ToolDescriptor, ToolRegistry};
use serde_json::json;

// ---------------------------------------------------------------------------
// 1. Default construction smoke.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn chat_loop_constructs_from_default_config_with_in_memory_session() {
    // Build with all defaults — must succeed (in-memory SqliteLocal, stub
    // backend, empty tool registry, default permission mode).
    let chat = ChatLoop::builder(ChatConfig::default())
        .with_backend(StubBackend::with_reply("OK"))
        .build();
    assert!(chat.is_ok(), "default ChatLoop must build cleanly: {chat:?}");
    let loop_ = chat.unwrap();
    assert_eq!(loop_.turns_executed(), 0);
    // The system prompt is seeded as a synthetic turn, so the session log is
    // non-empty even before any user input.
    assert!(loop_.session_event_count() >= 1);
    assert!(loop_.session().id.as_str().len() == 32);
}

// ---------------------------------------------------------------------------
// 2. Single-turn round-trip against the stub backend.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn single_turn_with_stub_backend_returns_assistant_message() {
    let stub_reply = "stub-reply-for-smoke-test";
    let mut chat = ChatLoop::builder(ChatConfig::default())
        .with_backend(StubBackend::with_reply(stub_reply))
        .build()
        .expect("default builder must succeed");

    let turn: Turn = chat
        .single_turn("hello aphrody")
        .await
        .expect("single_turn must succeed against the stub backend");

    assert_eq!(turn.assistant_response, stub_reply);
    assert_eq!(turn.user_input, "hello aphrody");
    assert!(turn.tool_invocations.is_empty(), "stub never emits tools");
    assert_eq!(chat.turns_executed(), 1);
    // Both user + assistant turns must have been appended.
    let recorded_turns = chat.recorded_turns();
    // 1 system (seeded) + 1 user + 1 assistant = 3.
    assert!(
        recorded_turns >= 3,
        "expected ≥3 recorded turns, got {recorded_turns}",
    );
}

// ---------------------------------------------------------------------------
// 3. Tool dispatch through permission gate.
// ---------------------------------------------------------------------------

/// Custom backend that always emits one tool call so the orchestrator must
/// exercise the full registry → validate → permission → dispatch → session
/// pipeline.
#[derive(Debug, Clone)]
struct ToolCallingBackend;

#[async_trait::async_trait]
impl ModelBackend for ToolCallingBackend {
    fn name(&self) -> &str {
        "tool-calling-backend"
    }
    async fn complete(
        &self,
        _messages: &[aphrody_chat::Message],
    ) -> Result<aphrody_chat::backend::BackendResponse, ChatError> {
        let invocation = aphrody_chat::ToolInvocation::new(
            "call-1",
            "echo",
            json!({"text": "ping"}),
        );
        Ok(aphrody_chat::backend::BackendResponse {
            content: "calling echo".into(),
            tool_calls: vec![invocation],
            prompt_tokens: 5,
            completion_tokens: 3,
            resolved_model: None,
        })
    }
}

#[tokio::test]
async fn turn_with_tool_call_passes_permission_gate_and_dispatches() {
    // Register a permissive `echo` tool — strict input schema with a single
    // required `text` field, allow-listed in the permission engine.
    let mut tools = ToolRegistry::new();
    tools
        .register(
            ToolDescriptor::new(
                "echo",
                "Echo the supplied text",
                json!({
                    "type": "object",
                    "properties": { "text": {"type": "string"} },
                    "required": ["text"]
                }),
            )
            .unwrap(),
        )
        .unwrap();

    // Allow-list `echo` in the permission engine so the gate returns Allow.
    let mut perm_cfg = PermissionConfig::default();
    perm_cfg.allow.push("echo".to_string());
    let perms = PermissionEngine::from_config(perm_cfg).unwrap();

    let mut chat = ChatLoop::builder(ChatConfig::default())
        .with_backend(ToolCallingBackend)
        .with_tools(tools)
        .with_permissions(perms)
        .build()
        .expect("builder must succeed");

    let turn = chat
        .single_turn("please echo ping")
        .await
        .expect("turn must succeed end-to-end");

    assert_eq!(turn.tool_invocations.len(), 1);
    let invocation = &turn.tool_invocations[0];
    assert_eq!(invocation.name, "echo");
    assert_eq!(invocation.arguments["text"], "ping");
    assert!(invocation.result.is_some(), "tool dispatch must record a result");
    assert!(invocation.error.is_none());
    assert_eq!(turn.prompt_tokens, 5);
    assert_eq!(turn.completion_tokens, 3);
}

// ---------------------------------------------------------------------------
// 4. Tool not registered → structured error (no panic).
// ---------------------------------------------------------------------------

#[tokio::test]
async fn unknown_tool_call_returns_tool_not_registered() {
    // Register nothing, so the tool-emitting backend's invocation must fail.
    let mut chat = ChatLoop::builder(ChatConfig::default())
        .with_backend(ToolCallingBackend)
        .build()
        .expect("builder must succeed");

    let err = chat
        .single_turn("dispatch a tool the registry does not know")
        .await
        .expect_err("must fail when the tool is absent from the registry");

    match err {
        ChatError::ToolNotRegistered(name) => assert_eq!(name, "echo"),
        other => panic!("expected ToolNotRegistered, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 5. Session persistence round-trip.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn session_persistence_writes_jsonl_to_disk() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let mut cfg = ChatConfig::default();
    cfg.session_dir = Some(tmp.path().to_path_buf());

    let mut chat = ChatLoop::builder(cfg)
        .with_backend(StubBackend::with_reply("persisted"))
        .build()
        .expect("builder must succeed");

    chat.single_turn("save me").await.expect("turn must succeed");

    // The orchestrator persists on every turn when session_dir is set.
    let session_id = chat.session().id.to_string();
    let expected = tmp.path().join(format!("{session_id}.jsonl"));
    assert!(
        expected.exists(),
        "expected session JSONL at {}, missing",
        expected.display()
    );
    let raw = tokio::fs::read_to_string(&expected).await.unwrap();
    assert!(raw.contains("session_start"));
    assert!(raw.contains("\"role\":\"user\""));
    assert!(raw.contains("\"role\":\"assistant\""));
}

// ---------------------------------------------------------------------------
// 6. max_turns enforcement.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn max_turns_zero_means_unbounded() {
    let mut cfg = ChatConfig::default();
    cfg.max_turns = 0;
    let mut chat = ChatLoop::builder(cfg)
        .with_backend(StubBackend::with_reply("ok"))
        .build()
        .unwrap();
    for _ in 0..5 {
        chat.single_turn("ping").await.unwrap();
    }
    assert_eq!(chat.turns_executed(), 5);
}

#[tokio::test]
async fn max_turns_enforced_when_set() {
    let mut cfg = ChatConfig::default();
    cfg.max_turns = 2;
    let mut chat = ChatLoop::builder(cfg)
        .with_backend(StubBackend::with_reply("ok"))
        .build()
        .unwrap();
    chat.single_turn("turn 1").await.unwrap();
    chat.single_turn("turn 2").await.unwrap();
    let err = chat.single_turn("turn 3 should fail").await.unwrap_err();
    match err {
        ChatError::InvalidConfig(msg) => assert!(msg.contains("max_turns")),
        other => panic!("expected InvalidConfig on max_turns overflow, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// 7. Memory provider can be swapped via the builder.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn shared_memory_provider_can_be_injected() {
    let memory = Arc::new(
        aphrody_memory::sqlite_local::SqliteLocalProvider::open_in_memory()
            .expect("sqlite open"),
    );
    let chat = ChatLoop::builder(ChatConfig::default())
        .with_backend(StubBackend::default())
        .with_shared_memory(memory.clone())
        .build();
    assert!(chat.is_ok(), "shared memory injection must succeed");
}
