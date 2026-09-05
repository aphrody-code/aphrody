// SPDX-License-Identifier: Apache-2.0
//! Engine error type.

use aphrody_model_client::ModelError;
use aphrody_rollout::RolloutError;
use aphrody_toolcall::ToolError;

/// Errors that can abort a [`run_turn`](crate::run_turn) call.
#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    /// The underlying [`ModelClient`](aphrody_model_client::ModelClient) failed
    /// while opening or draining the stream.
    #[error("model error: {0}")]
    Model(#[from] ModelError),

    /// A tool registry / executor surfaced a hard error (distinct from a tool
    /// returning a soft [`ToolOutput::error`](aphrody_toolcall::ToolOutput::error),
    /// which is fed back to the model instead of aborting the turn).
    #[error("tool error: {0}")]
    Tool(#[from] ToolError),

    /// Recording an event to the rollout failed.
    #[error("rollout error: {0}")]
    Rollout(#[from] RolloutError),

    /// The turn was aborted by an approval `Abort` decision.
    #[error("turn aborted by approval decision")]
    Aborted,

    /// The approval channel closed while the engine was waiting for a decision
    /// in [`AutonomyMode::Gated`](crate::AutonomyMode::Gated).
    #[error("approval channel closed before a decision arrived")]
    ApprovalChannelClosed,
}
