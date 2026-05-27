// SPDX-License-Identifier: Apache-2.0
//! Model selection: a [`ModelChoice`] that yields an
//! [`Arc<dyn ModelClient>`](aphrody_model_client::ModelClient), plus an offline
//! [`StubModelClient`] that replays a scripted event sequence.

use std::pin::Pin;
use std::sync::Arc;
use std::sync::Mutex;

use aphrody_model_client::{
    ChatTurn, GeminiClient, ModelClient, ModelError, ModelStream, ModelStreamEvent,
};
use futures::Stream;

/// Which backend a runtime drives its turns through.
///
/// Construct one with [`ModelChoice::gemini`] for a live Gemini client, or
/// [`ModelChoice::stub`] for a deterministic, network-free replay used by tests
/// and offline surfaces.
#[derive(Clone)]
pub enum ModelChoice {
    /// A live, streaming Gemini `generateContent` client.
    Gemini(Arc<GeminiClient>),
    /// An offline client that replays a fixed sequence of events.
    Stub(Arc<StubModelClient>),
}

impl std::fmt::Debug for ModelChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Gemini(client) => f
                .debug_tuple("Gemini")
                .field(&client.model_id().to_string())
                .finish(),
            Self::Stub(_) => f.write_str("Stub"),
        }
    }
}

impl ModelChoice {
    /// Build a live Gemini client for `model` authenticating with `api_key`.
    ///
    /// This constructs a `reqwest::Client` internally. To avoid the rustls 0.23
    /// "no provider set" panic (CLAUDE.md §7) the ring crypto provider is
    /// installed as the process default on first use; the call is idempotent and
    /// a no-op if a provider is already installed.
    #[must_use]
    pub fn gemini(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        install_default_crypto_provider();
        Self::Gemini(Arc::new(GeminiClient::new(api_key, model)))
    }

    /// Build an offline stub client that replays `client`.
    #[must_use]
    pub fn stub(client: StubModelClient) -> Self {
        Self::Stub(Arc::new(client))
    }

    /// Erase the choice into the trait object the engine consumes.
    #[must_use]
    pub(crate) fn into_client(self) -> Arc<dyn ModelClient> {
        match self {
            Self::Gemini(client) => client,
            Self::Stub(client) => client,
        }
    }
}

/// Install the ring crypto provider as the process default (idempotent).
fn install_default_crypto_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}

/// One scripted turn: the events the [`StubModelClient`] emits for a single
/// model call.
///
/// `Created` is prepended and `Completed` is appended automatically by the
/// stub, so a turn only lists the meaningful middle events
/// ([`ModelStreamEvent::TextDelta`], [`ModelStreamEvent::ReasoningDelta`],
/// [`ModelStreamEvent::ToolCall`]).
#[derive(Debug, Clone, Default)]
pub struct ScriptedTurn {
    /// The events emitted between `Created` and `Completed` for this call.
    pub events: Vec<ModelStreamEvent>,
}

impl ScriptedTurn {
    /// A turn that emits a single block of visible text.
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            events: vec![ModelStreamEvent::TextDelta(text.into())],
        }
    }

    /// A turn that requests a single tool call.
    #[must_use]
    pub fn tool_call(
        id: impl Into<String>,
        name: impl Into<String>,
        arguments: serde_json::Value,
    ) -> Self {
        Self {
            events: vec![ModelStreamEvent::ToolCall {
                id: id.into(),
                name: name.into(),
                arguments,
            }],
        }
    }

    /// Append an event to this turn (builder style).
    #[must_use]
    pub fn with_event(mut self, event: ModelStreamEvent) -> Self {
        self.events.push(event);
        self
    }
}

/// A [`ModelClient`] that replays a fixed list of [`ScriptedTurn`]s.
///
/// Each call to [`ModelClient::stream`] consumes the next scripted turn (in
/// order), wrapping its events between an automatic
/// [`ModelStreamEvent::Created`] and [`ModelStreamEvent::Completed`]. This is
/// exactly what the engine turn loop needs to prove a multi-step interaction
/// offline: script a [`ModelStreamEvent::ToolCall`] for the first call and a
/// final [`ModelStreamEvent::TextDelta`] for the second, and the engine will run
/// the tool, re-inject its result, then finish on the text.
///
/// When the script is exhausted the client returns an empty (just
/// `Created` + `Completed`) turn so a runaway loop terminates rather than
/// panicking.
pub struct StubModelClient {
    model: String,
    turns: Mutex<std::collections::VecDeque<ScriptedTurn>>,
}

impl std::fmt::Debug for StubModelClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let remaining = self
            .turns
            .lock()
            .map(|q| q.len())
            .unwrap_or_default();
        f.debug_struct("StubModelClient")
            .field("model", &self.model)
            .field("remaining_turns", &remaining)
            .finish()
    }
}

impl StubModelClient {
    /// Build a stub for `model` that replays `turns` in order.
    #[must_use]
    pub fn new(model: impl Into<String>, turns: Vec<ScriptedTurn>) -> Self {
        Self {
            model: model.into(),
            turns: Mutex::new(turns.into_iter().collect()),
        }
    }

    /// Number of scripted turns not yet consumed.
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.turns
            .lock()
            .map(|q| q.len())
            .unwrap_or_default()
    }

    /// Pop the next scripted turn, or an empty one once the script is spent.
    fn next_turn(&self) -> ScriptedTurn {
        self.turns
            .lock()
            .map(|mut q| q.pop_front())
            .ok()
            .flatten()
            .unwrap_or_default()
    }
}

#[async_trait::async_trait]
impl ModelClient for StubModelClient {
    async fn stream(&self, _turn: ChatTurn) -> Result<ModelStream, ModelError> {
        let turn = self.next_turn();
        let mut events: Vec<Result<ModelStreamEvent, ModelError>> = Vec::with_capacity(turn.events.len() + 2);
        events.push(Ok(ModelStreamEvent::Created));
        events.extend(turn.events.into_iter().map(Ok));
        events.push(Ok(ModelStreamEvent::Completed {
            usage: None,
            finish_reason: Some("STOP".to_string()),
        }));

        let stream: Pin<Box<dyn Stream<Item = Result<ModelStreamEvent, ModelError>> + Send>> =
            Box::pin(futures::stream::iter(events));
        Ok(stream)
    }

    fn model_id(&self) -> &str {
        &self.model
    }
}
