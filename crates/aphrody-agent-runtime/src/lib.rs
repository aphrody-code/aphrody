// SPDX-License-Identifier: Apache-2.0
//! `aphrody-agent-runtime` — the wiring/factory layer for aphrody agents.
//!
//! This crate is Phase 3 of aphrody's Rust rewrite of Antigravity (see
//! `docs/VISION.md`). Where [`aphrody_engine`] owns the turn loop and the
//! lower crates own the model client, the tool catalogue, and the rollout
//! recorder, this crate is the ergonomic *assembly* layer: it takes a
//! [`RuntimeConfig`] plus a [`ModelChoice`] and produces a ready-to-run agent.
//!
//! It is the single entry point every aphrody surface (CLI / TUI / desktop /
//! MCP) uses to start an autonomous agent:
//!
//! - [`AgentRuntime::run_once`] drives one full headless turn end to end,
//!   draining every [`Event`](aphrody_agent_proto::Event) and returning a
//!   [`RunResult`].
//! - [`AgentRuntime::spawn`] hands back a long-lived
//!   [`SessionHandle`](aphrody_engine::SessionHandle) for interactive surfaces.
//!
//! # Autonomy
//!
//! Per the repo's no-human-in-the-loop policy (CLAUDE.md §0.1) the default
//! autonomy is [`AutonomyMode::FullAuto`](aphrody_engine::AutonomyMode::FullAuto):
//! every tool call is executed immediately, no approval round trips.
//!
//! # Offline testing
//!
//! [`StubModelClient`] implements [`ModelClient`](aphrody_model_client::ModelClient)
//! by replaying a scripted sequence of
//! [`ModelStreamEvent`](aphrody_model_client::ModelStreamEvent)s. It needs no
//! network and no TLS provider, so surfaces (and this crate's own tests) can
//! exercise the full multi-turn tool loop deterministically without ever
//! constructing a `reqwest::Client` (which would panic under rustls 0.23 in a
//! bare test process — CLAUDE.md §7).
//!
//! # Example
//!
//! ```no_run
//! # use aphrody_agent_runtime::{AgentRuntime, ModelChoice, RuntimeConfig};
//! # async fn demo() -> Result<(), aphrody_agent_runtime::RuntimeError> {
//! let runtime = AgentRuntime::builder()
//!     .config(RuntimeConfig::new("gemini-2.5-flash"))
//!     .model(ModelChoice::gemini("API_KEY", "gemini-2.5-flash"))
//!     .build()?;
//! let result = runtime.run_once("List the files in the current directory").await?;
//! if let Some(message) = result.last_agent_message {
//!     println!("{message}");
//! }
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]

mod config;
mod error;
mod model;
mod runtime;
mod tools;

pub use config::{RuntimeConfig, ShellGuardrails};
pub use error::RuntimeError;
pub use model::{ModelChoice, ScriptedTurn, StubModelClient};
pub use runtime::{AgentRuntime, AgentRuntimeBuilder, RunResult};
pub use tools::build_tool_registry;

// Re-export the engine autonomy enum so callers can configure a runtime
// without taking a separate dependency on `aphrody-engine`.
pub use aphrody_engine::AutonomyMode;

// Re-export the local-backend default base URL so CLI surfaces can resolve it
// without taking a separate dependency on `aphrody-model-client`.
pub use aphrody_model_client::DEFAULT_LOCAL_BASE_URL;

#[cfg(test)]
mod runtime_tests;
