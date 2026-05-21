// SPDX-License-Identifier: Apache-2.0
//! The composed turn-loop orchestrator.
//!
//! [`ChatLoop`] is the public composition root that wires every aphrody
//! building block together into a single `single_turn` method. The struct
//! itself is intentionally **inert** — it holds owned handles to each
//! subsystem but never blocks on construction. All side effects happen
//! inside `single_turn`.
//!
//! ## Construction
//!
//! Use [`ChatLoop::builder`] to thread custom subsystems through:
//!
//! ```
//! use aphrody_chat::{ChatConfig, ChatLoop, backend::StubBackend};
//!
//! # fn main() -> Result<(), aphrody_chat::ChatError> {
//! let chat = ChatLoop::builder(ChatConfig::default())
//!     .with_backend(StubBackend::with_reply("OK"))
//!     .build()?;
//! drop(chat);
//! # Ok(()) }
//! ```
//!
//! Every subsystem has a sensible default so the minimal construction above
//! is enough to drive `single_turn` end-to-end (with the stub backend). All
//! handles can be supplied or replaced through dedicated builder methods —
//! the orchestrator never reaches into globals.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use aphrody_context::{ContextMessage, ContextWindow, HeuristicTokenEstimator};
use aphrody_events::EventBus;
use aphrody_skills::hooks::{HookConfig, HookDispatcher, HookEvent, HookOutcome};
use aphrody_memory::provider::MemoryProvider;
use aphrody_memory::sqlite_local::SqliteLocalProvider;
use aphrody_memory::types::MemoryQuery;
use aphrody_skills::permissions::{PermissionConfig, PermissionDecision, PermissionEngine};
use aphrody_prompts::PromptRegistry;
use aphrody_session::{
    JsonlFileStore, Session, SessionEvent, SessionId, SessionStore, ToolCall as SessionToolCall,
    Turn as SessionTurn,
};
use aphrody_tools::ToolRegistry;

use crate::backend::{BackendResponse, ModelBackend, StubBackend};
use crate::error::ChatError;
use crate::types::{ChatConfig, Message, ToolInvocation, Turn};

// ---------------------------------------------------------------------------
// ChatLoopBuilder — fluent configuration
// ---------------------------------------------------------------------------

/// Fluent builder for [`ChatLoop`].
///
/// Every dependency has a sensible default that keeps the chat loop callable
/// without a live LLM (stub backend, in-process SqliteLocal memory at
/// `:memory:`, empty tool registry, default-mode permission engine, empty
/// hook config, ad-hoc prompt registry, fresh event bus, 32 k-token context
/// window). Override any of them via the `with_*` methods before calling
/// [`Self::build`].
pub struct ChatLoopBuilder {
    config: ChatConfig,
    backend: Option<Box<dyn ModelBackend>>,
    tools: Option<ToolRegistry>,
    memory: Option<Arc<dyn MemoryProvider>>,
    permissions: Option<PermissionEngine>,
    hooks: Option<HookDispatcher>,
    prompts: Option<PromptRegistry>,
    events: Option<EventBus>,
    session_id: Option<SessionId>,
    session_store: Option<JsonlFileStore>,
}

impl ChatLoopBuilder {
    /// Start a new builder rooted at `config`.
    #[must_use]
    pub fn new(config: ChatConfig) -> Self {
        Self {
            config,
            backend: None,
            tools: None,
            memory: None,
            permissions: None,
            hooks: None,
            prompts: None,
            events: None,
            session_id: None,
            session_store: None,
        }
    }

