// SPDX-License-Identifier: Apache-2.0
//! Integration tests for `aphrody-terminal-json-out`.

use aphrody_terminal_json_out::{Envelope, Stream, frame_binary, frame_line, spawn_framed_child};
use regex::Regex;
use serde_json::Value;
use tokio::sync::mpsc;

// ── Synchronous framing primitives ────────────────────────────────────────────

#[test]
fn frame_stdout_plain_text() {
    let env = frame_line(Stream::Stdout, "hello");
    let json = serde_json::to_value(&env).unwrap();
    assert_eq!(json["stream"], "stdout");
    assert_eq!(json["line"], "hello");
    assert!(json["ts"].is_string(), "ts must serialize as a JSON string, got {:?}", json["ts"]);
}

#[test]
fn passthrough_app_json() {
    let env = frame_line(Stream::Stdout, "{\"event\":\"build\",\"ok\":true}");
    let line = &env.line;
    assert!(line.is_object(), "expected passthrough object, got {line:?}");
    assert_eq!(line["event"], "build");
    assert_eq!(line["ok"], true);
}

#[test]
fn jsonl_format_terminated_with_newline() {
    let env = frame_line(Stream::Stdout, "hello");
    let s = env.to_jsonl_string().unwrap();
    assert!(s.ends_with('\n'), "expected trailing newline, got {s:?}");
    // Exactly one newline (the terminator).
    assert_eq!(s.matches('\n').count(), 1);
    // The body up to the newline must parse back into a valid envelope.
    let parsed: Envelope = serde_json::from_str(s.trim_end_matches('\n')).unwrap();
    assert_eq!(parsed.stream, Stream::Stdout);
}

#[test]
fn binary_data_base64() {
    let env = frame_binary(Stream::Stdout, &[0xFF, 0x00, 0xAB]);
    let line = &env.line;
    assert_eq!(line["b64"], "/wCr");
}

#[test]
fn frame_stderr_distinct_stream() {
    let env = frame_line(Stream::Stderr, "oops");
    let json = serde_json::to_value(&env).unwrap();
    assert_eq!(json["stream"], "stderr");
    assert_eq!(json["line"], "oops");
}

#[test]
fn ts_format_iso_8601_utc() {
    let env = frame_line(Stream::Stdout, "x");
    let re = Regex::new(r"^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z$").unwrap();
    assert!(re.is_match(&env.ts), "ts {:?} does not match ISO-8601 UTC pattern", env.ts);
}

#[test]
fn passthrough_app_json_array() {
    let env = frame_line(Stream::Stdout, "[1,2,3]");
    assert!(env.line.is_array(), "expected JSON array passthrough");
    assert_eq!(env.line[1], 2);
}

#[test]
fn raw_json_string_literal_is_text() {
    // A bare quoted string ("foo") is technically valid JSON but treating it
    // as passthrough would silently swallow the quotes. Keep it as plain text.
    let env = frame_line(Stream::Stdout, "\"quoted\"");
    assert_eq!(env.line, Value::String("\"quoted\"".to_string()));
}

// ── Async child-process integration ───────────────────────────────────────────

/// Build a shell command that prints `out` to stdout and `err` to stderr,
/// using whatever shell is reliably present on the host platform.
fn echo_both_streams_cmd() -> tokio::process::Command {
    #[cfg(windows)]
    {
        let mut c = tokio::process::Command::new("cmd");
        c.args(["/C", "echo out & echo err 1>&2"]);
        c
    }
    #[cfg(not(windows))]
    {
        let mut c = tokio::process::Command::new("sh");
        c.args(["-c", "echo out; echo err 1>&2"]);
        c
    }
}

fn exit_with_code_cmd(code: i32) -> tokio::process::Command {
    #[cfg(windows)]
    {
        let mut c = tokio::process::Command::new("cmd");
        c.args(["/C", &format!("exit {code}")]);
        c
    }
    #[cfg(not(windows))]
    {
        let mut c = tokio::process::Command::new("sh");
        c.args(["-c", &format!("exit {code}")]);
        c
    }
}

#[tokio::test]
async fn spawn_framed_child_captures_both_streams() {
    let (tx, mut rx) = mpsc::channel::<String>(16);
    let mut cmd = echo_both_streams_cmd();
    let code = spawn_framed_child(&mut cmd, tx).await.expect("spawn ok");
    assert_eq!(code, 0, "echo command should exit 0, got {code}");

    let mut envelopes = Vec::new();
    while let Some(s) = rx.recv().await {
        let env: Envelope = serde_json::from_str(s.trim_end_matches('\n')).expect("valid jsonl");
        envelopes.push(env);
    }
    assert!(
        envelopes.len() >= 2,
        "expected at least one stdout + one stderr envelope, got {envelopes:#?}"
    );

    let has_stdout = envelopes.iter().any(|e| e.stream == Stream::Stdout);
    let has_stderr = envelopes.iter().any(|e| e.stream == Stream::Stderr);
    assert!(has_stdout, "missing stdout envelope: {envelopes:#?}");
    assert!(has_stderr, "missing stderr envelope: {envelopes:#?}");

    let stdout_line = envelopes
        .iter()
        .find(|e| e.stream == Stream::Stdout)
        .map(|e| e.line.as_str().unwrap_or_default().trim().to_string())
        .unwrap_or_default();
    assert!(stdout_line.starts_with("out"), "stdout line {stdout_line:?} did not start with 'out'");
}

#[tokio::test]
async fn spawn_framed_child_returns_exit_code() {
    let (tx, mut rx) = mpsc::channel::<String>(4);
    let mut cmd = exit_with_code_cmd(42);
    let code = spawn_framed_child(&mut cmd, tx).await.expect("spawn ok");
    assert_eq!(code, 42, "expected explicit exit code 42, got {code}");
    // Drain the receiver so the task ends cleanly.
    while rx.recv().await.is_some() {}
}
