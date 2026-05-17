// SPDX-License-Identifier: Apache-2.0
//! Hook event log: fixed-capacity ring buffer of hook firings.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use crate::LlmEvent;

/// A single hook firing record.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct HookEntry {
    /// The hook event name (e.g. `"PreToolUse"`, `"PostToolUse"`).
    pub event: String,
    /// Arbitrary JSON payload attached to this firing.
    pub payload: serde_json::Value,
}

struct Inner {
    entries: VecDeque<HookEntry>,
    capacity: usize,
}

/// Thread-safe ring buffer of hook events with a fixed capacity.
///
/// When at capacity, the oldest entry is dropped to make room for the new one.
#[derive(Clone)]
pub struct HookEventLog {
    inner: Arc<Mutex<Inner>>,
}

impl HookEventLog {
    /// Create a log with the given maximum number of entries.
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                entries: VecDeque::with_capacity(capacity.min(1024)),
                capacity,
            })),
        }
    }

    /// Push a new entry. If at capacity, the oldest is evicted.
    pub fn push(&self, entry: HookEntry) {
        let mut guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        if guard.entries.len() >= guard.capacity {
            guard.entries.pop_front();
        }
        guard.entries.push_back(entry);
    }

    /// Drain and return all buffered entries, leaving the log empty.
    pub fn drain(&self) -> Vec<HookEntry> {
        let mut guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        guard.entries.drain(..).collect()
    }

    /// Return a cloned snapshot of all entries without clearing the log.
    pub fn iter(&self) -> Vec<HookEntry> {
        let guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        guard.entries.iter().cloned().collect()
    }

    /// Current number of buffered entries.
    pub fn len(&self) -> usize {
        let guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        guard.entries.len()
    }

    /// True if there are no buffered entries.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Convenience method: apply an `LlmEvent::Hook` event from the bus.
    pub fn apply_event(&self, ev: &LlmEvent) {
        if let LlmEvent::Hook { event, payload } = ev {
            self.push(HookEntry {
                event: event.clone(),
                payload: payload.clone(),
            });
        }
    }
}