    /// Install a concrete [`ModelBackend`] (consumed by `build`).
    #[must_use]
    pub fn with_backend<B: ModelBackend + 'static>(mut self, backend: B) -> Self {
        self.backend = Some(Box::new(backend));
        self
    }

    /// Install a fully-built boxed backend (helpful for trait-object plumbing).
    #[must_use]
    pub fn with_boxed_backend(mut self, backend: Box<dyn ModelBackend>) -> Self {
        self.backend = Some(backend);
        self
    }

    /// Replace the default empty tool registry.
    #[must_use]
    pub fn with_tools(mut self, tools: ToolRegistry) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Replace the default in-memory SqliteLocal memory provider.
    #[must_use]
    pub fn with_memory<M: MemoryProvider + 'static>(mut self, memory: M) -> Self {
        self.memory = Some(Arc::new(memory));
        self
    }

    /// Install an externally-built memory provider behind an [`Arc`] — useful
    /// when the same provider is shared across sub-agents.
    #[must_use]
    pub fn with_shared_memory(mut self, memory: Arc<dyn MemoryProvider>) -> Self {
        self.memory = Some(memory);
        self
    }

    /// Replace the default permission engine.
    #[must_use]
    pub fn with_permissions(mut self, permissions: PermissionEngine) -> Self {
        self.permissions = Some(permissions);
        self
    }

    /// Replace the default empty hook dispatcher.
    #[must_use]
    pub fn with_hooks(mut self, hooks: HookDispatcher) -> Self {
        self.hooks = Some(hooks);
        self
    }

    /// Replace the default prompt registry.
    #[must_use]
    pub fn with_prompts(mut self, prompts: PromptRegistry) -> Self {
        self.prompts = Some(prompts);
        self
    }

    /// Replace the default event bus.
    #[must_use]
    pub fn with_events(mut self, events: EventBus) -> Self {
        self.events = Some(events);
        self
    }

    /// Override the session id (otherwise auto-generated as
    /// [`SessionId::generate`]).
    #[must_use]
    pub fn with_session_id(mut self, id: SessionId) -> Self {
        self.session_id = Some(id);
        self
    }

    /// Install a custom session store. Defaults to a [`JsonlFileStore`]
    /// rooted at [`ChatConfig::session_dir`] when set, otherwise persistence
    /// is skipped (the orchestrator keeps the session in memory only).
    #[must_use]
    pub fn with_session_store(mut self, store: JsonlFileStore) -> Self {
        self.session_store = Some(store);
        self
    }

    /// Finalise the construction.
    ///
    /// # Errors
    ///
    /// * [`ChatError::InvalidConfig`] when [`ChatConfig::max_context_tokens`]
    ///   is zero (the context window manager rejects a zero budget).
    /// * [`ChatError::MemoryFailure`] when the default SqliteLocal provider
    ///   cannot open its in-memory database.
    pub fn build(self) -> Result<ChatLoop, ChatError> {
        if self.config.max_context_tokens == 0 {
            return Err(ChatError::InvalidConfig(
                "max_context_tokens must be > 0".to_string(),
            ));
        }
        if self.config.context_max_utilisation <= 0.0
            || self.config.context_max_utilisation > 1.0
            || self.config.context_max_utilisation.is_nan()
        {
            return Err(ChatError::InvalidConfig(
                "context_max_utilisation must be in (0.0, 1.0]".to_string(),
            ));
        }

        let backend: Box<dyn ModelBackend> =
            self.backend.unwrap_or_else(|| Box::new(StubBackend::default()));

        let tools = self.tools.unwrap_or_default();

        let memory: Arc<dyn MemoryProvider> = if let Some(m) = self.memory {
            m
        } else {
            // Use rusqlite's `:memory:` shortcut via the dedicated helper — it
            // avoids the `SQLITE_OPEN_CREATE` flag dance that the on-disk
            // constructor performs and never touches the filesystem.
            let sqlite = SqliteLocalProvider::open_in_memory()
                .map_err(|e| ChatError::MemoryFailure(e.to_string()))?;
            Arc::new(sqlite)
        };

        let permissions = if let Some(p) = self.permissions {
            p
        } else {
            PermissionEngine::from_config(PermissionConfig::default())
                .map_err(|e| ChatError::InvalidConfig(format!("permissions: {e}")))?
        };

        let hooks = if let Some(h) = self.hooks {
            h
        } else {
            HookDispatcher::new(HookConfig::default())?
        };

        let prompts = self.prompts.unwrap_or_default();
        let events = self.events.unwrap_or_else(|| EventBus::new(0));

        let session_id = self.session_id.unwrap_or_else(SessionId::generate);
        let session_metadata = serde_json::json!({
            "agent_id": self.config.agent_id,
            "model": self.config.model,
        });
        let mut session = Session::new(session_id, session_metadata);

        // Seed the session with the system prompt as a synthetic system turn
        // so the JSONL transcript is self-describing on replay.
        if !self.config.system_prompt.is_empty() {
            session.add_turn(SessionTurn {
                id: "system-0".to_string(),
                role: aphrody_session::TurnRole::System,
                content: self.config.system_prompt.clone(),
                ts: chrono::Utc::now(),
                prompt_tokens: 0,
                completion_tokens: 0,
                cost_usd: 0.0,
            });
        }

        let session_store = self.session_store.or_else(|| {
            self.config
                .session_dir
                .as_ref()
                .map(|dir| JsonlFileStore::new(dir.clone()))
        });

        let estimator = Arc::new(HeuristicTokenEstimator::default());
        let mut context = ContextWindow::new(self.config.max_context_tokens, estimator);
        // Pin the system prompt so eviction can never drop it.
        if !self.config.system_prompt.is_empty() {
            let id = ContextMessage::hash_id(
                aphrody_context::MessageRole::System,
                &self.config.system_prompt,
            );
            let mut msg = ContextMessage::new(
                id,
                aphrody_context::MessageRole::System,
                self.config.system_prompt.clone(),
            );
            msg.pinned = true;
            context.push(msg);
        }

        Ok(ChatLoop {
            config: self.config,
            backend,
            tools,
            memory,
            permissions,
            hooks,
            prompts,
            events,
            session,
            session_store,
            context,
            turns_executed: 0,
        })
    }
}

