// SPDX-License-Identifier: Apache-2.0
//! Tests for [`ShellExecTool`]. Cross-platform: argv is shaped per-OS so the
//! tool itself stays shell-agnostic (it execs argv[0] directly).

use super::*;
use aphrody_toolcall::ToolExecutor;
use serde_json::json;
use std::sync::Arc;
use std::sync::Mutex;

/// Build an argv that runs `script` through the platform's shell as a single
/// child process (the tool never invokes a shell itself).
#[cfg(windows)]
fn shell_argv(script: &str) -> Vec<String> {
    vec!["cmd".to_string(), "/c".to_string(), script.to_string()]
}

#[cfg(not(windows))]
fn shell_argv(script: &str) -> Vec<String> {
    vec!["sh".to_string(), "-c".to_string(), script.to_string()]
}

#[tokio::test]
async fn definition_advertises_shell() {
    let tool = ShellExecTool::new();
    let def = tool.definition();
    assert_eq!(def.name, "shell");
    let props = def.input_schema.properties.as_ref().expect("properties");
    assert!(props.contains_key("command"));
    assert!(props.contains_key("cwd"));
    assert!(props.contains_key("timeout_ms"));
}

#[tokio::test]
async fn echo_captures_exit_zero_and_output() {
    let tool = ShellExecTool::new();
    let out = tool
        .handle(json!({ "command": shell_argv("echo aphrody-marker") }))
        .await
        .expect("handle echo");
    assert!(!out.is_error, "output: {}", out.content);
    assert!(out.content.contains("exit_code: 0"), "output: {}", out.content);
    assert!(out.content.contains("aphrody-marker"), "output: {}", out.content);
}

#[tokio::test]
async fn non_zero_exit_is_reported_not_errored() {
    let tool = ShellExecTool::new();
    let out = tool
        .handle(json!({ "command": shell_argv("exit 3") }))
        .await
        .expect("handle exit 3");
    // A non-zero exit is a real result, not a tool error.
    assert!(!out.is_error, "output: {}", out.content);
    assert!(out.content.contains("exit_code: 3"), "output: {}", out.content);
}

#[tokio::test]
async fn timeout_yields_error_output() {
    // A short timeout against a command that sleeps longer.
    let tool = ShellExecTool::new();
    let out = tool
        .handle(json!({
            "command": shell_argv("sleep 5"),
            "timeout_ms": 200,
        }))
        .await
        .expect("handle timeout");
    assert!(out.is_error, "output: {}", out.content);
    assert!(out.content.contains("timed out"), "output: {}", out.content);
}

#[tokio::test]
async fn spawn_failure_is_tool_error() {
    let tool = ShellExecTool::new();
    let err = tool
        .handle(json!({ "command": ["this-binary-does-not-exist-aphrody"] }))
        .await
        .expect_err("nonexistent binary must fail to spawn");
    assert!(matches!(err, ToolError::Execution { .. }));
}

#[tokio::test]
async fn empty_command_is_error_output() {
    let tool = ShellExecTool::new();
    let out = tool
        .handle(json!({ "command": [] }))
        .await
        .expect("handle empty");
    assert!(out.is_error);
    assert!(out.content.contains("non-empty"), "output: {}", out.content);
}

#[tokio::test]
async fn output_cap_truncates() {
    // Emit ~4 KiB with a 256-byte cap.
    let config = ShellExecConfig {
        default_timeout: DEFAULT_TIMEOUT,
        max_output_bytes: 256,
    };
    let tool = ShellExecTool::with_config(config);
    // `for` loops differ across cmd/sh; emit a long single line instead.
    #[cfg(windows)]
    let script = "echo AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA & echo AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA & echo AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA & echo AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA & echo AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    #[cfg(not(windows))]
    let script = "yes A | head -c 4096";
    let out = tool
        .handle(json!({ "command": shell_argv(script) }))
        .await
        .expect("handle cap");
    assert!(!out.is_error, "output: {}", out.content);
    assert!(out.content.contains("truncated"), "output: {}", out.content);
}

#[tokio::test]
async fn streaming_sink_receives_chunks() {
    let collected: Arc<Mutex<String>> = Arc::new(Mutex::new(String::new()));
    let sink_target = Arc::clone(&collected);
    let sink: OutputSink = Arc::new(move |chunk: String| {
        sink_target.lock().expect("lock").push_str(&chunk);
    });
    let tool = ShellExecTool::new().with_sink(sink);
    let out = tool
        .handle(json!({ "command": shell_argv("echo streamed-bytes") }))
        .await
        .expect("handle stream");
    assert!(!out.is_error, "output: {}", out.content);
    let seen = collected.lock().expect("lock").clone();
    assert!(seen.contains("streamed-bytes"), "streamed: {seen}");
}
