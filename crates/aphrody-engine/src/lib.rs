// SPDX-License-Identifier: Apache-2.0
//! `aphrody-engine` — the unified aphrody agent turn loop (the engine).
//!
//! This crate is the keystone of aphrody's Rust rewrite of Antigravity (see
//! `docs/VISION.md` §2.1 and `docs/plans/gemini-codex.md`). It drives a single
//! agent *turn*: it streams a [`ModelClient`], maps the unified
//! [`ModelStreamEvent`](aphrody_model_client::ModelStreamEvent) into the SQ/EQ
//! [`EventMsg`](aphrody_agent_proto::EventMsg) protocol, executes any requested
//! tool calls through a [`ToolRegistry`](aphrody_toolcall::ToolRegistry),
//! re-injects the tool results for multi-step reasoning, and (optionally)
//! records every emitted event to a
//! [`RolloutRecorder`](aphrody_rollout::RolloutRecorder).
//!
//! The same engine is shared by every aphrody surface (CLI / TUI / desktop /
//! MCP). The protocol shape is inspired by the OpenAI Codex turn loop
//! (Apache-2.0).
//!
//! # Autonomy
//!
//! Per the repo's no-human-in-the-loop policy (CLAUDE.md §0.1) the default
//! [`AutonomyMode`] is [`AutonomyMode::FullAuto`], which auto-approves every
//! tool call. [`AutonomyMode::Gated`] makes the engine emit an
//! [`EventMsg::ExecApprovalRequest`] and wait for an
//! [`Op::ExecApproval`](aphrody_agent_proto::Op::ExecApproval) before running
//! each tool.
//!
//! # Example
//!
//! ```no_run
//! # use aphrody_engine::{EngineConfig, Session, run_turn, EventSink};
//! # use aphrody_agent_proto::InputItem;
//! # use aphrody_model_client::ModelClient;
//! # use aphrody_toolcall::ToolRegistry;
//! # async fn demo(client: &dyn ModelClient, tools: &ToolRegistry) {
//! let mut session = Session::new(EngineConfig::default());
//! let (sink, mut events) = EventSink::unbounded();
//! let input = vec![InputItem::Text { text: "hello".into() }];
//! let outcome = run_turn(&mut session, client, tools, None, input, &sink, None)
//!     .await
//!     .expect("turn");
//! while let Ok(ev) = events.try_recv() {
//!     let _ = ev;
//! }
//! let _ = outcome.last_agent_message;
//! # }
//! ```

#![forbid(unsafe_code)]

mod actor;
mod approval;
mod config;
mod error;
mod session;
mod sink;
mod turn;

pub use actor::{SessionHandle, spawn_session};
pub use approval::ApprovalGate;
pub use config::{AutonomyMode, DEFAULT_MAX_TOOL_ITERATIONS, EngineConfig};
pub use error::EngineError;
pub use session::Session;
pub use sink::EventSink;
pub use turn::{
    InterruptFlag, SteerQueue, TurnOutcome, run_turn, run_turn_with_controls,
    run_turn_with_interrupt,
};

#[cfg(test)]
mod lib_tests;