// ---------------------------------------------------------------------------
// ChatLoop — the orchestrator
// ---------------------------------------------------------------------------

/// Composed turn-loop orchestrator.
///
/// One [`ChatLoop`] owns the running [`Session`], the live
/// [`ContextWindow`], a boxed [`ModelBackend`], plus shared handles to every
/// subsystem ([`ToolRegistry`], [`MemoryProvider`], [`PermissionEngine`],
/// [`HookDispatcher`], [`PromptRegistry`], [`EventBus`], optional
/// [`JsonlFileStore`]).
///
/// Use [`ChatLoop::single_turn`] to drive one user→assistant exchange. The
/// orchestrator is `!Sync` because it holds a mutable [`Session`] — wrap it
/// in `tokio::sync::Mutex` if you need cross-task driver access.
pub struct ChatLoop {
    config: ChatConfig,
    backend: Box<dyn ModelBackend>,
    tools: ToolRegistry,
    memory: Arc<dyn MemoryProvider>,
    permissions: PermissionEngine,
    hooks: HookDispatcher,
    #[allow(dead_code)] // wired into single_turn via render_system_prompt.
    prompts: PromptRegistry,
    events: EventBus,
    session: Session,
    session_store: Option<JsonlFileStore>,
    context: ContextWindow,
    turns_executed: u32,
}

impl std::fmt::Debug for ChatLoop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChatLoop")
            .field("agent_id", &self.config.agent_id)
            .field("model", &self.config.model)
            .field("backend", &self.backend.name())
            .field("registered_tools", &self.tools.len())
            .field("turns_executed", &self.turns_executed)
            .field("session_id", &self.session.id.to_string())
            .finish()
    }
}

impl ChatLoop {
    /// Start a fluent builder.
    #[must_use]
    pub fn builder(config: ChatConfig) -> ChatLoopBuilder {
        ChatLoopBuilder::new(config)
    }

    /// Convenience: build a [`ChatLoop`] from a [`ChatConfig`] using all
    /// defaults (stub backend, in-memory SqliteLocal, etc.). Equivalent to
    /// `ChatLoop::builder(config).build()`.
    ///
    /// # Errors
    ///
    /// Same surface as [`ChatLoopBuilder::build`].
    pub fn from_config(config: ChatConfig) -> Result<Self, ChatError> {
        Self::builder(config).build()
    }

    /// Borrow the active configuration.
    #[must_use]
    pub fn config(&self) -> &ChatConfig {
        &self.config
    }

    /// Borrow the underlying session — useful for tests and for surfacing the
    /// JSONL log to the CLI / UI without persisting it.
    #[must_use]
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// Borrow the live context window.
    #[must_use]
    pub fn context(&self) -> &ContextWindow {
        &self.context
    }

    /// How many `single_turn` calls have completed successfully so far.
    #[must_use]
    pub fn turns_executed(&self) -> u32 {
        self.turns_executed
    }

