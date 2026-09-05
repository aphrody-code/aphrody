// SPDX-License-Identifier: Apache-2.0
//! A2A coord JSONL proxy — exposes an append-only JSONL mailbox file (the
//! aphrody/winclean cross-Claude coord inbox) as either a one-shot NDJSON
//! snapshot or a Server-Sent Events (SSE) stream.
//!
//! Consumed by the terminal backend HTTP surface (route `/coord`). The
//! path to the JSONL file is injected by the caller — this module never
//! hard-codes the well-known `C:/winclean/.coord/...` location.
//!
//! # Streaming semantics
//!
//! - **Snapshot mode** (`application/x-ndjson`): the file is read once, line by line, and each
//!   non-empty line is emitted as a single chunk.
//! - **SSE mode** (`text/event-stream`): each non-empty line is wrapped in the canonical `data:
//!   <line>\n\n` envelope per the W3C EventSource spec.
//! - **Tail mode** ([`CoordProxy::tail_stream`]): polls the file size periodically (default 250 ms)
//!   and emits any new bytes appended past the last known offset, splitting them at newline
//!   boundaries.
//!
//! Missing files are treated as the empty stream — never an error — so
//! the route stays useful before the peer has produced any output.
//!
//! # Wire format
//!
//! Lines are forwarded verbatim. The proxy does **not** validate that
//! each line is well-formed JSON; that is the producer's responsibility
//! (here, the bun listener writing the `.coord/inbox-from-aphrody.jsonl`
//! file). Empty lines (`""` after trimming `\r\n`) are dropped.

use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
    pin::Pin,
    time::Duration,
};

use futures_util::stream::{self, BoxStream, Stream, StreamExt};
use thiserror::Error;
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, AsyncReadExt, AsyncSeekExt, BufReader, SeekFrom},
    time,
};

/// Default polling interval for [`CoordProxy::tail_stream`].
pub const DEFAULT_TAIL_INTERVAL: Duration = Duration::from_millis(250);

/// MIME type for the snapshot response.
pub const NDJSON_CONTENT_TYPE: &str = "application/x-ndjson";

/// MIME type for the SSE stream response.
pub const SSE_CONTENT_TYPE: &str = "text/event-stream";

/// Errors emitted by the coord proxy.
#[derive(Debug, Error)]
pub enum CoordError {
    /// Underlying filesystem I/O failed.
    #[error("coord I/O: {0}")]
    Io(#[from] std::io::Error),
}

/// Response body kind requested by the client.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoordResponseKind {
    /// One-shot snapshot, `application/x-ndjson`.
    Snapshot,
    /// Server-Sent Events stream, `text/event-stream`.
    Sse,
}

impl CoordResponseKind {
    /// Pick the response kind from a raw `Accept` header value.
    ///
    /// Returns [`CoordResponseKind::Sse`] iff the header contains
    /// `text/event-stream` (case-insensitive), otherwise
    /// [`CoordResponseKind::Snapshot`].
    #[must_use]
    pub fn from_accept(accept: Option<&str>) -> Self {
        match accept {
            Some(value) if value.to_ascii_lowercase().contains("text/event-stream") => Self::Sse,
            _ => Self::Snapshot,
        }
    }

    /// Canonical `Content-Type` value for this response kind.
    #[must_use]
    pub fn content_type(self) -> &'static str {
        match self {
            Self::Snapshot => NDJSON_CONTENT_TYPE,
            Self::Sse => SSE_CONTENT_TYPE,
        }
    }
}

/// Minimal HTTP-ish response payload returned by [`CoordProxy::handle_get_coord`].
///
/// The backend currently exposes a WebSocket-only surface, so we model
/// the HTTP response abstractly: callers wire `status` / `content_type`
/// / `body` into whichever HTTP server they bolt on (axum, hyper, the
/// existing tungstenite TCP listener via a manual upgrade path, etc.).
pub struct HttpResponse {
    /// HTTP status code.
    pub status: u16,
    /// Value for the `Content-Type` header.
    pub content_type: &'static str,
    /// Streaming body — each item is a fully-formed chunk ready to be
    /// flushed to the socket (NDJSON line for snapshot, `data: ...\n\n`
    /// envelope for SSE).
    pub body: Pin<Box<dyn Stream<Item = Result<String, CoordError>> + Send>>,
}

impl std::fmt::Debug for HttpResponse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpResponse")
            .field("status", &self.status)
            .field("content_type", &self.content_type)
            .field("body", &"<stream>")
            .finish()
    }
}

/// Proxy over an append-only JSONL mailbox file.
///
/// Cheap to clone (just a `PathBuf`).
#[derive(Debug, Clone)]
pub struct CoordProxy {
    jsonl_path: PathBuf,
    tail_interval: Duration,
}

impl CoordProxy {
    /// Construct a new proxy bound to `jsonl_path`.
    ///
    /// The path is **not** validated at construction time — it may not
    /// yet exist. Missing files surface as empty streams at request time.
    #[must_use]
    pub fn new(jsonl_path: PathBuf) -> Self {
        Self { jsonl_path, tail_interval: DEFAULT_TAIL_INTERVAL }
    }

