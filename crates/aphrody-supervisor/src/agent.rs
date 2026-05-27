// SPDX-License-Identifier: Apache-2.0
//! Agent identity ([`AgentId`]) and the public lifecycle snapshot
//! ([`AgentStatus`]) returned by [`Supervisor::list`](crate::Supervisor::list).

use std::fmt;

use serde::{Deserialize, Serialize};

/// A human-readable, unique name for an agent tracked by the supervisor.
///
/// `AgentId` is a thin newtype over [`String`]. It is cheap to clone, hashes
/// and orders as the underlying string, and round-trips through serde as a
/// bare JSON string (`"planner"`, not `{"0":"planner"}`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentId(String);

impl AgentId {
    /// Build an id from anything string-like.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Borrow the underlying name.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Consume the id, returning the owned [`String`].
    #[must_use]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl From<String> for AgentId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for AgentId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl From<AgentId> for String {
    fn from(value: AgentId) -> Self {
        value.0
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// A point-in-time snapshot of one tracked agent, returned by
/// [`Supervisor::list`](crate::Supervisor::list).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentStatus {
    /// The agent's unique id.
    pub id: AgentId,
    /// The model id the agent's session targets (from its
    /// [`EngineConfig`](aphrody_engine::EngineConfig)). Informational only.
    pub model: String,
    /// Whether the engine reports tool calls are auto-approved
    /// ([`FullAuto`](aphrody_engine::AutonomyMode::FullAuto)) for this agent.
    pub full_auto: bool,
}
