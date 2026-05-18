// SPDX-License-Identifier: Apache-2.0
//! Spawn a detected agent and stream its events as a uniform
//! [`AgentEvent`] sequence.
//!
//! Two transports converge into one event shape for the 3-CLI set:
//!
//!  - **Stdio** adapters (Claude Code, Gemini) emit JSONL lines; we parse
//!    each line as JSON, look for the common `text`/`content`/`delta`
//!    fields, and surface them as [`AgentEvent::ChatChunk`]. Tool calls
//!    land in [`AgentEvent::ToolCall`].
//!  - **ACP** (Antigravity) speaks JSON-RPC: we drive the
//!    `initialize → session/new → session/prompt` handshake from
//!    [`crate::protocol`] and translate `session/update` notifications into
//!    `ChatChunk`s.
//!
//! On Windows, prompts that exceed `max_prompt_arg_bytes` are written to a
//! temp file (via [`tempfile::NamedTempFile`]) and the path is exposed via
//! the `APHRODY_PROMPT_FILE` env var on the spawned child so adapters that
//! grow a `--prompt-file` flag downstream can opt in without changing the
//! Rust crate. Adapters that already stream the prompt via stdin (all three
//! today) bypass that code path entirely.

use std::{
    path::{Path, PathBuf},
    pin::Pin,
    process::Stdio,
};

use futures_core::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
};
use tokio_stream::wrappers::ReceiverStream;

use crate::{AgentDescriptor, AgentId, Protocol, agent_def, protocol};

/// Boxed `Stream<Item = AgentEvent>` so callers can pin it without naming
/// the concrete generator type. `Send + Unpin` keeps it usable from
/// `tokio::spawn` and `select!`.
pub type AgentEventStream = Pin<Box<dyn Stream<Item = AgentEvent> + Send>>;

/// Normalized event surfaced to callers, identical shape across both
/// transports. Modeled after open-design's `send('agent', ...)` payload
/// (`text_delta` / `tool_use` / `usage` / `done`) collapsed to the three
/// shapes a caller actually has to switch on.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    /// Streamed assistant text. May be a single full message or many small
    /// deltas — callers should concatenate.
    ChatChunk { text: String },
    /// A tool/function-call event from the agent. `args` is the raw JSON
    /// arguments object; the caller is responsible for interpretation.
    ToolCall { name: String, args: Value },
    /// Terminal status. `ok=true` means the agent exited 0 (or finished a
    /// successful ACP turn); `ok=false` carries the failure reason.
    Done { ok: bool, message: Option<String> },
    /// Surface-level error event for transport or framing failures that
    /// did not produce a clean `Done` (used by callers to log + recover).
    Error { message: String },
}

/// Options handed to [`spawn_agent`]. `cwd` defaults to the current
/// directory; `prompt` is the full composed user message.
#[derive(Debug, Clone)]
pub struct SpawnOptions {
    pub cwd: PathBuf,
    pub prompt: String,
    /// Extra env vars to merge into the child process (override on collide).
    pub env: Vec<(String, String)>,
}

impl SpawnOptions {
    pub fn new(cwd: impl Into<PathBuf>, prompt: impl Into<String>) -> Self {
        Self { cwd: cwd.into(), prompt: prompt.into(), env: Vec::new() }
    }
}

