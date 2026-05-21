// SPDX-License-Identifier: Apache-2.0
//! Agent lifecycle and turn loop.
//!
//! TS provenance: `packages/gemini-cli/packages/sdk/src/agent.ts` +
//! `session.ts` (the bulk of the turn loop lives on `GeminiCliSession` in TS).
//!
//! In Rust we surface a stable [`Agent`] that owns a [`Backend`] trait object
//! and forwards `single_turn` / `stream_turn` to it. Two backends ship in v1:
//!
//! * [`StubBackend`] — deterministic echo backend, never hits the network.
//!   Used by tests and by callers that have not configured a live provider.
//! * [`GeminiBackend`] — adapter on top of [`gemini_runtime::detect`] that
//!   spawns the local `gemini` CLI and streams JSON events back. Only
//!   active when the binary is reachable on `PATH`.

use std::pin::Pin;
use std::sync::Arc;
use std::time::Instant;

use futures_util::Stream;
use tokio::sync::Mutex;

use crate::types::{SdkConfig, SdkError, SdkResult, TurnResult};

// ---------------------------------------------------------------------------
// Backend trait — keeps the agent backend-agnostic.
// ---------------------------------------------------------------------------

/// One token / chunk of the streaming response, exposed to consumers.
///
/// PUBLIC API — semver stable from v1.0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamChunk {
    /// A textual token / fragment to be appended to the assistant reply.
    Text(String),
    /// The model asked to invoke `name` with the supplied JSON arguments.
    ToolCall {
        /// Tool name.
        name: String,
        /// Tool arguments as a JSON object.
        args: serde_json::Value,
    },
    /// Stream is done (terminal). No further chunks will be emitted.
    Done,
}

/// Backend trait implemented by every model provider we expose through the
/// SDK. Always lives behind an `Arc<dyn Backend>` on the [`Agent`].
///
/// PUBLIC API — semver stable from v1.0.
#[async_trait::async_trait]
pub trait Backend: Send + Sync + std::fmt::Debug {
    /// One-shot turn: send `user_input`, collect the assistant reply, return
    /// the concatenated text.
    async fn single_turn(&self, user_input: &str) -> SdkResult<String>;

    /// Streaming turn: yield [`StreamChunk`] events as they arrive.
    async fn stream_turn(
        &self,
        user_input: &str,
    ) -> SdkResult<Pin<Box<dyn Stream<Item = StreamChunk> + Send>>>;
}

// ---------------------------------------------------------------------------
// StubBackend — deterministic, no network.
// ---------------------------------------------------------------------------

/// Deterministic backend used by tests and the SDK quickstart.
///
/// PUBLIC API — semver stable from v1.0.
#[derive(Debug, Clone)]
pub struct StubBackend {
    reply_template: String,
}

impl StubBackend {
    /// Build a stub that returns `reply` verbatim regardless of input.
    ///
    /// PUBLIC API — semver stable from v1.0.
    #[must_use]
    pub fn with_reply(reply: impl Into<String>) -> Self {
        Self {
            reply_template: reply.into(),
        }
    }

    /// Build a stub that echoes the input prefixed with `"echo: "`.
    ///
    /// PUBLIC API — semver stable from v1.0.
    #[must_use]
    pub fn echo() -> Self {
        Self {
            reply_template: String::from("echo: {input}"),
        }
    }
}

impl Default for StubBackend {
    fn default() -> Self {
        Self::echo()
    }
}

#[async_trait::async_trait]
impl Backend for StubBackend {
    async fn single_turn(&self, user_input: &str) -> SdkResult<String> {
        Ok(self.reply_template.replace("{input}", user_input))
    }

    async fn stream_turn(
        &self,
        user_input: &str,
    ) -> SdkResult<Pin<Box<dyn Stream<Item = StreamChunk> + Send>>> {
        let reply = self.reply_template.replace("{input}", user_input);
        let chunks = vec![StreamChunk::Text(reply), StreamChunk::Done];
        let stream = futures_util::stream::iter(chunks);
        Ok(Box::pin(stream))
    }
}

