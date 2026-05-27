// SPDX-License-Identifier: Apache-2.0
//! Offline, deterministic tests for the runtime factory.
//!
//! Every test drives a [`StubModelClient`] against the real
//! [`ShellExecTool`](aphrody_agent_tools::ShellExecTool) /
//! [`ApplyPatchTool`](aphrody_agent_tools::ApplyPatchTool). No test constructs a
//! `reqwest::Client` or a `GeminiClient` (CLAUDE.md §7: that panics with "no
//! provider set" under rustls 0.23 in a bare test process).

use aphrody_agent_proto::EventMsg;
use aphrody_engine::AutonomyMode;
use aphrody_model_client::ModelStreamEvent;
use pretty_assertions::assert_eq;
use serde_json::json;

use crate::{AgentRuntime, ModelChoice, RuntimeConfig, ScriptedTurn, StubModelClient};

/// The argv the `shell` tool should run to echo `marker` on this platform.
///
/// On Unix `echo` is a real binary in PATH; on Windows it is a `cmd` builtin, so
/// it must be invoked through `cmd /C` (the shell tool launches executables
/// directly, with no shell wrapper).
fn echo_command(marker: &str) -> serde_json::Value {
    #[cfg(windows)]
    {
        json!(["cmd", "/C", "echo", marker])
    }
    #[cfg(not(windows))]
    {
        json!(["echo", marker])
    }
}

#[test]
fn default_config_is_full_auto() {
    let config = RuntimeConfig::default();
    assert_eq!(config.autonomy, AutonomyMode::FullAuto);
    // And via the model-targeting constructor.
    assert_eq!(
        RuntimeConfig::new("gemini-2.5-flash").autonomy,
        AutonomyMode::FullAuto
    );
}

#[test]
fn builder_requires_a_model() {
    let err = AgentRuntime::builder()
        .config(RuntimeConfig::new("m"))
        .build()
        .expect_err("missing model must error");
    assert!(matches!(err, crate::RuntimeError::Config(_)));
}

#[test]
fn build_tool_registry_loads_shell_and_apply_patch() {
    let config = RuntimeConfig::new("m");
    let tools = crate::build_tool_registry(&config).expect("registry");
    assert_eq!(tools.len(), 2);
    assert!(tools.get("shell").is_some());
    assert!(tools.get("apply_patch").is_some());
}

#[tokio::test]
async fn run_once_drives_a_tool_then_a_final_message() {
    // Turn 1: the model asks to run a shell command.
    // Turn 2: after seeing the tool output, the model produces its answer.
    let marker = "RUNTIME_OK";
    let stub = StubModelClient::new(
        "stub-model",
        vec![
            ScriptedTurn::tool_call("call-1", "shell", json!({ "command": echo_command(marker) })),
            ScriptedTurn::text("all done"),
        ],
    );

    let runtime = AgentRuntime::builder()
        .config(RuntimeConfig::new("stub-model"))
        .model(ModelChoice::stub(stub))
        .build()
        .expect("build runtime");

    let result = runtime.run_once("please run echo").await.expect("run_once");

    // The model's final message is surfaced.
    assert_eq!(result.last_agent_message.as_deref(), Some("all done"));
    // Two model calls happened: the tool-call turn and the final-message turn.
    assert_eq!(result.outcome.tool_iterations, 2);
    assert!(!result.outcome.interrupted);
    assert!(!result.outcome.hit_iteration_cap);

    // The engine surfaced the tool dispatch lifecycle.
    let mut begin_ok = false;
    let mut end_ok = false;
    let mut final_message_ok = false;
    for event in &result.events {
        match &event.msg {
            EventMsg::ToolCallBegin { call_id, name } => {
                if call_id == "call-1" && name == "shell" {
                    begin_ok = true;
                }
            }
            EventMsg::ToolCallEnd { call_id, ok } => {
                if call_id == "call-1" && *ok {
                    end_ok = true;
                }
            }
            EventMsg::AgentMessage { text } => {
                if text == "all done" {
                    final_message_ok = true;
                }
            }
            _ => {}
        }
    }
    assert!(begin_ok, "expected a ToolCallBegin for the shell call");
    assert!(end_ok, "expected a successful ToolCallEnd for the shell call");
    assert!(final_message_ok, "expected the final AgentMessage");

    // A TurnComplete must close the turn.
    assert!(result.events.iter().any(|e| matches!(
        &e.msg,
        EventMsg::TurnComplete { .. }
    )));
}

