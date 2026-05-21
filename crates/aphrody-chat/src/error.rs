// SPDX-License-Identifier: Apache-2.0
//! Unified [`ChatError`] envelope.
//!
//! Every recoverable failure produced inside [`crate::turn_loop`] or
//! [`crate::backend`] maps onto a variant here. Transparent `#[from]`
//! conversions are used for the major upstream error types so call sites can
//! propagate with `?` without a manual `map_err` ladder.

use thiserror::Error;

/// Recoverable failure surface for the chat orchestrator.
///
/// The shape mirrors the conventions used by sibling crates (see
/// `aphrody-session::SessionError`, `aphrody-router::RouterError`,
/// `aphrody-hooks::HookError`): an opaque transparent IO/JSON forward plus
/// named variants for everything domain-specific.
#[derive(Debug, Error)]
pub enum ChatError {
    /// The configured backend returned a transport / API / decode error.
    /// The inner string carries a human-readable rendering of the underlying
    /// error so call sites don't need to depend on every backend's native
    /// error type.
    #[error("backend failure: {0}")]
    BackendFailure(String),

    /// The model emitted a tool call but the registered [`aphrody_tools::ToolRegistry`]
    /// has no matching descriptor. Typically a hallucinated function name.
    #[error("tool not registered: {0}")]
    ToolNotRegistered(String),

    /// The model emitted a tool call whose args failed JSON-Schema validation
    /// against the registered descriptor.
    #[error("tool args invalid for {tool}: {source}")]
    ToolArgsInvalid {
        /// The offending tool name.
        tool: String,
        /// The validation error lifted from [`aphrody_tools::ToolsError`].
        #[source]
        source: aphrody_tools::ToolsError,
    },

    /// The permission engine refused to run the tool. `reason` is the engine's
    /// own rationale string.
    #[error("tool denied by permission engine: {tool} — {reason}")]
    ToolDenied {
        /// Tool name that was rejected.
        tool: String,
        /// Human-readable rationale lifted from the [`aphrody_skills::permissions::PermissionDecision`].
        reason: String,
    },

    /// A registered pre-tool hook returned a `block` decision.
    #[error("tool blocked by hook: {tool} — {reason}")]
    ToolBlockedByHook {
        /// Tool name that the hook blocked.
        tool: String,
        /// Reason string lifted from the hook's `{"decision":"block","reason":...}` JSON.
        reason: String,
    },

    /// The rate guard refused to release a token within the configured wait
    /// ceiling.
    #[error("rate-limited: wait_ms={wait_ms}")]
    RateLimited {
        /// Suggested wait time (in milliseconds) reported by the bucket.
        wait_ms: u64,
    },

    /// The cost calculator detected that the call would breach the configured
    /// budget guard.
    #[error("budget exceeded: limit={limit_usd} attempted={attempted_usd}")]
    BudgetExceeded {
        /// USD ceiling configured on the [`aphrody_cost::BudgetGuard`].
        limit_usd: f64,
        /// Total spend that would have resulted from the rejected charge.
        attempted_usd: f64,
    },

    /// The configured memory provider failed (Mem0/Honcho/SqliteLocal).
    #[error("memory provider failure: {0}")]
    MemoryFailure(String),

    /// The configured session store failed (load/save/list/delete).
    #[error("session store failure: {0}")]
    SessionFailure(#[from] aphrody_session::SessionError),

    /// A prompt template failed to render (missing variable, syntax error).
    #[error("prompt render failure: {0}")]
    PromptFailure(#[from] aphrody_prompts::PromptError),

    /// The hook dispatcher failed to spawn or run a hook command.
    #[error("hook failure: {0}")]
    HookFailure(#[from] aphrody_skills::hooks::HookError),

    /// Filesystem I/O error.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON (de)serialization failed.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// The orchestrator was asked to use a feature that is wired through the
    /// type-system but not yet hooked up end-to-end. The variant exists so we
    /// can return a structured error instead of `todo!()` / `unimplemented!()`
    /// — both of which would (a) panic at runtime and (b) trip the
    /// workspace-wide clippy `deny` for `todo` / `unimplemented`.
    #[error("feature not yet wired end-to-end: {feature}")]
    NotYetImplemented {
        /// Free-form identifier of the missing capability so test logs and
        /// downstream UIs can surface a precise diagnostic.
        feature: &'static str,
    },

    /// A configuration value was missing or malformed at chat-loop construction
    /// time.
    #[error("invalid configuration: {0}")]
    InvalidConfig(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_includes_variant_payload() {
        let err = ChatError::ToolNotRegistered("Banana".into());
        let rendered = err.to_string();
        assert!(rendered.contains("Banana"));
    }

    #[test]
    fn not_yet_implemented_carries_feature_tag() {
        let err = ChatError::NotYetImplemented { feature: "antigravity_backend" };
        let rendered = err.to_string();
        assert!(rendered.contains("antigravity_backend"));
    }

    #[test]
    fn budget_exceeded_includes_numbers() {
        let err = ChatError::BudgetExceeded { limit_usd: 1.0, attempted_usd: 1.5 };
        let s = err.to_string();
        assert!(s.contains("1") && s.contains("1.5"));
    }
}