    /// Borrow the tool registry — read-only access so callers can introspect
    /// the wired surface (e.g. for diagnostic `/tools` REPL commands).
    #[must_use]
    pub fn tools(&self) -> &ToolRegistry {
        &self.tools
    }

    /// Borrow the event bus — useful for downstream subscribers (Discord
    /// notifier, WebSocket fan-out, NDJSON sink, …).
    #[must_use]
    pub fn events(&self) -> &EventBus {
        &self.events
    }

    /// Drive one full user→assistant exchange.
    ///
    /// The sequence is documented in the crate-level docs (`lib.rs`).
    /// Returns a populated [`Turn`] on success; every recoverable failure
    /// surfaces through [`ChatError`].
    ///
    /// # Errors
    ///
    /// * [`ChatError::BackendFailure`] / [`ChatError::ToolDenied`] /
    ///   [`ChatError::ToolBlockedByHook`] / [`ChatError::ToolNotRegistered`] /
    ///   [`ChatError::ToolArgsInvalid`] — surfaced from the relevant
    ///   subsystem.
    /// * [`ChatError::SessionFailure`] when an optional session persistence
    ///   round-trip fails.
    /// * [`ChatError::InvalidConfig`] when [`ChatConfig::max_turns`] has
    ///   been exhausted.
    pub async fn single_turn(&mut self, user_input: &str) -> Result<Turn, ChatError> {
        if self.config.max_turns > 0 && self.turns_executed >= self.config.max_turns {
            return Err(ChatError::InvalidConfig(format!(
                "max_turns exhausted: {}",
                self.config.max_turns
            )));
        }

        let start = Instant::now();

        // ─── 1. UserPromptSubmit hook ────────────────────────────────────
        let prompt_event = HookEvent::UserPromptSubmit { prompt: user_input.to_string() };
        let prompt_outcomes = self.hooks.run_hooks(&prompt_event).await?;
        for outcome in &prompt_outcomes {
            if let HookOutcome::Block { reason } = &outcome.outcome {
                return Err(ChatError::ToolBlockedByHook {
                    tool: "<UserPromptSubmit>".to_string(),
                    reason: reason.clone(),
                });
            }
        }

        // ─── 2. Append user turn to the session log ─────────────────────
        let user_turn_id = format!("u{}", self.session.events.len());
        self.session.add_turn(SessionTurn {
            id: user_turn_id.clone(),
            role: aphrody_session::TurnRole::User,
            content: user_input.to_string(),
            ts: chrono::Utc::now(),
            prompt_tokens: 0,
            completion_tokens: 0,
            cost_usd: 0.0,
        });

        // ─── 3. Push user message into the live context window ──────────
        let user_ctx_id = ContextMessage::hash_id(
            aphrody_context::MessageRole::User,
            user_input,
        );
        self.context.push(ContextMessage::new(
            user_ctx_id,
            aphrody_context::MessageRole::User,
            user_input.to_string(),
        ));

        // ─── 4. Recall relevant memories (best-effort, never aborts turn) ─
        let mut recalled: Vec<aphrody_memory::types::MemoryRecord> = Vec::new();
        let recall_query = MemoryQuery::for_agent(self.config.agent_id.clone());
        match self.memory.search(recall_query).await {
            Ok(records) => recalled = records,
            Err(e) => {
                // Memory failure should never abort a chat turn — it degrades
                // gracefully to "no recall" and surfaces a tracing diagnostic.
                tracing::warn!(
                    target: "aphrody_chat::turn_loop",
                    error = %e,
                    "memory recall failed; continuing without recalled context",
                );
            },
        }

        // ─── 5. Build the message list handed to the backend ────────────
        let mut messages: Vec<Message> = Vec::with_capacity(8 + recalled.len());
        if !self.config.system_prompt.is_empty() {
            messages.push(Message::system(self.config.system_prompt.clone()));
        }
        for r in &recalled {
            messages.push(Message::system(format!("[recall] {}", r.content)));
        }
        messages.push(Message::user(user_input.to_string()));

        // ─── 6. Drive the backend ───────────────────────────────────────
        let backend_response: BackendResponse = self.backend.complete(&messages).await?;

        // ─── 7-9. Process tool calls (permission → pre-hook → dispatch → post-hook) ─
        let mut completed_invocations: Vec<ToolInvocation> = Vec::new();
        if !backend_response.tool_calls.is_empty() {
            if !self.config.tools_enabled {
                return Err(ChatError::ToolDenied {
                    tool: backend_response.tool_calls[0].name.clone(),
                    reason: "tools are disabled in ChatConfig".to_string(),
                });
            }
            for mut invocation in backend_response.tool_calls.into_iter() {
                let call_start = Instant::now();
                let descriptor = self
                    .tools
                    .get(&invocation.name)
                    .ok_or_else(|| ChatError::ToolNotRegistered(invocation.name.clone()))?;

                // Validate args against the registered schema.
                if let Err(e) = self
                    .tools
                    .validate_args(&invocation.name, &invocation.arguments)
                {
                    return Err(ChatError::ToolArgsInvalid {
                        tool: invocation.name.clone(),
                        source: e,
                    });
                }

                // Permission gate.
                let decision = self
                    .permissions
                    .evaluate(&invocation.name, &invocation.arguments);
                if let PermissionDecision::Deny { reason } = &decision {
                    return Err(ChatError::ToolDenied {
                        tool: invocation.name.clone(),
                        reason: reason.clone(),
                    });
                }

                // PreToolUse hook.
                let pre_event = HookEvent::PreToolUse {
                    tool_name: invocation.name.clone(),
                    tool_input: invocation.arguments.clone(),
                };
                let pre_outcomes = self.hooks.run_hooks(&pre_event).await?;
                for outcome in &pre_outcomes {
                    if let HookOutcome::Block { reason } = &outcome.outcome {
                        return Err(ChatError::ToolBlockedByHook {
                            tool: invocation.name.clone(),
                            reason: reason.clone(),
                        });
                    }
                }

                // ─── 9. Dispatch the tool ───────────────────────────────
                // The vendor-neutral ToolRegistry only stores descriptors,
                // not handlers. We surface a structured result that records
                // the call as "dispatched" (a real handler registry will
                // land in a follow-up sprint — see crate-level NotYetImplemented).
                invocation.result = Some(serde_json::json!({
                    "status": "dispatched",
                    "descriptor": {
                        "name": descriptor.name,
                        "dangerous": descriptor.dangerous,
                    },
                }));
                invocation.duration_ms = u64::try_from(
                    call_start.elapsed().as_millis(),
                )
                .unwrap_or(u64::MAX);

                // PostToolUse hook.
                let post_event = HookEvent::PostToolUse {
                    tool_name: invocation.name.clone(),
                    tool_input: invocation.arguments.clone(),
                    tool_response: invocation
                        .result
                        .clone()
                        .unwrap_or(serde_json::Value::Null),
                };
                let _ = self.hooks.run_hooks(&post_event).await?;

                // Mirror the call into the session JSONL log.
                self.session.add_tool_call(SessionToolCall {
                    id: invocation.id.clone(),
                    name: invocation.name.clone(),
                    args: invocation.arguments.clone(),
                    result: invocation.result.clone(),
                    error: invocation.error.clone(),
                    ts: chrono::Utc::now(),
                    duration_ms: invocation.duration_ms,
                });

                completed_invocations.push(invocation);
            }
        }

        // ─── 10. Append the assistant turn to the session + context ─────
        let cost_usd = 0.0_f64; // CostCalculator wiring is opt-in (TODO Phase 2).
        let assistant_turn_id = format!("a{}", self.session.events.len());
        self.session.add_turn(SessionTurn {
            id: assistant_turn_id,
            role: aphrody_session::TurnRole::Assistant,
            content: backend_response.content.clone(),
            ts: chrono::Utc::now(),
            prompt_tokens: backend_response.prompt_tokens,
            completion_tokens: backend_response.completion_tokens,
            cost_usd,
        });

        let assistant_ctx_id = ContextMessage::hash_id(
            aphrody_context::MessageRole::Assistant,
            &backend_response.content,
        );
        self.context.push(ContextMessage::new(
            assistant_ctx_id,
            aphrody_context::MessageRole::Assistant,
            backend_response.content.clone(),
        ));

        // ─── 11. Emit lifecycle events on the bus ───────────────────────
        let payload = serde_json::json!({
            "agent_id": self.config.agent_id,
            "session_id": self.session.id.to_string(),
            "backend": self.backend.name(),
            "tool_count": completed_invocations.len(),
            "prompt_tokens": backend_response.prompt_tokens,
            "completion_tokens": backend_response.completion_tokens,
        });
        // Best-effort publish; an unsubscribed bus is fine.
        let _ = self
            .events
            .publish(aphrody_events::Event::new(
                "chat.turn.done",
                aphrody_events::EventLevel::Info,
                "aphrody_chat",
                payload,
            ))
            .await;

        if self.context.should_summarize() {
            let payload = serde_json::json!({
                "utilisation": self.context.utilization(),
                "threshold": self.config.context_max_utilisation,
            });
            let _ = self
                .events
                .publish(aphrody_events::Event::new(
                    "chat.context.over_budget",
                    aphrody_events::EventLevel::Warn,
                    "aphrody_chat",
                    payload,
                ))
                .await;
        }

        // ─── 12. Optional persist ───────────────────────────────────────
        if let Some(store) = self.session_store.as_ref() {
            store.save_session(&self.session).await?;
        }

        // ─── 13. Stop hook (terminal turn marker) ───────────────────────
        let _ = self.hooks.run_hooks(&HookEvent::Stop).await?;

        self.turns_executed = self.turns_executed.saturating_add(1);

        let duration_ms = u64::try_from(start.elapsed().as_millis()).unwrap_or(u64::MAX);
        Ok(Turn {
            user_input: user_input.to_string(),
            assistant_response: backend_response.content,
            tool_invocations: completed_invocations,
            cost_usd,
            prompt_tokens: backend_response.prompt_tokens,
            completion_tokens: backend_response.completion_tokens,
            duration_ms,
        })
    }