// ---------------------------------------------------------------------------
// GeminiBackend — wires gemini-runtime.
// ---------------------------------------------------------------------------

/// Backend adapter on top of the local `gemini` CLI binary.
///
/// Detection is performed lazily on first call so constructing an `Agent`
/// without `gemini` installed remains cheap. When the binary is unreachable,
/// every call returns [`SdkError::MissingRuntime`].
///
/// PUBLIC API — semver stable from v1.0.
#[derive(Debug, Clone, Default)]
pub struct GeminiBackend;

#[async_trait::async_trait]
impl Backend for GeminiBackend {
    async fn single_turn(&self, user_input: &str) -> SdkResult<String> {
        let runtime = gemini_runtime::detect()
            .await
            .map_err(|e| SdkError::MissingRuntime(format!("gemini detect: {e}")))?;
        let opts = gemini_runtime::TurnOptions::default();
        let mut stream = runtime.turn(user_input, &opts).await;
        let mut out = String::new();
        use futures_util::StreamExt as _;
        while let Some(event) = stream.next().await {
            match event {
                gemini_runtime::StreamEvent::Text(t) => out.push_str(&t),
                gemini_runtime::StreamEvent::Done { .. }
                | gemini_runtime::StreamEvent::ToolCall { .. }
                | gemini_runtime::StreamEvent::Unknown(_) => {}
                gemini_runtime::StreamEvent::Error { message, .. } => {
                    return Err(SdkError::Backend(message));
                }
            }
        }
        Ok(out)
    }

    async fn stream_turn(
        &self,
        user_input: &str,
    ) -> SdkResult<Pin<Box<dyn Stream<Item = StreamChunk> + Send>>> {
        // gemini_runtime::Runtime::turn returns a BoxStream borrowing from
        // `self`, `prompt`, and `opts` — none of which can outlive this
        // function. We sidestep the lifetime by spawning an owning task that
        // pumps events into an `mpsc::Sender` and surface a `ReceiverStream`
        // (which IS `'static`). This preserves the streaming property end-
        // to-end without forcing the consumer to drain into a `Vec` first.
        let input = user_input.to_string();
        let (tx, rx) = tokio::sync::mpsc::channel::<StreamChunk>(16);
        tokio::spawn(async move {
            let runtime = match gemini_runtime::detect().await {
                Ok(r) => r,
                Err(_) => {
                    let _ = tx.send(StreamChunk::Done).await;
                    return;
                }
            };
            let opts = gemini_runtime::TurnOptions::default();
            let mut stream = runtime.turn(&input, &opts).await;
            use futures_util::StreamExt as _;
            while let Some(event) = stream.next().await {
                let chunk = match event {
                    gemini_runtime::StreamEvent::Text(t) => StreamChunk::Text(t),
                    gemini_runtime::StreamEvent::ToolCall { name, arguments, .. } => {
                        StreamChunk::ToolCall { name, args: arguments }
                    }
                    gemini_runtime::StreamEvent::Done { .. }
                    | gemini_runtime::StreamEvent::Error { .. }
                    | gemini_runtime::StreamEvent::Unknown(_) => StreamChunk::Done,
                };
                if tx.send(chunk).await.is_err() {
                    break;
                }
            }
            let _ = tx.send(StreamChunk::Done).await;
        });
        Ok(Box::pin(tokio_stream::wrappers::ReceiverStream::new(rx)))
    }
}

// ---------------------------------------------------------------------------
// Agent
// ---------------------------------------------------------------------------

/// The agent surface used by SDK consumers.
///
/// Construct via [`Agent::with_backend`] (or via [`crate::AphrodySdk`] which
/// wires the building blocks for you).
///
/// PUBLIC API — semver stable from v1.0.
#[derive(Clone)]
pub struct Agent {
    backend: Arc<dyn Backend>,
    config: Arc<SdkConfig>,
    last_turn_metrics: Arc<Mutex<Option<TurnResult>>>,
}