#[tokio::test]
async fn run_once_reinjects_tool_output_into_history() {
    // The second scripted turn echoes nothing useful on its own; we assert the
    // tool actually ran and its output ended up in a ToolCallEnd(ok=true),
    // proving the engine re-injected the result and looped to a second model
    // call that produced the final message.
    let stub = StubModelClient::new(
        "stub-model",
        vec![
            ScriptedTurn::tool_call(
                "c-echo",
                "shell",
                json!({ "command": echo_command("HELLO_FROM_TOOL") }),
            ),
            ScriptedTurn::text("acknowledged"),
        ],
    );

    let runtime = AgentRuntime::builder()
        .config(RuntimeConfig::new("stub-model"))
        .model(ModelChoice::stub(stub))
        .build()
        .expect("build");

    let result = runtime.run_once("go").await.expect("run_once");
    assert_eq!(result.last_agent_message.as_deref(), Some("acknowledged"));
    assert_eq!(result.outcome.tool_iterations, 2);
}

#[tokio::test]
async fn run_once_apply_patch_creates_a_file_on_disk() {
    let dir = tempfile::tempdir().expect("tempdir");
    let target = dir.path().join("created.txt");
    let target_str = target.to_string_lossy().replace('\\', "/");

    // A minimal apply_patch document that adds one file with a single line.
    let patch = format!(
        "*** Begin Patch\n*** Add File: {target_str}\n+hello patched world\n*** End Patch\n"
    );

    let stub = StubModelClient::new(
        "stub-model",
        vec![
            ScriptedTurn::tool_call("patch-1", "apply_patch", json!({ "patch": patch })),
            ScriptedTurn::text("file written"),
        ],
    );

    let runtime = AgentRuntime::builder()
        .config(RuntimeConfig::new("stub-model"))
        .model(ModelChoice::stub(stub))
        .build()
        .expect("build");

    let result = runtime.run_once("write the file").await.expect("run_once");
    assert_eq!(result.last_agent_message.as_deref(), Some("file written"));

    // The patch tool ran successfully and the file exists with our content.
    assert!(
        result.events.iter().any(|e| matches!(
            &e.msg,
            EventMsg::ToolCallEnd { call_id, ok } if call_id == "patch-1" && *ok
        )),
        "expected a successful apply_patch ToolCallEnd"
    );

    let contents = std::fs::read_to_string(&target).expect("created file must exist");
    assert!(
        contents.contains("hello patched world"),
        "unexpected file contents: {contents:?}"
    );
}

#[tokio::test]
async fn run_once_text_only_turn_finishes_in_one_iteration() {
    let stub = StubModelClient::new(
        "stub-model",
        vec![ScriptedTurn {
            events: vec![
                ModelStreamEvent::ReasoningDelta("thinking...".to_string()),
                ModelStreamEvent::TextDelta("just text".to_string()),
            ],
        }],
    );

    let runtime = AgentRuntime::builder()
        .config(RuntimeConfig::new("stub-model"))
        .model(ModelChoice::stub(stub))
        .build()
        .expect("build");

    let result = runtime.run_once("say something").await.expect("run_once");
    assert_eq!(result.last_agent_message.as_deref(), Some("just text"));
    assert_eq!(result.outcome.tool_iterations, 1);
    assert!(result.events.iter().any(|e| matches!(
        &e.msg,
        EventMsg::AgentReasoningDelta { delta } if delta == "thinking..."
    )));
}
