// SPDX-License-Identifier: Apache-2.0
//! Errors surfaced by the runtime factory and its headless driver.

use aphrody_engine::EngineError;
use aphrody_rollout::RolloutError;

/// Errors produced while building or running an [`AgentRuntime`](crate::AgentRuntime).
#[derive(Debug, thiserror::Error)]
pub enum RuntimeError {
    /// The builder was missing a required field (e.g. no [`ModelChoice`](crate::ModelChoice)).
    #[error("runtime configuration error: {0}")]
    Config(String),

    /// A tool could not be registered into the [`ToolRegistry`](aphrody_toolcall::ToolRegistry).
    #[error("tool registration error: {0}")]
    ToolRegistration(#[from] aphrody_toolcall::ToolError),

    /// The underlying engine turn loop failed.
    #[error("engine error: {0}")]
    Engine(#[from] EngineError),

    /// Initializing the optional rollout recorder failed.
    #[error("rollout error: {0}")]
    Rollout(#[from] RolloutError),

    /// The spawned session actor task could not be joined (it panicked).
    #[error("session task join error: {0}")]
    Join(#[from] tokio::task::JoinError),
}
