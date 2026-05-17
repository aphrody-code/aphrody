// SPDX-License-Identifier: Apache-2.0
//! MCP server status registry: tracks state and last RPC per server name.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::LlmEvent;

/// Point-in-time status of one MCP server.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct McpServerStatus {
    /// Server name as declared in `mcp.json`.
    pub server: String,
    /// Current state string (e.g. `"connected"`, `"disconnected"`, `"error"`).
    pub state: String,
    /// Last RPC method name observed, if any.
    pub last_rpc: Option<String>,
}

struct Inner {
    servers: HashMap<String, McpServerStatus>,
}

/// Thread-safe registry of MCP server statuses.
#[derive(Clone)]
pub struct McpStatusRegistry {
    inner: Arc<Mutex<Inner>>,
}

impl McpStatusRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                servers: HashMap::new(),
            })),
        }
    }

    /// Update (insert or replace) the status entry for `server`.
    pub fn update(
        &self,
        server: impl Into<String>,
        state: impl Into<String>,
        rpc: Option<String>,
    ) {
        let status = McpServerStatus {
            server: server.into(),
            state: state.into(),
            last_rpc: rpc,
        };
        let mut guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        guard.servers.insert(status.server.clone(), status);
    }

    /// Return a snapshot of all known server statuses.
    pub fn list(&self) -> Vec<McpServerStatus> {
        let guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        guard.servers.values().cloned().collect()
    }

    /// Convenience method: apply an `LlmEvent::Mcp` event from the bus.
    pub fn apply_event(&self, ev: &LlmEvent) {
        if let LlmEvent::Mcp { server, state, rpc } = ev {
            self.update(server, state, rpc.clone());
        }
    }
}

impl Default for McpStatusRegistry {
    fn default() -> Self {
        Self::new()
    }
}
