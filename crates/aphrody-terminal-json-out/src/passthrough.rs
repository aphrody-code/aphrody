// SPDX-License-Identifier: Apache-2.0
//! Async line-oriented passthrough: read from any `AsyncBufRead`, classify,
//! frame, and write NDJSON to any `AsyncWrite`.
//!
//! # Behaviour
//!
//! - Reads one line at a time via [`AsyncBufReadExt::read_line`] (matches the
//!   `BufReader::lines` pattern used by `aphrody_terminal_backend` for its
//!   pty reader tasks).
//! - Each line (with the trailing `\n` stripped) is classified via
//!   [`crate::classifier::classify_line`] and wrapped in a [`StdoutFrame`].
//! - The frame is rendered to NDJSON via [`crate::frame::emit_frame`]
//!   (in-memory) and the resulting bytes are written to `writer` in one
//!   `write_all`, preserving NDJSON's one-record-per-line invariant.
//! - Returns the total number of frames emitted (`u64`).
//!
//! # Errors
//!
//! IO errors on the reader / writer halt the loop and propagate as
//! [`JsonOutError`]. Serialisation failures (effectively impossible) likewise
//! propagate.
//!
//! # Pattern reuse
//!
//! - The async-spawn helper [`crate::spawn_framed_child`] already mirrors the
//!   per-pipe-reader pattern from `aphrody_terminal_backend::ws`.
//! - The `tokio::io::AsyncBufRead` bound matches the generic-IO surface used by
//!   the `crates/backend` async reader helpers.

use tokio::io::{AsyncBufReadExt, AsyncWrite, AsyncWriteExt};

use crate::classifier::classify_line;
use crate::error::JsonOutError;
use crate::frame::{FrameSource, StdoutFrame, frame_to_jsonl};

/// Read lines from `reader`, classify + frame each, emit NDJSON to `writer`.
///
/// `source` tags every frame with the same origin (the helper does not
/// disambiguate stdout vs stderr — it's the caller's job to spin up one
/// passthrough per stream with the appropriate tag).
///
/// Returns the total number of frames emitted.
///
/// # Errors
///
/// Returns [`JsonOutError::Io`] on reader/writer IO failure, or
/// [`JsonOutError::Serialize`] on serde failure (unreachable in practice).
pub async fn passthrough<R, W>(
    mut reader: R,
    mut writer: W,
    source: FrameSource,
) -> Result<u64, JsonOutError>
where
    R: AsyncBufReadExt + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut frames: u64 = 0;
    let mut buf = String::new();

    loop {
        buf.clear();
        let n = reader.read_line(&mut buf).await.map_err(JsonOutError::Io)?;
        if n == 0 {
            // EOF.
            break;
        }
        // Strip the trailing newline (and a possible CR) — `read_line` keeps
        // them on success.
        let trimmed_len = buf.trim_end_matches('\n').trim_end_matches('\r').len();
        let line = &buf[..trimmed_len];

        let payload = classify_line(line);
        let frame = StdoutFrame::new(source, payload);
        let rendered = frame_to_jsonl(&frame)?;
        writer.write_all(rendered.as_bytes()).await.map_err(JsonOutError::Io)?;
        frames = frames.saturating_add(1);
    }

    writer.flush().await.map_err(JsonOutError::Io)?;
    Ok(frames)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frame::FramePayload;
    use tokio::io::BufReader;

    #[tokio::test]
    async fn passthrough_text_lines() {
        let input = b"hello\nworld\n";
        let reader = BufReader::new(&input[..]);
        let mut out = Vec::new();
        let n = passthrough(reader, &mut out, FrameSource::App).await.unwrap();
        assert_eq!(n, 2);
        let text = String::from_utf8(out).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(lines.len(), 2);
        for l in &lines {
            let frame: StdoutFrame = serde_json::from_str(l).unwrap();
            assert!(matches!(frame.payload, FramePayload::Text(_)));
        }
    }

    #[tokio::test]
    async fn passthrough_json_line_is_passthrough() {
        let input = b"{\"event\":\"foo\"}\n";
        let reader = BufReader::new(&input[..]);
        let mut out = Vec::new();
        let n = passthrough(reader, &mut out, FrameSource::App).await.unwrap();
        assert_eq!(n, 1);
        let text = String::from_utf8(out).unwrap();
        let frame: StdoutFrame = serde_json::from_str(text.trim_end()).unwrap();
        match frame.payload {
            FramePayload::Json(v) => assert_eq!(v["event"], "foo"),
            other => panic!("expected Json passthrough, got {other:?}"),
        }
    }
}