#[derive(Debug, Error)]
pub enum SpawnError {
    #[error("agent {0:?} not in registry; run discover() first")]
    AgentMissing(AgentId),
    #[error("prompt is {bytes} bytes which exceeds adapter argv budget {budget}")]
    PromptTooLarge { bytes: usize, budget: usize },
    #[error("failed to spawn child: {0}")]
    Spawn(#[from] std::io::Error),
}

/// Spawn `agent` with `opts.prompt` against `opts.cwd` and return a stream
/// of normalized events. The child process is owned by the returned
/// stream — dropping the stream signals SIGKILL.
pub async fn spawn_agent(
    agent: &AgentDescriptor,
    opts: SpawnOptions,
) -> Result<AgentEventStream, SpawnError> {
    let id = AgentId::from_slug(&agent.id).ok_or_else(|| {
        // Should never happen: AgentDescriptor.id is always a slug we
        // emitted. Map to AgentMissing for the caller's convenience.
        SpawnError::AgentMissing(AgentId::ClaudeCode)
    })?;
    let def = agent_def(id);
    let mut cmd = Command::new(&agent.binary_path);
    cmd.args(def.spawn_args);
    cmd.current_dir(&opts.cwd);
    for (k, v) in &opts.env {
        cmd.env(k, v);
    }
    cmd.stdin(if def.prompt_via_stdin { Stdio::piped() } else { Stdio::null() });
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    // Hold onto any tempfile for the lifetime of the child so the OS does
    // not unlink the prompt before the agent reads it. Only used when an
    // adapter is configured for argv-prompt carry and the prompt overflows
    // the Windows CreateProcess limit.
    let mut _prompt_file_guard: Option<tempfile::NamedTempFile> = None;

    if !def.prompt_via_stdin {
        let bytes = opts.prompt.as_bytes().len();
        if bytes > def.max_prompt_arg_bytes
            || would_exceed_windows_command_line(&agent.binary_path, def.spawn_args)
        {
            // ENAMETOOLONG fallback: spill the prompt to a temp file and
            // expose its path via APHRODY_PROMPT_FILE so future argv-prompt
            // adapters can `--prompt-file $APHRODY_PROMPT_FILE` instead of
            // carrying the bytes through CreateProcess.
            let tmp = write_prompt_tempfile(&opts.prompt).map_err(SpawnError::Spawn)?;
            cmd.env("APHRODY_PROMPT_FILE", tmp.path());
            _prompt_file_guard = Some(tmp);
        } else {
            cmd.arg(&opts.prompt);
        }
    }

    let mut child = cmd.spawn()?;

    // Stream wiring: a Tokio mpsc channel that the dispatcher writes into.
    // Buffer size 64 keeps producers from blocking on a fast event stream.
    let (tx, rx) = tokio::sync::mpsc::channel::<AgentEvent>(64);

    match def.protocol {
        Protocol::Stdio => {
            // Send the prompt down stdin, then close it so the agent
            // sees EOF and starts producing. Adapters expecting
            // streaming JSONL input (Claude Code `--input-format stream-json`)
            // can still drive the conversation forward — for the
            // one-shot port we only need a single prompt.
            if def.prompt_via_stdin {
                if let Some(mut stdin) = child.stdin.take() {
                    let payload = match id {
                        AgentId::ClaudeCode => render_claude_stream_json(&opts.prompt),
                        _ => opts.prompt.clone(),
                    };
                    let _ = stdin.write_all(payload.as_bytes()).await;
                    // Explicit shutdown so the child sees EOF promptly.
                    let _ = stdin.shutdown().await;
                }
            }
            spawn_stdio_pump(child, tx, _prompt_file_guard);
        }
        Protocol::Acp => {
            spawn_acp_pump(
                child,
                opts.cwd.clone(),
                opts.prompt.clone(),
                tx,
                _prompt_file_guard,
            );
        }
        Protocol::Sse => {
            // No agent in the 3-set ships an SSE bridge today. Surface
            // the unsupported protocol as a Done(false) event so the
            // caller can fall back instead of hanging.
            let _ = tx
                .send(AgentEvent::Done {
                    ok: false,
                    message: Some("SSE bridge transport is not implemented".into()),
                })
                .await;
            drop(child);
        }
    }

    Ok(Box::pin(ReceiverStream::new(rx)))
}

/// Spawn a tokio task that drains the child's stdout line-by-line, tries
/// to parse each line as JSON, and forwards normalized [`AgentEvent`]s.
fn spawn_stdio_pump(
    mut child: Child,
    tx: tokio::sync::mpsc::Sender<AgentEvent>,
    _prompt_file_guard: Option<tempfile::NamedTempFile>,
) {
    tokio::spawn(async move {
        // Keep the prompt-file alive for as long as the pump runs; dropped
        // when this task returns so the OS reclaims the temp inode.
        let _guard = _prompt_file_guard;
        let stdout = match child.stdout.take() {
            Some(s) => s,
            None => {
                let _ = tx
                    .send(AgentEvent::Done {
                        ok: false,
                        message: Some("child has no stdout pipe".into()),
                    })
                    .await;
                return;
            }
        };
        let mut reader = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = reader.next_line().await {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            // Try JSON first; if it parses, look for common text fields.
            // Otherwise treat the whole line as a chat chunk.
            match serde_json::from_str::<Value>(trimmed) {
                Ok(v) => {
                    if let Some(text) = extract_text_field(&v) {
                        if tx
                            .send(AgentEvent::ChatChunk { text: text.to_string() })
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    if let Some(call) = extract_tool_call(&v) {
                        if tx.send(call).await.is_err() {
                            break;
                        }
                    }
                }
                Err(_) => {
                    if tx
                        .send(AgentEvent::ChatChunk { text: line })
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
            }
        }
        let ok = child
            .wait()
            .await
            .map(|s| s.success())
            .unwrap_or(false);
        let _ = tx.send(AgentEvent::Done { ok, message: None }).await;
    });
}

/// Drive an ACP session: initialize, open a session, send a prompt, stream
/// chunks back. The full ACP state machine lives in
/// `open-design/apps/daemon/src/acp.ts` — this is a slimmer one-shot
/// version that handles the happy path and exits on the first error or
/// terminal response.
fn spawn_acp_pump(
    mut child: Child,
    cwd: PathBuf,
    prompt: String,
    tx: tokio::sync::mpsc::Sender<AgentEvent>,
    _prompt_file_guard: Option<tempfile::NamedTempFile>,
) {
    tokio::spawn(async move {
        let _guard = _prompt_file_guard;
        let mut stdin = match child.stdin.take() {
            Some(s) => s,
            None => {
                let _ = tx
                    .send(AgentEvent::Done {
                        ok: false,
                        message: Some("child has no stdin pipe".into()),
                    })
                    .await;
                return;
            }
        };
        let stdout = match child.stdout.take() {
            Some(s) => s,
            None => {
                let _ = tx
                    .send(AgentEvent::Done {
                        ok: false,
                        message: Some("child has no stdout pipe".into()),
                    })
                    .await;
                return;
            }
        };

        // Open the handshake: initialize then session/new. We don't wait
        // for the response of `initialize` before sending `session/new`
        // because every ACP impl we ship for processes them in order, and
        // the spec doesn't require strict ordering.
        let init = protocol::build_initialize(1, "aphrody-design-agents", env!("CARGO_PKG_VERSION"));
        let new_sess = protocol::build_session_new(2, &cwd.display().to_string());
        if stdin.write_all(init.to_line().as_bytes()).await.is_err()
            || stdin.write_all(new_sess.to_line().as_bytes()).await.is_err()
        {
            let _ = tx
                .send(AgentEvent::Done {
                    ok: false,
                    message: Some("acp handshake write failed".into()),
                })
                .await;
            return;
        }

        let mut reader = BufReader::new(stdout).lines();
        let mut session_id: Option<String> = None;
        let mut next_id: i64 = 3;
        let mut prompt_sent = false;

        while let Ok(Some(line)) = reader.next_line().await {
            let msg = match protocol::RpcMessage::parse(&line) {
                Ok(m) => m,
                Err(_) => continue,
            };
            // session/new response → capture id, then send prompt.
            if !prompt_sent && session_id.is_none() {
                if let Some(result) = msg.result.as_ref() {
                    if let Some(sid) = result.get("sessionId").and_then(|v| v.as_str()) {
                        session_id = Some(sid.to_string());
                        let prompt_msg = protocol::build_session_prompt(next_id, sid, &prompt);
                        next_id += 1;
                        if stdin
                            .write_all(prompt_msg.to_line().as_bytes())
                            .await
                            .is_err()
                        {
                            break;
                        }
                        prompt_sent = true;
                        continue;
                    }
                }
            }
            // session/update notifications carry the streamed deltas.
            if msg.method.as_deref() == Some("session/update") {
                if let Some(params) = msg.params.as_ref() {
                    if let Some(update) = params.get("update") {
                        if let Some(content) = update.get("content") {
                            if let Some(text) = content.get("text").and_then(|v| v.as_str()) {
                                if tx
                                    .send(AgentEvent::ChatChunk { text: text.to_string() })
                                    .await
                                    .is_err()
                                {
                                    break;
                                }
                            }
                        }
                    }
                }
                continue;
            }
            // session/prompt result → terminal success.
            if prompt_sent
                && msg.result.is_some()
                && msg.error.is_none()
                && msg.id.is_some()
            {
                let _ = tx.send(AgentEvent::Done { ok: true, message: None }).await;
                break;
            }
            // error envelope → terminal failure.
            if let Some(err) = msg.error.as_ref() {
                let m = err.get("message").and_then(|v| v.as_str()).unwrap_or("acp error");
                let _ = tx
                    .send(AgentEvent::Done {
                        ok: false,
                        message: Some(m.to_string()),
                    })
                    .await;
                break;
            }
        }
        // Always reap the child so we don't leak processes.
        let _ = child.start_kill();
    });
}

fn extract_text_field(v: &Value) -> Option<&str> {
    // Walk the common shapes:
    //  - Claude Code stream-json: { type: "content_block_delta", delta: { text } }
    //  - Gemini stream-json: { event: "text_delta", text }
    //  - Antigravity ACP session/update: { update: { content: { text } } }
    if let Some(t) = v.get("text").and_then(|x| x.as_str()) {
        return Some(t);
    }
    if let Some(t) = v
        .get("delta")
        .and_then(|d| d.get("text"))
        .and_then(|x| x.as_str())
    {
        return Some(t);
    }
    if let Some(t) = v
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.get(0))
        .and_then(|c0| c0.get("text"))
        .and_then(|x| x.as_str())
    {
        return Some(t);
    }
    None
}

fn extract_tool_call(v: &Value) -> Option<AgentEvent> {
    // Tool calls come back as `{ type: "tool_use", name, input }` in
    // Claude Code stream JSON; Gemini uses `{ event: "tool_call", tool:
    // { name, arguments } }`.
    if let Some(name) = v.get("name").and_then(|x| x.as_str()) {
        if v.get("input").is_some() || v.get("arguments").is_some() {
            let args = v
                .get("input")
                .cloned()
                .or_else(|| v.get("arguments").cloned())
                .unwrap_or(Value::Null);
            return Some(AgentEvent::ToolCall { name: name.to_string(), args });
        }
    }
    if let Some(tool) = v.get("tool") {
        if let Some(name) = tool.get("name").and_then(|x| x.as_str()) {
            let args = tool.get("arguments").cloned().unwrap_or(Value::Null);
            return Some(AgentEvent::ToolCall { name: name.to_string(), args });
        }
    }
    None
}

/// Render the Claude Code `--input-format stream-json` first turn. Claude
/// Code expects a JSONL envelope wrapping the user message; raw text on
/// stdin is rejected with `expected a JSON object`. open-design's
/// `claude-stream.ts` builds the exact same shape.
fn render_claude_stream_json(prompt: &str) -> String {
    let v = serde_json::json!({
        "type": "user",
        "message": {
            "role": "user",
            "content": [{ "type": "text", "text": prompt }],
        },
    });
    let mut s = serde_json::to_string(&v).expect("static shape serializes");
    s.push('\n');
    s
}

/// Write `prompt` to a freshly-created temp file and return the guard. The
/// caller must hold the guard for as long as the child process needs to
/// read the file — `NamedTempFile` unlinks the path on drop.
pub fn write_prompt_tempfile(prompt: &str) -> std::io::Result<tempfile::NamedTempFile> {
    let mut tmp = tempfile::Builder::new()
        .prefix("aphrody-prompt-")
        .suffix(".txt")
        .tempfile()?;
    use std::io::Write as _;
    tmp.write_all(prompt.as_bytes())?;
    tmp.flush()?;
    Ok(tmp)
}

/// Detect whether `path`'s string form would exceed the Windows
/// CreateProcess limit. Used by tests + by callers that want to refuse
/// huge prompts up-front instead of waiting for a spawn failure.
pub fn would_exceed_windows_command_line(path: &Path, args: &[&str]) -> bool {
    // CreateProcessW caps lpCommandLine at 32_768 wide chars; through a
    // .cmd shim the cmd.exe parser caps at 8_192. Use the tighter bound
    // so we're safe either way.
    const LIMIT: usize = 8_000;
    let mut total = path.as_os_str().len() + 2; // 2 for the quoting
    for a in args {
        total += a.len() + 3;
    }
    total > LIMIT
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_claude_stream_json_emits_user_envelope() {
        let s = render_claude_stream_json("hello");
        let v: Value = serde_json::from_str(s.trim()).unwrap();
        assert_eq!(v["type"], "user");
        assert_eq!(v["message"]["role"], "user");
        assert_eq!(v["message"]["content"][0]["type"], "text");
        assert_eq!(v["message"]["content"][0]["text"], "hello");
    }

    #[test]
    fn extract_text_field_handles_three_shapes() {
        // Direct text field (Gemini stream-json events).
        let v: Value = serde_json::from_str(r#"{"event":"text_delta","text":"hi"}"#).unwrap();
        assert_eq!(extract_text_field(&v), Some("hi"));

        // Delta wrapper (Claude Code content_block_delta).
        let v: Value =
            serde_json::from_str(r#"{"type":"content_block_delta","delta":{"text":"foo"}}"#).unwrap();
        assert_eq!(extract_text_field(&v), Some("foo"));

        // Message.content array (batched message shape).
        let v: Value = serde_json::from_str(
            r#"{"message":{"content":[{"type":"text","text":"bar"}]}}"#,
        )
        .unwrap();
        assert_eq!(extract_text_field(&v), Some("bar"));

        // No text field → None.
        let v: Value = serde_json::from_str(r#"{"event":"thinking_start"}"#).unwrap();
        assert_eq!(extract_text_field(&v), None);
    }

    #[test]
    fn extract_tool_call_handles_claude_and_gemini_shapes() {
        // Claude Code stream-json: top-level name + input.
        let v: Value =
            serde_json::from_str(r#"{"type":"tool_use","name":"Bash","input":{"cmd":"ls"}}"#)
                .unwrap();
        match extract_tool_call(&v) {
            Some(AgentEvent::ToolCall { name, args }) => {
                assert_eq!(name, "Bash");
                assert_eq!(args["cmd"], "ls");
            }
            other => panic!("expected tool call, got {other:?}"),
        }

        // Gemini: nested tool object with arguments.
        let v: Value = serde_json::from_str(
            r#"{"event":"tool_call","tool":{"name":"shell","arguments":{"script":"pwd"}}}"#,
        )
        .unwrap();
        match extract_tool_call(&v) {
            Some(AgentEvent::ToolCall { name, args }) => {
                assert_eq!(name, "shell");
                assert_eq!(args["script"], "pwd");
            }
            other => panic!("expected tool call, got {other:?}"),
        }

        // No tool call → None.
        let v: Value = serde_json::from_str(r#"{"event":"text_delta","text":"x"}"#).unwrap();
        assert!(extract_tool_call(&v).is_none());
    }

    #[test]
    fn would_exceed_windows_command_line_triggers_above_limit() {
        let bin = Path::new("C:/Users/yohan/.local/bin/claude.exe");
        let big = "x".repeat(9_000);
        let args = &[big.as_str()];
        assert!(
            would_exceed_windows_command_line(bin, args),
            "9KB argv should trip the 8KB Windows shim cap"
        );

        let small = "hello";
        let args = &[small];
        assert!(!would_exceed_windows_command_line(bin, args));
    }

    #[test]
    fn write_prompt_tempfile_round_trip() {
        // The ENAMETOOLONG fallback writes the prompt to disk so a future
        // `--prompt-file` argv adapter can read it. Verify the round-trip
        // so we don't silently truncate on flush.
        let body = "hello\nworld\nfin";
        let guard = write_prompt_tempfile(body).expect("tempfile created");
        let read_back = std::fs::read_to_string(guard.path()).expect("read back");
        assert_eq!(read_back, body);
    }

    #[test]
    fn agent_event_variants_serialize_with_tag() {
        // Round-trip every public AgentEvent shape through serde so the
        // CLI JSONL output stays stable (consumer scripts depend on the
        // `type` discriminant).
        let chunk = AgentEvent::ChatChunk { text: "hi".into() };
        let v = serde_json::to_value(&chunk).unwrap();
        assert_eq!(v["type"], "chat_chunk");

        let call = AgentEvent::ToolCall { name: "Bash".into(), args: serde_json::json!({"cmd":"ls"}) };
        let v = serde_json::to_value(&call).unwrap();
        assert_eq!(v["type"], "tool_call");
        assert_eq!(v["name"], "Bash");

        let done = AgentEvent::Done { ok: true, message: None };
        let v = serde_json::to_value(&done).unwrap();
        assert_eq!(v["type"], "done");
        assert_eq!(v["ok"], true);

        let err = AgentEvent::Error { message: "boom".into() };
        let v = serde_json::to_value(&err).unwrap();
        assert_eq!(v["type"], "error");
        assert_eq!(v["message"], "boom");
    }
}
