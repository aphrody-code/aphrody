// SPDX-License-Identifier: Apache-2.0
//! Task tree: tracks hierarchical task relationships and their statuses.
//!
//! Task statuses are arbitrary strings emitted by sub-agents:
//! `"pending"`, `"in_progress"`, `"completed"`, `"deleted"`, `"blocked"`, etc.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::LlmEvent;

/// A single node in the task tree.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TaskNode {
    /// Unique task id (arbitrary string).
    pub id: String,
    /// Optional parent task id.
    pub parent: Option<String>,
    /// Direct children of this task (by id).
    pub children: Vec<String>,
    /// Arbitrary status string.
    pub status: String,
    /// Human-readable description / title.
    pub subject: String,
}

struct Inner {
    nodes: HashMap<String, TaskNode>,
}

/// Thread-safe directed task tree keyed by id.
///
/// Parent/child links are maintained automatically: updating a task that
/// declares a parent inserts a child link into the parent if the parent exists.
#[derive(Clone)]
pub struct TaskTree {
    inner: Arc<Mutex<Inner>>,
}

impl TaskTree {
    /// Create a new empty task tree.
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                nodes: HashMap::new(),
            })),
        }
    }

    /// Update (insert or replace) a task node.
    ///
    /// If the node already exists its `parent` and `children` fields are
    /// preserved unless the caller explicitly passes a new parent via a wrapper
    /// that provides the full `TaskNode`. Use `update_fields` for partial updates.
    ///
    /// The `parent` parameter, if `Some`, will have `id` appended to its
    /// children list (deduplicated).
    pub fn update(
        &self,
        id: impl Into<String>,
        parent: Option<String>,
        status: impl Into<String>,
        subject: impl Into<String>,
    ) {
        let id = id.into();
        let status = status.into();
        let subject = subject.into();

        let mut guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());

        // Preserve existing children if the node already existed.
        let existing_children = guard
            .nodes
            .get(&id)
            .map(|n| n.children.clone())
            .unwrap_or_default();

        let node = TaskNode {
            id: id.clone(),
            parent: parent.clone(),
            children: existing_children,
            status,
            subject,
        };
        guard.nodes.insert(id.clone(), node);

        // Wire child link into parent, if parent exists.
        if let Some(ref pid) = parent {
            if let Some(parent_node) = guard.nodes.get_mut(pid) {
                if !parent_node.children.contains(&id) {
                    parent_node.children.push(id);
                }
            }
        }
    }

    /// Return a snapshot of the node with the given id, if it exists.
    pub fn get(&self, id: &str) -> Option<TaskNode> {
        let guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        guard.nodes.get(id).cloned()
    }

    /// Return a snapshot of all nodes.
    pub fn list(&self) -> Vec<TaskNode> {
        let guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        guard.nodes.values().cloned().collect()
    }

    /// Return top-level task ids (nodes with no parent or a parent not yet seen).
    pub fn roots(&self) -> Vec<String> {
        let guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        guard
            .nodes
            .values()
            .filter(|n| {
                n.parent
                    .as_ref()
                    .map(|pid| !guard.nodes.contains_key(pid))
                    .unwrap_or(true)
            })
            .map(|n| n.id.clone())
            .collect()
    }

    /// Convenience method: apply an `LlmEvent::Task` event from the bus.
    ///
    /// The OSC sequence for tasks does not carry parent info, so parent is set
    /// to `None` (preserving any existing parent from a prior update).
    pub fn apply_event(&self, ev: &LlmEvent) {
        if let LlmEvent::Task { id, status, subject } = ev {
            // Preserve existing parent.
            let existing_parent = self
                .get(id)
                .and_then(|n| n.parent);
            self.update(id, existing_parent, status, subject);
        }
    }
}

impl Default for TaskTree {
    fn default() -> Self {
        Self::new()
    }
}
