// SPDX-License-Identifier: Apache-2.0
//! Sub-agent registry: tracks live sub-agent state by id.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use tokio::sync::broadcast;

use crate::LlmEvent;

/// Point-in-time snapshot of one sub-agent.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SubAgentInfo {
    /// Unique agent id (arbitrary string, e.g. `"sa-1"`, `"claude-subagent-3"`).
    pub id: String,
    /// Last reported status string (e.g. `"running"`, `"done"`, `"failed"`).
    pub status: String,
    /// Last text emitted by this agent.
    pub last_text: String,
}

struct Inner {
    agents: HashMap<String, SubAgentInfo>,
    /// Sender kept alive so the channel isn't dropped when all external
    /// receivers disconnect and reconnect.
    tx: broadcast::Sender<SubAgentInfo>,
}

/// Thread-safe registry of known sub-agents.
///
/// Callers receive cloned snapshots so no lock is held across await points.
#[derive(Clone)]
pub struct SubAgentRegistry {
    inner: Arc<Mutex<Inner>>,
}

impl SubAgentRegistry {
    /// Create a new empty registry with the given channel capacity for live updates.
    pub fn new(channel_capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(channel_capacity);
        Self { inner: Arc::new(Mutex::new(Inner { agents: HashMap::new(), tx })) }
    }

    /// Update (insert or replace) the entry for `id` and broadcast the new snapshot.
    pub fn update(
        &self,
        id: impl Into<String>,
        status: impl Into<String>,
        text: impl Into<String>,
    ) {
        let info = SubAgentInfo { id: id.into(), status: status.into(), last_text: text.into() };
        let mut guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        guard.agents.insert(info.id.clone(), info.clone());
        // Broadcast even if there are no listeners (that's fine — SendError just means 0
        // receivers).
        let _ = guard.tx.send(info);
    }

    /// Return a snapshot of all known agents.
    pub fn list(&self) -> Vec<SubAgentInfo> {
        let guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        guard.agents.values().cloned().collect()
    }

    /// Subscribe to live agent update events.
    pub fn live_stream(&self) -> broadcast::Receiver<SubAgentInfo> {
        let guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        guard.tx.subscribe()
    }

    /// Convenience method: apply an `LlmEvent::SubAgent` event from the bus.
    pub fn apply_event(&self, ev: &LlmEvent) {
        if let LlmEvent::SubAgent { id, status, text } = ev {
            self.update(id, status, text);
        }
    }
}
