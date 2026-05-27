// SPDX-License-Identifier: Apache-2.0
//! Offline, network-free tests for the engine turn loop.
//!
//! No `reqwest::Client` is ever constructed here (the [`MockModelClient`] is a
//! scripted in-memory stream), so the rustls "No provider set" panic
//! (CLAUDE.md §7) cannot occur.

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use aphrody_agent_proto::{Event, EventMsg, InputItem, Op, ReviewDecision};
use aphrody_model_client::{
    ChatTurn, ModelClient, ModelError, ModelStream, ModelStreamEvent, Part, Role, TokenUsage,
};
use aphrody_rollout::RolloutRecorder;
use aphrody_toolcall::{
    JsonSchema, ToolDefinition, ToolError, ToolExecutor, ToolOutput, ToolRegistry,
};
use async_trait::async_trait;
use serde_json::{Value, json};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::{ApprovalGate, AutonomyMode, EngineConfig, EventSink, Session, run_turn};

/// A model client that replays pre-scripted turns, one per `stream` call.
///
/// Each entry is the full list of events that the corresponding `stream` call
/// emits. This lets a test exercise multi-turn re-injection: turn 0 may emit a
/// `ToolCall`, turn 1 then emits plain text + completion.
struct MockModelClient {
    turns: Mutex<std::collections::VecDeque<Vec<ModelStreamEvent>>>,
    /// When set, every `stream` call returns this scripted turn forever (used
    /// to test the iteration cap).
    looping: Option<Vec<ModelStreamEvent>>,
    calls: AtomicUsize,
}

impl MockModelClient {
    fn scripted(turns: Vec<Vec<ModelStreamEvent>>) -> Self {
        Self {
            turns: Mutex::new(turns.into_iter().collect()),
            looping: None,
            calls: AtomicUsize::new(0),
        }
    }

    fn looping(turn: Vec<ModelStreamEvent>) -> Self {
        Self {
            turns: Mutex::new(std::collections::VecDeque::new()),
            looping: Some(turn),
            calls: AtomicUsize::new(0),
        }
    }