    /// Override the tail-mode polling interval.
    #[must_use]
    pub fn with_tail_interval(mut self, interval: Duration) -> Self {
        self.tail_interval = interval;
        self
    }

    /// Returns the JSONL path this proxy is bound to.
    #[must_use]
    pub fn jsonl_path(&self) -> &Path {
        &self.jsonl_path
    }

    /// Serve a coord request.
    ///
    /// `accept` is the raw `Accept` header value. Pass `None` to default
    /// to NDJSON snapshot.
    ///
    /// # Errors
    ///
    /// Only fatal I/O errors propagate from this function. A missing
    /// file is **not** an error — it yields an empty body with `200 OK`.
    pub async fn handle_get_coord(&self, accept: Option<&str>) -> Result<HttpResponse, CoordError> {
        let kind = CoordResponseKind::from_accept(accept);
        let body: Pin<Box<dyn Stream<Item = Result<String, CoordError>> + Send>> = match kind {
            CoordResponseKind::Snapshot => Box::pin(self.snapshot_stream().await?),
            CoordResponseKind::Sse => Box::pin(self.sse_snapshot_stream().await?),
        };
        Ok(HttpResponse { status: 200, content_type: kind.content_type(), body })
    }

    /// One-shot snapshot: reads the file once and emits each non-empty
    /// line (terminated with `\n`).
    ///
    /// Missing file → empty stream.
    ///
    /// # Errors
    ///
    /// Returns [`CoordError::Io`] for any non-`NotFound` filesystem
    /// failure (permission denied, I/O hardware error, …).
    pub async fn snapshot_stream(
        &self,
    ) -> Result<BoxStream<'static, Result<String, CoordError>>, CoordError> {
        let lines = read_all_lines(&self.jsonl_path).await?;
        let mapped = lines.into_iter().map(|line| Ok(format!("{line}\n")));
        Ok(stream::iter(mapped).boxed())
    }

    /// One-shot snapshot in SSE format: each non-empty line becomes a
    /// `data: <line>\n\n` event.
    ///
    /// Missing file → empty stream.
    ///
    /// # Errors
    ///
    /// Returns [`CoordError::Io`] for any non-`NotFound` filesystem
    /// failure.
    pub async fn sse_snapshot_stream(
        &self,
    ) -> Result<BoxStream<'static, Result<String, CoordError>>, CoordError> {
        let lines = read_all_lines(&self.jsonl_path).await?;
        let mapped = lines.into_iter().map(|line| Ok(encode_sse(&line)));
        Ok(stream::iter(mapped).boxed())
    }

    /// `tail -f`-style stream: emits each new non-empty line appended to
    /// the file after the call. Lines already present at call time are
    /// **not** replayed — combine with [`Self::snapshot_stream`] if you
    /// want a backfill + live feed.
    ///
    /// Missing file: the stream waits, polling at `tail_interval`, until
    /// the file appears.
    ///
    /// The byte offset is **seeded eagerly** here (not on first poll),
    /// so any data appended between the call to `tail_stream()` and the
    /// first `.next().await` is reliably observed.
    pub async fn tail_stream(&self) -> BoxStream<'static, Result<String, CoordError>> {
        let offset = seed_offset(&self.jsonl_path).await;
        Box::pin(tail_impl(self.jsonl_path.clone(), self.tail_interval, false, offset))
    }

    /// Like [`Self::tail_stream`] but each emitted item is already
    /// wrapped in the SSE `data: <line>\n\n` envelope.
    pub async fn tail_stream_sse(&self) -> BoxStream<'static, Result<String, CoordError>> {
        let offset = seed_offset(&self.jsonl_path).await;
        Box::pin(tail_impl(self.jsonl_path.clone(), self.tail_interval, true, offset))
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

/// Read every non-empty line of `path` into memory.
///
/// Returns an empty `Vec` if the file does not exist.
async fn read_all_lines(path: &Path) -> Result<Vec<String>, CoordError> {
    let file = match File::open(path).await {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(CoordError::Io(e)),
    };
    let reader = BufReader::new(file);
    let mut lines = reader.lines();
    let mut out = Vec::new();
    while let Some(line) = lines.next_line().await? {
        let trimmed = line.trim_end_matches('\r');
        if !trimmed.is_empty() {
            out.push(trimmed.to_owned());
        }
    }
    Ok(out)
}

/// Format a single line as a SSE `data: ...` event.
///
/// The W3C EventSource grammar requires every `\n` in `data` to be
/// re-prefixed with `data: `. Input is normally a single JSONL line
/// (newline-free), but defend against rogue `\n` anyway.
fn encode_sse(line: &str) -> String {
    let mut buf = String::with_capacity(line.len() + 8);
    for chunk in line.split('\n') {
        buf.push_str("data: ");
        buf.push_str(chunk);
        buf.push('\n');
    }
    buf.push('\n');
    buf
}

