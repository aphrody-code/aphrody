// SPDX-License-Identifier: Apache-2.0
//! Top-level DTOs consumed by [`crate::turn_loop::ChatLoop`].
//!
//! The shapes here are deliberately **vendor-neutral**: a [`Message`] is a
//! `(role, content, timestamp, optional tool_calls)` triple, regardless of
//! whether the underlying model is Claude, Gemini, or the Antigravity
//! gateway. Backend implementations translate these DTOs into their native
//! wire format inside [`crate::backend`].
//!
//! ## Cohesion with sibling crates
//!
//! [`MessageRole`] mirrors [`aphrody_router::Role`] /
//! [`aphrody_session::TurnRole`] /
//! [`aphrody_context::MessageRole`] one-to-one (four variants:
//! `system` / `user` / `assistant` / `tool`). We re-state the enum here so the
//! `aphrody-chat` public surface is not transitively coupled to whichever of
//! those three crates a downstream caller already happens to depend on; the
//! [`MessageRole::to_router`] / [`MessageRole::to_session`] /
//! [`MessageRole::to_context`] helpers do the in-crate conversion at the seam.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// MessageRole — wire-shared four-variant enum
// ---------------------------------------------------------------------------

/// Role of the speaker on the conversation thread.
///
/// Identical in spirit to [`aphrody_router::Role`] /
/// [`aphrody_session::TurnRole`] / [`aphrody_context::MessageRole`]; the
/// conversion helpers ensure they round-trip without loss.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// Persistent system prompt — almost always pinned at the head of the
    /// context window.
    System,
    /// Human / driver-side message.
    User,
    /// Assistant / model reply.
    Assistant,
    /// Tool / function call result returned to the model.
    Tool,
}

impl MessageRole {
    /// Stable lowercase identifier suitable for logs and JSON labelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Assistant => "assistant",
            Self::Tool => "tool",
        }
    }

    /// Project into [`aphrody_router::Role`] for backend dispatch.
    #[must_use]
    pub const fn to_router(self) -> aphrody_router::Role {
        match self {
            Self::System => aphrody_router::Role::System,
            Self::User => aphrody_router::Role::User,
            Self::Assistant => aphrody_router::Role::Assistant,
            Self::Tool => aphrody_router::Role::Tool,
        }
    }

    /// Project into [`aphrody_session::TurnRole`] for the JSONL session log.
    #[must_use]
    pub const fn to_session(self) -> aphrody_session::TurnRole {
        match self {
            Self::System => aphrody_session::TurnRole::System,
            Self::User => aphrody_session::TurnRole::User,
            Self::Assistant => aphrody_session::TurnRole::Assistant,
            Self::Tool => aphrody_session::TurnRole::Tool,
        }
    }

    /// Project into [`aphrody_context::MessageRole`] for the live window.
    #[must_use]
    pub const fn to_context(self) -> aphrody_context::MessageRole {
        match self {
            Self::System => aphrody_context::MessageRole::System,
            Self::User => aphrody_context::MessageRole::User,
            Self::Assistant => aphrody_context::MessageRole::Assistant,
            Self::Tool => aphrody_context::MessageRole::Tool,
        }
    }
}

// ---------------------------------------------------------------------------
// Message — single conversation entry
// ---------------------------------------------------------------------------

/// One message on the conversation thread.
///
/// `tool_calls` is non-empty when the assistant requested one or more tool
/// invocations during this turn. Each [`ToolInvocation`] carries enough
/// information for the orchestrator to dispatch it through the registered
/// [`aphrody_tools::ToolRegistry`] and gate it through
/// [`aphrody_skills::permissions::PermissionEngine::evaluate`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    /// Who produced this message.
    pub role: MessageRole,
    /// UTF-8 content body.
    pub content: String,
    /// Wall-clock timestamp of when the message was finalised.
    pub timestamp: DateTime<Utc>,
    /// Tool invocations the assistant requested as part of this message.
    /// Empty for user / system / tool-result messages.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tool_calls: Vec<ToolInvocation>,
}

impl Message {
    /// Build a user message stamped at `Utc::now()`.
    #[must_use]
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: content.into(),
            timestamp: Utc::now(),
            tool_calls: Vec::new(),
        }
    }

    /// Build a system message.
    #[must_use]
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::System,
            content: content.into(),
            timestamp: Utc::now(),
            tool_calls: Vec::new(),
        }
    }

    /// Build an assistant message.
    #[must_use]
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: content.into(),
            timestamp: Utc::now(),
            tool_calls: Vec::new(),
        }
    }

    /// Attach tool calls — builder helper.
    #[must_use]
    pub fn with_tool_calls(mut self, calls: Vec<ToolInvocation>) -> Self {
        self.tool_calls = calls;
        self
    }
}