impl std::fmt::Debug for Agent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Agent")
            .field("backend", &self.backend)
            .field("model", &self.config.model)
            .finish_non_exhaustive()
    }
}

impl Agent {
    /// Build an agent that delegates to `backend`.
    ///
    /// PUBLIC API — semver stable from v1.0.
    #[must_use]
    pub fn with_backend(config: Arc<SdkConfig>, backend: Arc<dyn Backend>) -> Self {
        Self {
            backend,
            config,
            last_turn_metrics: Arc::new(Mutex::new(None)),
        }
    }

    /// Borrow the active configuration snapshot.
    #[must_use]
    pub fn config(&self) -> &SdkConfig {
        &self.config
    }

    /// Borrow the underlying backend (for advanced consumers).
    #[must_use]
    pub fn backend(&self) -> &Arc<dyn Backend> {
        &self.backend
    }

    /// One-shot turn: yields the concatenated assistant reply.
    ///
    /// # Errors
    /// Propagates any [`SdkError`] raised by the backend.
    pub async fn single_turn(&self, user_input: &str) -> SdkResult<String> {
        let started = Instant::now();
        let reply = self.backend.single_turn(user_input).await?;
        let elapsed = started.elapsed();
        *self.last_turn_metrics.lock().await = Some(TurnResult {
            assistant: reply.clone(),
            tool_call_count: 0,
            duration_ms: elapsed.as_millis() as u64,
        });
        Ok(reply)
    }

    /// Streaming turn: yields [`StreamChunk`] events as they arrive.
    ///
    /// # Errors
    /// Returns an error when the backend cannot start the stream. Errors
    /// raised *during* streaming are folded into a terminal [`StreamChunk::Done`].
    pub async fn stream_turn(
        &self,
        user_input: &str,
    ) -> SdkResult<Pin<Box<dyn Stream<Item = StreamChunk> + Send>>> {
        self.backend.stream_turn(user_input).await
    }

    /// Retrieve the metrics of the most recent `single_turn`. Returns `None`
    /// when no turn has run yet.
    pub async fn last_turn_metrics(&self) -> Option<TurnResult> {
        self.last_turn_metrics.lock().await.clone()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use futures_util::StreamExt as _;

    #[tokio::test]
    async fn stub_single_turn_echoes() {
        let cfg = Arc::new(SdkConfig::default());
        let agent = Agent::with_backend(cfg, Arc::new(StubBackend::echo()));
        let r = agent.single_turn("hello").await.unwrap();
        assert_eq!(r, "echo: hello");
    }

    #[tokio::test]
    async fn stub_stream_turn_emits_text_then_done() {
        let cfg = Arc::new(SdkConfig::default());
        let agent = Agent::with_backend(cfg, Arc::new(StubBackend::with_reply("hi")));
        let mut s = agent.stream_turn("anything").await.unwrap();
        let mut got_text = false;
        let mut got_done = false;
        while let Some(c) = s.next().await {
            match c {
                StreamChunk::Text(t) => {
                    assert_eq!(t, "hi");
                    got_text = true;
                }
                StreamChunk::Done => got_done = true,
                StreamChunk::ToolCall { .. } => panic!("unexpected tool call"),
            }
        }
        assert!(got_text && got_done);
    }

    #[tokio::test]
    async fn metrics_populated_after_single_turn() {
        let cfg = Arc::new(SdkConfig::default());
        let agent = Agent::with_backend(cfg, Arc::new(StubBackend::echo()));
        assert!(agent.last_turn_metrics().await.is_none());
        let _ = agent.single_turn("ping").await.unwrap();
        let m = agent.last_turn_metrics().await.expect("metrics");
        assert_eq!(m.assistant, "echo: ping");
        assert_eq!(m.tool_call_count, 0);
    }
}
