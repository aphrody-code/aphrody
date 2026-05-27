// SPDX-License-Identifier: Apache-2.0
//! Agent session: the configuration plus the running conversation history.

use aphrody_model_client::{ChatMessage, Part, Role};
use serde_json::Value;

use crate::config::EngineConfig;

/// A single agent conversation.
///
/// A session owns its [`EngineConfig`] and the full [`ChatMessage`] history. It
/// is the unit of state threaded through [`run_turn`](crate::run_turn): each
/// turn appends the user input, the model's visible message(s), and any tool
/// call / response pairs so that subsequent turns see the complete context.
#[derive(Debug, Clone)]
pub struct Session {
    config: EngineConfig,
    history: Vec<ChatMessage>,
}

impl Session {
    /// Create an empty session driven by `config`.
    #[must_use]
    pub fn new(config: EngineConfig) -> Self {
        Self {
            config,
            history: Vec::new(),
        }
    }

    /// The session's configuration.
    #[must_use]
    pub fn config(&self) -> &EngineConfig {
        &self.config
    }

    /// The full conversation history, oldest first.
    #[must_use]
    pub fn history(&self) -> &[ChatMessage] {
        &self.history
    }

    /// Number of messages currently in the history.
    #[must_use]
    pub fn len(&self) -> usize {
        self.history.len()
    }

    /// Whether the history is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.history.is_empty()
    }

    /// Append a fully-formed message to the history.
    pub fn push_message(&mut self, message: ChatMessage) {
        self.history.push(message);
    }

    /// Append a user message composed of `parts`.
    pub fn push_user(&mut self, parts: Vec<Part>) {
        self.push_message(ChatMessage {
            role: Role::User,
            parts,
        });
    }

    /// Append a model text message.
    pub fn push_model_text(&mut self, text: impl Into<String>) {
        self.push_message(ChatMessage {
            role: Role::Model,
            parts: vec![Part::Text(text.into())],
        });
    }

    /// Append a model function-call message (the model asked to run `name`).
    pub fn push_model_function_call(&mut self, name: impl Into<String>, args: Value) {
        self.push_message(ChatMessage {
            role: Role::Model,
            parts: vec![Part::FunctionCall {
                name: name.into(),
                args,
            }],
        });
    }

    /// Append a tool function-response message (the result fed back to the
    /// model).
    pub fn push_tool_response(&mut self, name: impl Into<String>, response: Value) {
        self.push_message(ChatMessage {
            role: Role::Tool,
            parts: vec![Part::FunctionResponse {
                name: name.into(),
                response,
            }],
        });
    }
}