// ---------------------------------------------------------------------------
// ToolInvocation — one assistant→tool dispatch unit
// ---------------------------------------------------------------------------

/// One tool the assistant wants the orchestrator to invoke on its behalf.
///
/// Mirrors [`aphrody_session::ToolCall`] plus the call-time outcome fields
/// (`result` / `error`). The orchestrator constructs a [`ToolInvocation`]
/// per tool call emitted by [`crate::backend::BackendResponse::tool_calls`],
/// dispatches it, and surfaces the populated record on the returned
/// [`Turn::tool_invocations`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ToolInvocation {
    /// Stable identifier — copied from the backend's `tool_use.id` when
    /// available, otherwise generated as a UUID v4.
    pub id: String,
    /// Tool name. Must match a descriptor registered on the orchestrator's
    /// [`aphrody_tools::ToolRegistry`].
    pub name: String,
    /// JSON arguments object handed to the tool.
    pub arguments: serde_json::Value,
    /// JSON result on success. `None` if the call has not yet returned or if
    /// the call ended in `error`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Human-readable failure message. Mutually exclusive with `result` on a
    /// completed call.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Wall-clock duration of the dispatch, in milliseconds. Populated by
    /// the orchestrator once the call has completed.
    #[serde(default)]
    pub duration_ms: u64,
}

impl ToolInvocation {
    /// Build a new pending invocation. `id` is auto-generated as a UUID v4 when
    /// the supplied string is empty.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        arguments: serde_json::Value,
    ) -> Self {
        let raw_id = id.into();
        let final_id = if raw_id.is_empty() {
            uuid::Uuid::new_v4().to_string()
        } else {
            raw_id
        };
        Self {
            id: final_id,
            name: name.into(),
            arguments,
            result: None,
            error: None,
            duration_ms: 0,
        }
    }
}

// ---------------------------------------------------------------------------
// Turn — what `single_turn` returns
// ---------------------------------------------------------------------------

/// Outcome of a single turn through the orchestrator.
///
/// Returned by [`crate::turn_loop::ChatLoop::single_turn`]. Holds enough
/// information for a CLI to render the response, account for cost, and
/// surface tool-call diagnostics — without leaking the underlying provider's
/// native wire shape.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Turn {
    /// The raw user input that drove this turn.
    pub user_input: String,
    /// The assistant's textual reply (collapsed if the backend streamed).
    pub assistant_response: String,
    /// Every tool the assistant invoked during this turn, including their
    /// results / errors and per-call duration.
    #[serde(default)]
    pub tool_invocations: Vec<ToolInvocation>,
    /// Total USD cost computed from the backend-reported token usage and the
    /// [`aphrody_cost::PricingTable`] attached to the orchestrator.
    #[serde(default)]
    pub cost_usd: f64,
    /// Prompt-side tokens billed (mirrors [`aphrody_router::Usage::prompt_tokens`]).
    #[serde(default)]
    pub prompt_tokens: u32,
    /// Completion-side tokens billed.
    #[serde(default)]
    pub completion_tokens: u32,
    /// Wall-clock duration of the entire turn, in milliseconds.
    #[serde(default)]
    pub duration_ms: u64,
}

// ---------------------------------------------------------------------------
// ChatConfig — orchestrator construction parameters
// ---------------------------------------------------------------------------

/// Minimal default system prompt: keeps the agent in **goal mode** — it treats
/// every request as an objective to complete fully and autonomously, decides
/// and acts without asking, verifies its own work, and does not consider itself
/// done until the objective holds. Mirrors the `/goal` Stop-hook discipline.
///
/// Deliberately short (one tight paragraph) so it costs almost no context
/// budget; callers override it via `--system` or [`ChatConfig::system_prompt`].
pub const DEFAULT_SYSTEM_PROMPT: &str = "You are aphrody, a goal-driven CLI \
    agent. Treat every request as an objective: infer the goal, pursue it \
    end-to-end, and keep working until it is fully met — never stop half-done. \
    Decide and act autonomously instead of asking; make reasonable assumptions \
    and state them. Verify your own work before claiming success, and report \
    honestly what is done versus pending. Be concise.";