    fn call_count(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl ModelClient for MockModelClient {
    async fn stream(&self, _turn: ChatTurn) -> Result<ModelStream, ModelError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let events = if let Some(looping) = &self.looping {
            looping.clone()
        } else {
            self.turns
                .lock()
                .expect("turn queue lock")
                .pop_front()
                .unwrap_or_else(|| {
                    vec![ModelStreamEvent::Completed {
                        usage: None,
                        finish_reason: Some("STOP".to_string()),
                    }]
                })
        };
        let stream = futures::stream::iter(events.into_iter().map(Ok));
        Ok(Box::pin(stream))
    }

    fn model_id(&self) -> &str {
        "mock-model"
    }
}

/// A tool that echoes its arguments and counts invocations.
struct MockTool {
    definition: ToolDefinition,
    invocations: AtomicUsize,
}

impl MockTool {
    fn new(name: &str) -> Self {
        Self {
            definition: ToolDefinition::new(
                name,
                "echoes the provided arguments",
                JsonSchema::object(Default::default(), None, None),
            ),
            invocations: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl ToolExecutor for MockTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn handle(&self, arguments: Value) -> Result<ToolOutput, ToolError> {
        self.invocations.fetch_add(1, Ordering::SeqCst);
        Ok(ToolOutput::ok(format!("echo: {arguments}")))
    }
}

fn registry_with(name: &str) -> (ToolRegistry, Arc<MockTool>) {
    let tool = Arc::new(MockTool::new(name));
    let mut registry = ToolRegistry::new();
    registry
        .register(tool.clone() as Arc<dyn ToolExecutor>)
        .expect("register tool");
    (registry, tool)
}

fn user_text(text: &str) -> Vec<InputItem> {
    vec![InputItem::Text { text: text.into() }]
}

fn drain(rx: &mut UnboundedReceiver<Event>) -> Vec<EventMsg> {
    let mut out = Vec::new();
    while let Ok(event) = rx.try_recv() {
        out.push(event.msg);
    }
    out
}

fn kinds(msgs: &[EventMsg]) -> Vec<&'static str> {
    msgs.iter().map(kind).collect()
}

fn kind(msg: &EventMsg) -> &'static str {
    match msg {
        EventMsg::TurnStarted => "TurnStarted",
        EventMsg::AgentMessageDelta { .. } => "AgentMessageDelta",
        EventMsg::AgentMessage { .. } => "AgentMessage",
        EventMsg::AgentReasoningDelta { .. } => "AgentReasoningDelta",
        EventMsg::ExecCommandBegin { .. } => "ExecCommandBegin",
        EventMsg::ExecCommandOutputDelta { .. } => "ExecCommandOutputDelta",
        EventMsg::ExecCommandEnd { .. } => "ExecCommandEnd",
        EventMsg::ExecApprovalRequest { .. } => "ExecApprovalRequest",
        EventMsg::ApplyPatchApprovalRequest { .. } => "ApplyPatchApprovalRequest",
        EventMsg::ToolCallBegin { .. } => "ToolCallBegin",
        EventMsg::ToolCallEnd { .. } => "ToolCallEnd",
        EventMsg::TokenCount { .. } => "TokenCount",
        EventMsg::TurnComplete { .. } => "TurnComplete",
        EventMsg::Error { .. } => "Error",
    }
}

#[tokio::test]
async fn text_only_turn_emits_message_and_complete() {
    let client = MockModelClient::scripted(vec![vec![
        ModelStreamEvent::TextDelta("Hello ".to_string()),
        ModelStreamEvent::TextDelta("world".to_string()),
        ModelStreamEvent::Completed {
            usage: Some(TokenUsage {
                input_tokens: 5,
                output_tokens: 2,
                total_tokens: 7,
            }),
            finish_reason: Some("STOP".to_string()),
        },
    ]]);
    let tools = ToolRegistry::new();
    let mut session = Session::new(EngineConfig::new("mock-model"));
    let (sink, mut rx) = EventSink::unbounded();

    let outcome = run_turn(
        &mut session,
        &client,
        &tools,
        None,
        user_text("hi"),
        &sink,
        None,
    )
    .await
    .expect("turn");

    assert_eq!(outcome.last_agent_message.as_deref(), Some("Hello world"));
    assert_eq!(outcome.tool_iterations, 1);
    assert!(!outcome.interrupted);
    assert!(!outcome.hit_iteration_cap);

    let msgs = drain(&mut rx);
    let k = kinds(&msgs);
    assert_eq!(k.first(), Some(&"TurnStarted"));
    assert!(k.contains(&"AgentMessageDelta"));
    assert!(k.contains(&"AgentMessage"));
    assert!(k.contains(&"TokenCount"));
    assert_eq!(k.last(), Some(&"TurnComplete"));

    // History: user input + the finalized model text.
    assert_eq!(session.history().len(), 2);
    assert_eq!(session.history()[0].role, Role::User);
    assert_eq!(session.history()[1].role, Role::Model);
}

#[tokio::test]
async fn tool_call_turn_reinjects_and_completes() {
    let (tools, tool) = registry_with("echo");
    let client = MockModelClient::scripted(vec![
        // Turn 0: model asks to call the tool.
        vec![
            ModelStreamEvent::ToolCall {
                id: "call-1".to_string(),
                name: "echo".to_string(),
                arguments: json!({ "x": 1 }),
            },
            ModelStreamEvent::Completed {
                usage: None,
                finish_reason: Some("TOOL".to_string()),
            },
        ],
        // Turn 1: with the tool result re-injected, model answers in text.
        vec![
            ModelStreamEvent::TextDelta("done".to_string()),
            ModelStreamEvent::Completed {
                usage: None,
                finish_reason: Some("STOP".to_string()),
            },
        ],
    ]);
    let mut session = Session::new(EngineConfig::new("mock-model"));
    let (sink, mut rx) = EventSink::unbounded();

    let outcome = run_turn(
        &mut session,
        &client,
        &tools,
        None,
        user_text("call echo"),
        &sink,
        None,
    )
    .await
    .expect("turn");

    assert_eq!(outcome.last_agent_message.as_deref(), Some("done"));
    assert_eq!(outcome.tool_iterations, 2, "two model calls (re-injection)");
    assert_eq!(client.call_count(), 2);
    assert_eq!(tool.invocations.load(Ordering::SeqCst), 1);

    let k = kinds(&drain(&mut rx));
    assert!(k.contains(&"ToolCallBegin"));
    assert!(k.contains(&"ToolCallEnd"));
    assert_eq!(k.last(), Some(&"TurnComplete"));

    // History: user, model FunctionCall, tool FunctionResponse, model text.
    let history = session.history();
    assert_eq!(history.len(), 4);
    assert!(matches!(
        history[1].parts.first(),
        Some(Part::FunctionCall { name, .. }) if name == "echo"
    ));
    assert!(matches!(
        history[2].parts.first(),
        Some(Part::FunctionResponse { name, .. }) if name == "echo"
    ));
    assert_eq!(history[2].role, Role::Tool);
}

#[tokio::test]
async fn unknown_tool_feeds_error_response() {
    // Registry is empty: the requested tool does not exist.
    let tools = ToolRegistry::new();
    let client = MockModelClient::scripted(vec![
        vec![
            ModelStreamEvent::ToolCall {
                id: "call-x".to_string(),
                name: "missing".to_string(),
                arguments: json!({}),
            },
            ModelStreamEvent::Completed {
                usage: None,
                finish_reason: None,
            },
        ],
        vec![
            ModelStreamEvent::TextDelta("recovered".to_string()),
            ModelStreamEvent::Completed {
                usage: None,
                finish_reason: Some("STOP".to_string()),
            },
        ],
    ]);
    let mut session = Session::new(EngineConfig::new("mock-model"));
    let (sink, mut rx) = EventSink::unbounded();

    let outcome = run_turn(
        &mut session,
        &client,
        &tools,
        None,
        user_text("call missing"),
        &sink,
        None,
    )
    .await
    .expect("turn");

    assert_eq!(outcome.last_agent_message.as_deref(), Some("recovered"));

    // The tool-call end reports failure, and the function response carries the
    // error flag.
    let msgs = drain(&mut rx);
    assert!(msgs.iter().any(|m| matches!(
        m,
        EventMsg::ToolCallEnd { ok: false, .. }
    )));
    let response = match &session.history()[2].parts[0] {
        Part::FunctionResponse { response, .. } => response.clone(),
        other => panic!("expected FunctionResponse, got {other:?}"),
    };
    assert_eq!(response["is_error"], json!(true));
    assert!(
        response["output"]
            .as_str()
            .unwrap_or_default()
            .contains("unknown tool")
    );
}

#[tokio::test]
async fn gated_mode_approved_executes_tool() {
    let (tools, tool) = registry_with("echo");
    let client = MockModelClient::scripted(vec![
        vec![
            ModelStreamEvent::ToolCall {
                id: "g-1".to_string(),
                name: "echo".to_string(),
                arguments: json!({}),
            },
            ModelStreamEvent::Completed {
                usage: None,
                finish_reason: None,
            },
        ],
        vec![
            ModelStreamEvent::TextDelta("ok".to_string()),
            ModelStreamEvent::Completed {
                usage: None,
                finish_reason: Some("STOP".to_string()),
            },
        ],
    ]);
    let mut session = Session::new(EngineConfig::new("mock-model").with_autonomy(AutonomyMode::Gated));
    let (sink, mut rx) = EventSink::unbounded();
    let (approval_tx, mut gate) = ApprovalGate::channel();

    // Pre-load the approval so the gate finds it when it waits on "g-1".
    approval_tx
        .send(Op::ExecApproval {
            id: "g-1".to_string(),
            decision: ReviewDecision::Approved,
        })
        .expect("send approval");

    let outcome = run_turn(
        &mut session,
        &client,
        &tools,
        None,
        user_text("call echo"),
        &sink,
        Some(&mut gate),
    )
    .await
    .expect("turn");

    assert!(!outcome.interrupted);
    assert_eq!(outcome.last_agent_message.as_deref(), Some("ok"));
    assert_eq!(tool.invocations.load(Ordering::SeqCst), 1);

    let k = kinds(&drain(&mut rx));
    assert!(k.contains(&"ExecApprovalRequest"));
    assert!(k.contains(&"ToolCallEnd"));
}

#[tokio::test]
async fn gated_mode_abort_stops_turn() {
    let (tools, tool) = registry_with("echo");
    let client = MockModelClient::scripted(vec![vec![
        ModelStreamEvent::ToolCall {
            id: "g-2".to_string(),
            name: "echo".to_string(),
            arguments: json!({}),
        },
        ModelStreamEvent::Completed {
            usage: None,
            finish_reason: None,
        },
    ]]);
    let mut session = Session::new(EngineConfig::new("mock-model").with_autonomy(AutonomyMode::Gated));
    let (sink, mut rx) = EventSink::unbounded();
    let (approval_tx, mut gate) = ApprovalGate::channel();

    approval_tx
        .send(Op::ExecApproval {
            id: "g-2".to_string(),
            decision: ReviewDecision::Abort,
        })
        .expect("send abort");

    let outcome = run_turn(
        &mut session,
        &client,
        &tools,
        None,
        user_text("call echo"),
        &sink,
        Some(&mut gate),
    )
    .await
    .expect("turn");

    assert!(outcome.interrupted, "abort must mark the turn interrupted");
    assert_eq!(
        tool.invocations.load(Ordering::SeqCst),
        0,
        "aborted tool must not run"
    );

    let k = kinds(&drain(&mut rx));
    assert!(k.contains(&"ExecApprovalRequest"));
    // The tool never executed, so no ToolCallEnd was emitted.
    assert!(!k.contains(&"ToolCallEnd"));
    assert_eq!(k.last(), Some(&"TurnComplete"));
}

#[tokio::test]
async fn gated_mode_denied_skips_and_continues() {
    let (tools, tool) = registry_with("echo");
    let client = MockModelClient::scripted(vec![
        vec![
            ModelStreamEvent::ToolCall {
                id: "g-3".to_string(),
                name: "echo".to_string(),
                arguments: json!({}),
            },
            ModelStreamEvent::Completed {
                usage: None,
                finish_reason: None,
            },
        ],
        vec![
            ModelStreamEvent::TextDelta("after deny".to_string()),
            ModelStreamEvent::Completed {
                usage: None,
                finish_reason: Some("STOP".to_string()),
            },
        ],
    ]);
    let mut session = Session::new(EngineConfig::new("mock-model").with_autonomy(AutonomyMode::Gated));
    let (sink, _rx) = EventSink::unbounded();
    let (approval_tx, mut gate) = ApprovalGate::channel();

    approval_tx
        .send(Op::ExecApproval {
            id: "g-3".to_string(),
            decision: ReviewDecision::Denied,
        })
        .expect("send deny");

    let outcome = run_turn(
        &mut session,
        &client,
        &tools,
        None,
        user_text("call echo"),
        &sink,
        Some(&mut gate),
    )
    .await
    .expect("turn");

    assert!(!outcome.interrupted);
    assert_eq!(outcome.last_agent_message.as_deref(), Some("after deny"));
    assert_eq!(
        tool.invocations.load(Ordering::SeqCst),
        0,
        "denied tool must not run"
    );

    // The denied call still produced a FunctionResponse marked as an error.
    let response = match &session.history()[2].parts[0] {
        Part::FunctionResponse { response, .. } => response.clone(),
        other => panic!("expected FunctionResponse, got {other:?}"),
    };
    assert_eq!(response["is_error"], json!(true));
    assert!(
        response["output"]
            .as_str()
            .unwrap_or_default()
            .contains("denied")
    );
}

#[tokio::test]
async fn max_tool_iterations_is_respected() {
    let (tools, _tool) = registry_with("echo");
    // The model loops forever asking for the same tool.
    let client = MockModelClient::looping(vec![
        ModelStreamEvent::ToolCall {
            id: "loop".to_string(),
            name: "echo".to_string(),
            arguments: json!({}),
        },
        ModelStreamEvent::Completed {
            usage: None,
            finish_reason: None,
        },
    ]);
    let mut session =
        Session::new(EngineConfig::new("mock-model").with_max_tool_iterations(3));
    let (sink, mut rx) = EventSink::unbounded();

    let outcome = run_turn(
        &mut session,
        &client,
        &tools,
        None,
        user_text("loop"),
        &sink,
        None,
    )
    .await
    .expect("turn");

    assert!(outcome.hit_iteration_cap);
    assert_eq!(outcome.tool_iterations, 3);
    assert_eq!(client.call_count(), 3);

    let msgs = drain(&mut rx);
    assert!(msgs.iter().any(|m| matches!(
        m,
        EventMsg::Error { message } if message.contains("max tool iterations")
    )));
    assert!(matches!(msgs.last(), Some(EventMsg::TurnComplete { .. })));
}

#[tokio::test]
async fn model_stream_error_aborts_turn() {
    struct ErrClient;
    #[async_trait]
    impl ModelClient for ErrClient {
        async fn stream(&self, _turn: ChatTurn) -> Result<ModelStream, ModelError> {
            let stream = futures::stream::iter(vec![
                Ok(ModelStreamEvent::TextDelta("partial".to_string())),
                Err(ModelError::Stream("boom".to_string())),
            ]);
            Ok(Box::pin(stream))
        }
        fn model_id(&self) -> &str {
            "err"
        }
    }

    let tools = ToolRegistry::new();
    let mut session = Session::new(EngineConfig::new("err"));
    let (sink, mut rx) = EventSink::unbounded();

    let err = run_turn(
        &mut session,
        &ErrClient,
        &tools,
        None,
        user_text("hi"),
        &sink,
        None,
    )
    .await
    .expect_err("must surface model error");
    assert!(matches!(err, crate::EngineError::Model(_)));

    let k = kinds(&drain(&mut rx));
    assert!(k.contains(&"Error"));
}

#[tokio::test]
async fn rollout_captures_events() {
    let dir = tempfile::tempdir().expect("tempdir");
    let rollout = RolloutRecorder::new(dir.path(), Some("engine-test".to_string()))
        .await
        .expect("recorder");

    let client = MockModelClient::scripted(vec![vec![
        ModelStreamEvent::TextDelta("logged".to_string()),
        ModelStreamEvent::Completed {
            usage: None,
            finish_reason: Some("STOP".to_string()),
        },
    ]]);
    let tools = ToolRegistry::new();
    let mut session = Session::new(EngineConfig::new("mock-model"));
    let (sink, _rx) = EventSink::unbounded();

    run_turn(
        &mut session,
        &client,
        &tools,
        Some(&rollout),
        user_text("hi"),
        &sink,
        None,
    )
    .await
    .expect("turn");

    rollout.flush().await.expect("flush");
    let path = rollout.path().to_path_buf();
    rollout.shutdown().await.expect("shutdown");

    let raw = tokio::fs::read_to_string(&path).await.expect("read jsonl");
    let lines: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
    assert!(!lines.is_empty(), "rollout must contain lines");
    // The recorded items are the serialized engine events.
    assert!(raw.contains("turn_started"));
    assert!(raw.contains("agent_message"));
    assert!(raw.contains("turn_complete"));
}

#[tokio::test]
async fn actor_drives_user_input_to_completion() {
    use crate::spawn_session;

    let client: Arc<dyn ModelClient> = Arc::new(MockModelClient::scripted(vec![vec![
        ModelStreamEvent::TextDelta("hi from actor".to_string()),
        ModelStreamEvent::Completed {
            usage: None,
            finish_reason: Some("STOP".to_string()),
        },
    ]]));
    let tools = Arc::new(ToolRegistry::new());
    let session = Session::new(EngineConfig::new("mock-model"));

    let mut handle = spawn_session(session, client, tools, None);
    let mut events = handle.events().expect("events receiver");

    handle
        .submit(Op::UserInput {
            items: user_text("hello actor"),
        })
        .expect("submit");
    handle.submit(Op::Shutdown).expect("submit shutdown");

    let final_session = handle.join().await.expect("join");
    assert_eq!(final_session.history()[0].role, Role::User);

    // Collect emitted events until the channel closes.
    let mut msgs = Vec::new();
    while let Some(event) = events.recv().await {
        msgs.push(event.msg);
    }
    let k = kinds(&msgs);
    assert_eq!(k.first(), Some(&"TurnStarted"));
    assert!(k.contains(&"AgentMessage"));
    assert!(k.contains(&"TurnComplete"));
}
