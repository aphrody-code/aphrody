// SPDX-License-Identifier: Apache-2.0
//! Skill slot registry: tracks per-skill activation state.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::LlmEvent;

/// Activation state of a skill slot.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SkillState {
    /// Skill is loaded but not currently executing.
    Idle,
    /// Skill is currently executing.
    Active,
    /// Skill encountered an error on its last invocation.
    Error { message: String },
}

/// Per-skill slot record.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct SkillSlotEntry {
    pub name: String,
    pub state: SkillState,
    pub last_phase: Option<String>,
    pub last_payload: Option<serde_json::Value>,
}

struct Inner {
    slots: HashMap<String, SkillSlotEntry>,
}

/// Thread-safe registry of skill activation slots.
#[derive(Clone)]
pub struct SkillSlot {
    inner: Arc<Mutex<Inner>>,
}

impl SkillSlot {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self { inner: Arc::new(Mutex::new(Inner { slots: HashMap::new() })) }
    }

    /// Activate (or update) a skill slot. Sets state to `Active`.
    pub fn activate(
        &self,
        name: impl Into<String>,
        phase: impl Into<String>,
        payload: serde_json::Value,
    ) {
        let name = name.into();
        let phase = phase.into();
        let mut guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        let entry = guard.slots.entry(name.clone()).or_insert_with(|| SkillSlotEntry {
            name: name.clone(),
            state: SkillState::Idle,
            last_phase: None,
            last_payload: None,
        });
        entry.state = SkillState::Active;
        entry.last_phase = Some(phase);
        entry.last_payload = Some(payload);
    }

    /// Deactivate a skill slot by name. Sets state to `Idle` if it exists.
    pub fn deactivate(&self, name: &str) {
        let mut guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(entry) = guard.slots.get_mut(name) {
            entry.state = SkillState::Idle;
        }
    }

    /// Mark a skill slot as errored.
    pub fn set_error(&self, name: &str, message: impl Into<String>) {
        let mut guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(entry) = guard.slots.get_mut(name) {
            entry.state = SkillState::Error { message: message.into() };
        }
    }

    /// Return a cloned snapshot of all skill slots.
    pub fn list(&self) -> Vec<SkillSlotEntry> {
        let guard = self.inner.lock().unwrap_or_else(|p| p.into_inner());
        guard.slots.values().cloned().collect()
    }

    /// Convenience method: apply an `LlmEvent::Skill` event from the bus.
    ///
    /// Phase `"start"` / `"invoke"` sets Active; phase `"done"` / `"complete"`
    /// sets Idle; phase `"error"` sets Error. Any other phase updates last_phase
    /// without changing state.
    pub fn apply_event(&self, ev: &LlmEvent) {
        if let LlmEvent::Skill { name, phase, payload } = ev {
            match phase.as_str() {
                "start" | "invoke" => self.activate(name, phase, payload.clone()),
                "done" | "complete" => {
                    // Ensure slot exists, then set idle.
                    self.activate(name, phase, payload.clone());
                    self.deactivate(name);
                },
                "error" => {
                    let msg = payload
                        .get("message")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown error")
                        .to_owned();
                    // Create slot if it doesn't exist.
                    self.activate(name, phase, payload.clone());
                    self.set_error(name, msg);
                },
                _ => {
                    // Update last_phase without forcing a state transition.
                    self.activate(name, phase, payload.clone());
                },
            }
        }
    }
}

impl Default for SkillSlot {
    fn default() -> Self {
        Self::new()
    }
}