    /// Persist the running session to disk, regardless of whether
    /// [`ChatConfig::session_dir`] was set. Useful for explicit `/save`
    /// REPL commands.
    ///
    /// # Errors
    ///
    /// [`ChatError::SessionFailure`] when the store's atomic write fails.
    pub async fn persist_session(&self, dir: PathBuf) -> Result<(), ChatError> {
        let store = JsonlFileStore::new(dir);
        store.save_session(&self.session).await?;
        Ok(())
    }

    /// Close out the session by appending a `SessionEnd` event with the
    /// rolled-up totals. The loop is still usable afterwards (a new
    /// `SessionEnd` can be appended in the future); the explicit terminator
    /// is intentional so callers can mark milestones inside a long-running
    /// session.
    pub fn end(&mut self) {
        self.session.end();
    }

    /// Count of recorded events on the running session.
    #[must_use]
    pub fn session_event_count(&self) -> usize {
        self.session.events.len()
    }

    /// Count of `Turn` events recorded so far.
    #[must_use]
    pub fn recorded_turns(&self) -> usize {
        self.session
            .events
            .iter()
            .filter(|e| matches!(e, SessionEvent::Turn(_)))
            .count()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::StubBackend;

    #[tokio::test]
    async fn builder_with_zero_context_budget_fails() {
        let mut cfg = ChatConfig::default();
        cfg.max_context_tokens = 0;
        let err = ChatLoop::builder(cfg).build().unwrap_err();
        match err {
            ChatError::InvalidConfig(msg) => assert!(msg.contains("max_context_tokens")),
            other => panic!("expected InvalidConfig, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn builder_with_invalid_utilisation_fails() {
        let mut cfg = ChatConfig::default();
        cfg.context_max_utilisation = 1.5; // > 1.0
        let err = ChatLoop::builder(cfg).build().unwrap_err();
        match err {
            ChatError::InvalidConfig(msg) => {
                assert!(msg.contains("context_max_utilisation"));
            },
            other => panic!("expected InvalidConfig, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn build_with_defaults_succeeds() {
        let cfg = ChatConfig::default();
        let chat = ChatLoop::builder(cfg).with_backend(StubBackend::default()).build();
        assert!(chat.is_ok());
        let c = chat.unwrap();
        assert_eq!(c.turns_executed(), 0);
        // System prompt should have been recorded.
        assert!(c.session_event_count() >= 1);
    }
}