/// State carried across iterations of the tail-stream `unfold`.
struct TailState {
    path: PathBuf,
    interval: Duration,
    sse: bool,
    /// Byte offset already consumed from the file. Sentinel `None`
    /// means "not yet seeded": on the first successful open we seek to
    /// end-of-file and record the position here. The seeding step is
    /// normally performed eagerly by [`seed_offset`] before the stream
    /// is constructed, but we keep the `None` fallback so a file that
    /// only appears mid-stream is handled gracefully.
    offset: Option<u64>,
    /// Lines parsed from the latest read but not yet yielded.
    pending: VecDeque<String>,
}

/// Eagerly capture the current end-of-file offset for `path`.
///
/// Returns `Some(len)` if the file exists, `None` if it does not — in
/// which case the tail loop will seed on first appearance.
async fn seed_offset(path: &Path) -> Option<u64> {
    match File::open(path).await {
        Ok(mut f) => f.seek(SeekFrom::End(0)).await.ok(),
        Err(_) => None,
    }
}

/// Tail implementation shared by [`CoordProxy::tail_stream`] and
/// [`CoordProxy::tail_stream_sse`].
fn tail_impl(
    path: PathBuf,
    interval: Duration,
    sse: bool,
    initial_offset: Option<u64>,
) -> impl Stream<Item = Result<String, CoordError>> + Send + 'static {
    let init = TailState { path, interval, sse, offset: initial_offset, pending: VecDeque::new() };

    stream::unfold(Some(init), |maybe_state| async move {
        let mut state = maybe_state?;
        loop {
            // 1) Drain any buffered lines from a previous read.
            if let Some(line) = state.pending.pop_front() {
                let item = if state.sse { encode_sse(&line) } else { format!("{line}\n") };
                return Some((Ok(item), Some(state)));
            }

            // 2) Attempt to read new bytes from the file.
            match poll_new_lines(&mut state).await {
                Ok(true) => {
                    // New lines were appended to `state.pending` — loop
                    // back to drain them.
                    continue;
                },
                Ok(false) => {
                    // Nothing new — sleep and poll again.
                    time::sleep(state.interval).await;
                },
                Err(e) => {
                    // Surface the error and terminate the stream.
                    return Some((Err(e), None));
                },
            }
        }
    })
}

/// Poll the file pointed to by `state.path` for newly appended bytes,
/// pushing any complete lines into `state.pending`.
///
/// Returns `Ok(true)` if at least one new line is now pending,
/// `Ok(false)` if there was nothing new (file missing or unchanged),
/// `Err(_)` for fatal I/O failures.
async fn poll_new_lines(state: &mut TailState) -> Result<bool, CoordError> {
    let mut file = match File::open(&state.path).await {
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => return Err(CoordError::Io(e)),
    };

    let end = file.seek(SeekFrom::End(0)).await?;

    // Resolve the previous offset. If we had no eager seed (the file
    // was missing when the stream was constructed and only appeared
    // now), treat the entire file as new content — this matches the
    // operator expectation: "wait for the file, then show everything".
    let prev_offset = state.offset.unwrap_or(0);

    // File rotated / truncated → rebase to start.
    let start_offset = if end < prev_offset { 0 } else { prev_offset };

    if end <= start_offset {
        // Update offset in case we rebased without new data, then bail.
        state.offset = Some(start_offset);
        return Ok(false);
    }

    file.seek(SeekFrom::Start(start_offset)).await?;
    let mut buf = String::new();
    let n = file.read_to_string(&mut buf).await?;
    state.offset = Some(start_offset.saturating_add(n as u64));

    let mut produced = false;
    for line in buf.split('\n') {
        let trimmed = line.trim_end_matches('\r');
        if !trimmed.is_empty() {
            state.pending.push_back(trimmed.to_owned());
            produced = true;
        }
    }
    Ok(produced)
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_sse_simple_line() {
        let line = r#"{"hello":"world"}"#;
        assert_eq!(encode_sse(line), "data: {\"hello\":\"world\"}\n\n");
    }

    #[test]
    fn encode_sse_multiline_payload() {
        // Defensive: embedded newline becomes a second `data:` line.
        let payload = "a\nb";
        assert_eq!(encode_sse(payload), "data: a\ndata: b\n\n");
    }

    #[test]
    fn from_accept_picks_sse_when_event_stream_present() {
        assert_eq!(
            CoordResponseKind::from_accept(Some("text/event-stream")),
            CoordResponseKind::Sse,
        );
        assert_eq!(
            CoordResponseKind::from_accept(Some("application/json, TEXT/EVENT-STREAM;q=0.9")),
            CoordResponseKind::Sse,
        );
    }

    #[test]
    fn from_accept_defaults_to_snapshot() {
        assert_eq!(CoordResponseKind::from_accept(None), CoordResponseKind::Snapshot,);
        assert_eq!(
            CoordResponseKind::from_accept(Some("application/json")),
            CoordResponseKind::Snapshot,
        );
    }
}
