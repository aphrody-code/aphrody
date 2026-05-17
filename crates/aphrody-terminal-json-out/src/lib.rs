// SPDX-License-Identifier: Apache-2.0
//! JSONL framing for child-process stdout/stderr.
//!
//! `aphrody-terminal-json-out` wraps every line a child process emits in a
//! one-line JSON envelope so downstream LLM agents, hooks, MCP servers, and
//! `aphrody-terminal` viewers can consume tool output as a stream of typed
//! events instead of unstructured text.
//!
//! # Envelope schema
//!
//! ```json
//! {"ts":"2026-05-18T10:42:11.123Z","stream":"stdout","line":"hello"}
//! ```
//!
//! - `ts` — ISO-8601 UTC timestamp (millisecond precision, trailing `Z`).
//! - `stream` — one of `"stdout"`, `"stderr"`, `"meta"`.
//! - `line` — application payload. Two shapes are supported:
//!   - **Passthrough**: when the child line parses as valid JSON, the parsed
//!     value is embedded verbatim (no double quoting, no escape doubling).
//!   - **Plain text**: otherwise, the value is a JSON string.
//!   - **Binary**: see [`frame_binary`] — the value is `{"b64":"..."}`.
//!
//! # Wire format
//!
//! Envelopes are emitted as **NDJSON** (one JSON object per line, terminated
//! by `\n`). See [`Envelope::to_jsonl_string`].
//!
//! # Pattern reuse
//!
//! The async spawn helper mirrors the channel-bridging pattern used by the
//! WebSocket pty server in `aphrody-terminal-backend` (per-child reader tasks
//! feeding a single `mpsc::Sender<String>` sink).

#![forbid(unsafe_code)]

use base64::Engine as _;
use chrono::SecondsFormat;
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

mod error;
pub use error::JsonOutError;

// ── Public types ──────────────────────────────────────────────────────────────

/// Which underlying stream a framed line originated from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Stream {
    /// Child process stdout (default tool output).
    Stdout,
    /// Child process stderr (diagnostics).
    Stderr,
    /// Out-of-band framing/control event (e.g. spawn/exit notices).
    Meta,
}

impl Stream {
    /// Stable lowercase string form (matches the JSON serialization).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Stdout => "stdout",
            Self::Stderr => "stderr",
            Self::Meta => "meta",
        }
    }
}

/// One framed envelope ready for NDJSON emission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope {
    /// ISO-8601 UTC timestamp (`YYYY-MM-DDTHH:MM:SS.sssZ`).
    pub ts: String,
    /// Origin stream.
    pub stream: Stream,
    /// Application payload — passthrough JSON if the input parsed, otherwise
    /// a JSON string, otherwise `{"b64":"..."}` for binary frames.
    pub line: serde_json::Value,
}

impl Envelope {
    /// Render the envelope as a single NDJSON record (terminated by `\n`).
    ///
    /// # Errors
    ///
    /// Returns [`JsonOutError::Serialize`] if `serde_json` cannot serialize
    /// the envelope — in practice unreachable since every field is a string
    /// or a `serde_json::Value`.
    pub fn to_jsonl_string(&self) -> Result<String, JsonOutError> {
        let mut s = serde_json::to_string(self).map_err(JsonOutError::Serialize)?;
        s.push('\n');
        Ok(s)
    }
}

// ── Framing primitives ────────────────────────────────────────────────────────

/// Current UTC timestamp in millisecond ISO-8601 form (e.g.
/// `2026-05-18T10:42:11.123Z`).
#[must_use]
pub fn now_ts() -> String {
    chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true)
}

/// Frame a single text line. If `line` parses as valid JSON, the parsed value
/// is embedded as-is (passthrough). Otherwise it is wrapped as a JSON string.
#[must_use]
pub fn frame_line(stream: Stream, line: &str) -> Envelope {
    let parsed = serde_json::from_str::<serde_json::Value>(line).ok();
    // Only passthrough when the parsed value is a JSON container or a
    // non-string scalar; treat raw string literals like `"foo"` as plain text
    // so the user-facing payload survives a textual round-trip without loss.
    let payload = match parsed {
        Some(v) if !v.is_string() => v,
        _ => serde_json::Value::String(line.to_string()),
    };
    Envelope {
        ts: now_ts(),
        stream,
        line: payload,
    }
}

/// Frame a binary blob (e.g. an stdout chunk that wasn't valid UTF-8) as
/// `{"b64":"..."}` using standard (padded) base64.
#[must_use]
pub fn frame_binary(stream: Stream, bytes: &[u8]) -> Envelope {
    let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
    let payload = serde_json::json!({ "b64": encoded });
    Envelope {
        ts: now_ts(),
        stream,
        line: payload,
    }
}

// ── Async spawn helper ────────────────────────────────────────────────────────

/// Spawn `cmd`, capture stdout + stderr line-by-line, frame each line into an
/// [`Envelope`], and forward the rendered NDJSON string to `sink`.
///
/// Returns the child's exit code. If the process was terminated by a signal
/// (Unix), `-1` is returned.
///
/// # Errors
///
/// Returns [`JsonOutError`] if the child fails to spawn or its stdout/stderr
/// handles cannot be acquired.
pub async fn spawn_framed_child(
    cmd: &mut Command,
    sink: mpsc::Sender<String>,
) -> Result<i32, JsonOutError> {
    cmd.stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    let mut child = cmd.spawn().map_err(JsonOutError::Spawn)?;

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| JsonOutError::MissingPipe("stdout"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| JsonOutError::MissingPipe("stderr"))?;

    let sink_out = sink.clone();
    let stdout_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stdout).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let env = frame_line(Stream::Stdout, &line);
            match env.to_jsonl_string() {
                Ok(s) => {
                    if sink_out.send(s).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let sink_err = sink.clone();
    let stderr_task = tokio::spawn(async move {
        let mut lines = BufReader::new(stderr).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let env = frame_line(Stream::Stderr, &line);
            match env.to_jsonl_string() {
                Ok(s) => {
                    if sink_err.send(s).await.is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });

    let status = child.wait().await.map_err(JsonOutError::Wait)?;

    // Drain reader tasks before closing the original sink handle so callers
    // observing the receiver see a clean EOS after the exit code is returned.
    let _ = stdout_task.await;
    let _ = stderr_task.await;
    drop(sink);

    Ok(status.code().unwrap_or(-1))
}
