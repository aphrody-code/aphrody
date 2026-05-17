// SPDX-License-Identifier: Apache-2.0
//! LLM event bus and multiplexer for aphrody-terminal.
//!
//! This crate is native-only. The WASM renderer uses a separate adapter that
//! bridges the tokio broadcast channel over `wasm-bindgen-futures`.
#![deny(unsafe_code)]

pub mod hook;
pub mod mcp;
pub mod osc;
pub mod skill;
pub mod sub_agent;
pub mod task;

pub use hook::{HookEntry, HookEventLog};
pub use mcp::{McpServerStatus, McpStatusRegistry};
pub use osc::parse_aphrody_llm_osc;
pub use skill::{SkillSlot, SkillState};
pub use sub_agent::{SubAgentInfo, SubAgentRegistry};
pub use task::{TaskNode, TaskTree};

use tokio::sync::broadcast;

/// A single event flowing through the LLM event bus.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum LlmEvent {
    /// Markdown body decoded from `\e]aphrody-md;<b64>\a`.
    Markdown { body: String },
    /// JSON value decoded from `\e]aphrody-json;<b64>\a`.
    JsonInspect { payload: serde_json::Value },
    /// Sub-agent status update from `\e]aphrody-sub-agent;<id>;<status>;<text>\a`.
    SubAgent {
        id: String,
        status: String,
        text: String,
    },
    /// MCP server activity from `\e]aphrody-mcp;<server>;<state>[;<rpc>]\a`.
    Mcp {
        server: String,
        state: String,
        rpc: Option<String>,
    },
    /// Hook firing log entry from `\e]aphrody-hook;<b64-json>\a`.
    Hook {
        event: String,
        payload: serde_json::Value,
    },
    /// Skill invocation log from `\e]aphrody-skill;<b64-json>\a`.
    Skill {
        name: String,
        phase: String,
        payload: serde_json::Value,
    },
    /// Task tree update from `\e]aphrody-task;<id>;<status>;<subject>\a`.
    Task {
        id: String,
        status: String,
        subject: String,
    },
}

/// Fan-out broadcast bus for [`LlmEvent`].
///
/// All methods are synchronous: `publish` sends immediately; `subscribe` returns
/// a receiver. The underlying channel is a tokio broadcast so any number of
/// receivers can exist simultaneously.
pub struct EventBus {
    tx: broadcast::Sender<LlmEvent>,
}

impl EventBus {
    /// Create a new bus with the given channel capacity.
    ///
    /// Capacity is the maximum number of events that can be buffered for a slow
    /// receiver before the oldest are overwritten.
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self { tx }
    }

    /// Publish an event to all current subscribers.
    ///
    /// Returns the number of receivers that received the message, or a
    /// `SendError` if there are no active receivers.
    pub fn publish(
        &self,
        ev: LlmEvent,
    ) -> Result<usize, broadcast::error::SendError<LlmEvent>> {
        self.tx.send(ev)
    }

    /// Subscribe to future events on this bus.
    pub fn subscribe(&self) -> broadcast::Receiver<LlmEvent> {
        self.tx.subscribe()
    }
}
