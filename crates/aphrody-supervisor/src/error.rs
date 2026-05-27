// SPDX-License-Identifier: Apache-2.0
//! Errors surfaced by the [`Supervisor`](crate::Supervisor) control plane.

use crate::AgentId;

/// Errors produced while spawning, routing to, or tearing down agents.
#[derive(Debug, thiserror::Error)]
pub enum SupervisorError {
    /// [`spawn_agent`](crate::Supervisor::spawn_agent) was called with an id
    /// that is already tracked. Ids must be unique for the supervisor's
    /// lifetime (until the agent is shut down and removed).
    #[error("an agent named `{0}` is already registered")]
    DuplicateId(AgentId),

    /// A routing or lifecycle call named an agent the supervisor does not
    /// track (never spawned, or already shut down and removed).
    #[error("no agent named `{0}` is registered")]
    UnknownAgent(AgentId),

    /// The engine refused a submission: the underlying session actor has
    /// already stopped (its submission channel is closed), so the op could not
    /// be delivered.
    #[error("engine error for agent `{id}`: {message}")]
    Engine {
        /// The agent whose session rejected the submission.
        id: AgentId,
        /// Human-readable description of the failure.
        message: String,
    },

    /// The shared fan-in event channel has been closed (for example, the
    /// receiver returned by [`take_events`](crate::Supervisor::take_events) was
    /// dropped), so per-agent events can no longer be forwarded.
    #[error("the fan-in event channel is closed")]
    ChannelClosed,
}