/// Knobs that govern the behaviour of a [`crate::turn_loop::ChatLoop`].
///
/// Constructed via [`ChatConfig::default`] or
/// [`crate::turn_loop::ChatLoop::builder`]. The defaults are tuned for the
/// `aphrody chat --prompt …` one-shot smoke path: SqliteLocal memory provider,
/// default-mode permissions, stub backend, no live LLM call, no session
/// persistence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatConfig {
    /// Logical owner used both for session metadata and as the
    /// [`aphrody_memory::types::MemoryQuery::agent_id`] when recalling
    /// long-term memories.
    pub agent_id: String,
    /// Vendor-qualified model identifier (`"gemini/default"`,
    /// `"anthropic/claude-opus-4-7"`, …). Parsed via
    /// [`aphrody_router::ModelId::parse`] when a router-backed backend is
    /// used; passed through as-is to the stub / Gemini-CLI subprocess
    /// backends.
    pub model: String,
    /// System prompt prepended to every conversation. Stored as a pinned
    /// `system` message in [`aphrody_context::ContextWindow`].
    pub system_prompt: String,
    /// Hard ceiling on the number of `single_turn` calls. `0` means
    /// unbounded — the orchestrator never enforces a stop on its own.
    pub max_turns: u32,
    /// Toggle that gates tool dispatch end-to-end. When `false`, any tool
    /// call emitted by the backend short-circuits to
    /// [`crate::error::ChatError::ToolDenied`] without consulting the
    /// permission engine.
    pub tools_enabled: bool,
    /// Soft ceiling on context-window utilisation
    /// ([`aphrody_context::ContextWindow::utilization`]). When exceeded, the
    /// orchestrator publishes a `chat.context.over_budget` event so callers
    /// can trigger summarisation. Default `0.85`.
    pub context_max_utilisation: f32,
    /// Maximum context window tokens.
    pub max_context_tokens: u32,
    /// Optional absolute path under which the session log is persisted as
    /// `<agent_id>/<session_id>.jsonl`. `None` skips disk persistence.
    pub session_dir: Option<std::path::PathBuf>,
}

impl Default for ChatConfig {
    fn default() -> Self {
        Self {
            agent_id: "aphrody-default".to_string(),
            model: "gemini/default".to_string(),
            system_prompt: DEFAULT_SYSTEM_PROMPT.to_string(),
            max_turns: 0,
            tools_enabled: true,
            context_max_utilisation: 0.85,
            max_context_tokens: 32_000,
            session_dir: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_round_trips_via_router_session_context() {
        for r in [
            MessageRole::System,
            MessageRole::User,
            MessageRole::Assistant,
            MessageRole::Tool,
        ] {
            // Pure conversion sanity — the projections must match the
            // `as_str()` token from each downstream crate.
            assert_eq!(r.to_router(), match r {
                MessageRole::System => aphrody_router::Role::System,
                MessageRole::User => aphrody_router::Role::User,
                MessageRole::Assistant => aphrody_router::Role::Assistant,
                MessageRole::Tool => aphrody_router::Role::Tool,
            });
            assert_eq!(r.to_session(), match r {
                MessageRole::System => aphrody_session::TurnRole::System,
                MessageRole::User => aphrody_session::TurnRole::User,
                MessageRole::Assistant => aphrody_session::TurnRole::Assistant,
                MessageRole::Tool => aphrody_session::TurnRole::Tool,
            });
            assert_eq!(r.to_context(), match r {
                MessageRole::System => aphrody_context::MessageRole::System,
                MessageRole::User => aphrody_context::MessageRole::User,
                MessageRole::Assistant => aphrody_context::MessageRole::Assistant,
                MessageRole::Tool => aphrody_context::MessageRole::Tool,
            });
        }
    }

    #[test]
    fn message_user_helper_sets_role_and_timestamp() {
        let m = Message::user("hello");
        assert_eq!(m.role, MessageRole::User);
        assert_eq!(m.content, "hello");
        assert!(m.tool_calls.is_empty());
    }

    #[test]
    fn tool_invocation_generates_uuid_when_id_empty() {
        let inv = ToolInvocation::new("", "Bash", serde_json::json!({"cmd": "ls"}));
        assert!(!inv.id.is_empty(), "uuid should be generated");
        assert_eq!(inv.name, "Bash");
        assert!(inv.result.is_none());
        assert!(inv.error.is_none());
    }

    #[test]
    fn default_config_has_safe_defaults() {
        let c = ChatConfig::default();
        assert_eq!(c.model, "gemini/default");
        assert!(c.tools_enabled);
        assert_eq!(c.max_turns, 0);
        assert!(c.context_max_utilisation > 0.0 && c.context_max_utilisation <= 1.0);
        assert!(c.session_dir.is_none());
        assert!(!c.system_prompt.is_empty());
    }

    #[test]
    fn default_system_prompt_is_goal_mode() {
        let c = ChatConfig::default();
        assert_eq!(c.system_prompt, DEFAULT_SYSTEM_PROMPT);
        // Goal-mode invariants: pursue end-to-end, act autonomously, verify.
        let p = c.system_prompt.to_lowercase();
        assert!(p.contains("goal"), "must frame work as a goal");
        assert!(p.contains("autonomously"), "must push autonomy");
        assert!(
            p.contains("until it is fully met") || p.contains("never stop"),
            "must push relentless completion"
        );
        // "Minimal": one tight paragraph, not a wall of text.
        assert!(c.system_prompt.len() < 600, "default prompt must stay minimal");
    }
}
