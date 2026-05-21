// SPDX-License-Identifier: Apache-2.0
//! # aphrody-chat — unified turn-loop REPL for the aphrody agent stack.
//!
//! This crate is the **single composition root** that wires every other
//! aphrody building block into a complete chat agent equivalent to the
//! upstream `gemini` CLI. It does not invent new primitives: it threads the
//! production crates [`gemini_runtime`], [`aphrody_tools`], [`aphrody_session`],
//! [`aphrody_memory`], [`aphrody_permissions`], [`aphrody_hooks`],
//! [`aphrody_prompts`], [`aphrody_router`], [`aphrody_cost`],
//! [`aphrody_context`], [`aphrody_events`] (and friends) together into a single
//! `single_turn` orchestrator.
//!
//! ## Architecture
//!
//! ```text
//!   user input  ──► ChatLoop::single_turn ──►   1. UserPromptSubmit hook
//!                                                2. session.add_turn(user)
//!                                                3. context.push(user)
//!                                                4. memory.search(recall)
//!                                                5. prompts.render(system)
//!                                                6. backend.complete()
//!                                                7. permissions.evaluate(tool)
//!                                                8. hooks.run(PreToolUse)
//!                                                9. tools.invoke(tool)
//!                                               10. hooks.run(PostToolUse)
//!                                               11. cost.calculate(usage)
//!                                               12. session.add_turn(assistant)
//!                                               13. events.publish(turn.done)
//!                                               14. session.persist (optional)
//!                                                                └─► Turn
//! ```
//!
//! The orchestrator is **backend-agnostic**: any type implementing
//! [`backend::ModelBackend`] can drive the loop. Two production-grade
//! backends ship in this crate:
//!
//! * [`backend::GeminiBackend`] — adapter on top of [`gemini_runtime::detect`]
//!   that streams `StreamEvent`s from the local `gemini` CLI binary and
//!   collapses them into a single [`backend::BackendResponse`].
//! * [`backend::StubBackend`] — deterministic in-memory backend used for
//!   tests and for the `aphrody chat --prompt …` one-shot smoke when no live
//!   provider is configured. Returns a fixed assistant message and zero token
//!   usage, never hits the network.
//!
//! An Anthropic / Antigravity backend would also satisfy the trait by
//! delegating to [`aphrody_router::ChatProvider::complete`]; we keep that
//! adapter out of the default surface until the OAuth wiring is ready. See
//! [`backend::ModelBackend`] for the contract.
//!
//! ## Cohesion guarantees
//!
//! Every wired building block lives in its own crate. This crate adds zero
//! new dependencies that another crate already exposes — every dep declared in
//! `Cargo.toml` is a path dep to a sibling crate. The [`turn_loop::ChatLoop`]
//! struct is the *only* place where these crates are composed; downstream
//! consumers (the `aphrody chat` CLI subcommand, future REPL frontends, the
//! AG-UI bridge) should depend on this crate rather than re-implementing the
//! orchestration glue.
//!
//! ## Example
//!
//! ```
//! use aphrody_chat::{ChatConfig, ChatLoop, backend::StubBackend};
//!
//! # async fn run() -> Result<(), aphrody_chat::ChatError> {
//! let cfg = ChatConfig::default();
//! let backend = StubBackend::with_reply("OK");
//! let mut loop_ = ChatLoop::builder(cfg).with_backend(backend).build()?;
//! let turn = loop_.single_turn("hello").await?;
//! assert!(!turn.assistant_response.is_empty());
//! # Ok(()) }
//! ```

#![forbid(unsafe_code)]

pub mod backend;
pub mod error;
pub mod turn_loop;
pub mod types;

pub use crate::error::ChatError;
pub use crate::turn_loop::{ChatLoop, ChatLoopBuilder};
pub use crate::types::{ChatConfig, Message, MessageRole, ToolInvocation, Turn};

/// Crate version pulled at build time. Surfaced through the CLI `aphrody chat
/// --help` and by the `Doctor` / `Version` introspection subcommands.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Convenience result alias used pervasively across the crate.
pub type ChatResult<T> = Result<T, ChatError>;
