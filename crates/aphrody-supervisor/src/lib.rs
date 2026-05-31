// SPDX-License-Identifier: Apache-2.0
//! `aphrody-supervisor` — the multi-agent control plane for aphrody.
//!
//! This crate is Phase 3 of aphrody's Rust rewrite of Antigravity (see
//! `docs/VISION.md`): the orchestration layer that sits *above* the single-turn
//! [`aphrody-engine`](aphrody_engine). Where the engine drives one
//! [`Session`](aphrody_engine::Session), the [`Supervisor`] owns *N* named
//! sessions at once and gives a caller four things:
//!
//! - **Spawn / track** — [`Supervisor::spawn_agent`] starts an engine session
//!   under a unique [`AgentId`] and rejects a duplicate id.
//! - **Route** — [`Supervisor::submit`] (and the ergonomic
//!   [`Supervisor::send_user_text`]) deliver a
//!   [`Submission`](aphrody_agent_proto::Submission) to one chosen agent.
//! - **Fan-in** — every agent's event stream is multiplexed into a single
//!   tagged channel, drained once via [`Supervisor::take_events`], yielding
//!   `(AgentId, Event)` pairs so a caller knows which agent emitted what.
//! - **Lifecycle** — [`Supervisor::list`], [`Supervisor::interrupt`],
//!   [`Supervisor::shutdown`], [`Supervisor::shutdown_all`], and
//!   [`Supervisor::join`].
//!
//! # Autonomy
//!
//! The supervisor imposes no autonomy policy of its own. Each agent's policy
//! lives in the [`EngineConfig`](aphrody_engine::EngineConfig) carried by the
//! [`Session`](aphrody_engine::Session) the caller hands to
//! [`spawn_agent`](Supervisor::spawn_agent). Per the repo's
//! no-human-in-the-loop default (CLAUDE.md §0.1) that config defaults to
//! [`AutonomyMode::FullAuto`](aphrody_engine::AutonomyMode::FullAuto).
//!
//! # Example
//!
//! ```no_run
//! # use std::sync::Arc;
//! # use aphrody_supervisor::Supervisor;
//! # use aphrody_engine::{EngineConfig, Session};
//! # use aphrody_model_client::ModelClient;
//! # use aphrody_toolcall::ToolRegistry;
//! # async fn demo(client: Arc<dyn ModelClient>, tools: Arc<ToolRegistry>) {
//! let mut sup = Supervisor::new();
//! let mut events = sup.take_events().expect("events drained once");
//!
//! let session = Session::new(EngineConfig::new("gemini-2.5-flash"));
//! sup.spawn_agent("planner", session, client, tools, None)
//!     .expect("spawn");
//! sup.send_user_text("planner", "draft a plan").expect("route");
//!
//! while let Some((id, event)) = events.recv().await {
//!     let _ = (id, event);
//! }
//! # }
//! ```

#![forbid(unsafe_code)]

mod agent;
mod error;

use std::collections::HashMap;
use std::sync::Arc;

use aphrody_agent_proto::{Event, InputItem, Op};
use aphrody_engine::{AutonomyMode, Session, SessionHandle, spawn_session};
use aphrody_model_client::ModelClient;
use aphrody_rollout::RolloutRecorder;
use aphrody_toolcall::ToolRegistry;
use tokio::sync::mpsc;
use tokio::task::{JoinError, JoinHandle};

pub use agent::{AgentId, AgentStatus};
pub use error::SupervisorError;

/// One tracked agent: its live session handle, forwarding task, and the
/// metadata needed to project an [`AgentStatus`].
struct AgentEntry {
    /// Handle to the engine session actor (used for routing and shutdown).
    handle: SessionHandle,
    /// The task that forwards this agent's events into the shared fan-in
    /// channel, tagging each with the agent's id. Aborted on shutdown.
    forwarder: JoinHandle<()>,
    /// Model id from the session's config (informational, for [`AgentStatus`]).
    model: String,
    /// Whether the session runs fully autonomously (no approval gate).
    full_auto: bool,
}

/// The multi-agent control plane.
///
/// Owns a map of [`AgentId`] -> live engine session plus a single shared
/// fan-in channel that all agents' events are multiplexed into. See the
/// [crate docs](crate) for the full model.
pub struct Supervisor {
    /// Tracked agents keyed by id.
    agents: HashMap<AgentId, AgentEntry>,
    /// Producer side of the fan-in channel; cloned into each agent's forwarding
    /// task so every agent feeds the one shared stream.
    events_tx: mpsc::UnboundedSender<(AgentId, Event)>,
    /// Consumer side of the fan-in channel, handed out exactly once via
    /// [`Supervisor::take_events`].
    events_rx: Option<mpsc::UnboundedReceiver<(AgentId, Event)>>,
}

impl Supervisor {
    /// Create an empty supervisor with a fresh fan-in channel.
    #[must_use]
    pub fn new() -> Self {
        let (events_tx, events_rx) = mpsc::unbounded_channel();
        Self {
            agents: HashMap::new(),
            events_tx,
            events_rx: Some(events_rx),
        }
    }

    /// Number of currently tracked agents.
    #[must_use]
    pub fn count(&self) -> usize {
        self.agents.len()
    }

    /// Whether no agents are tracked.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.agents.is_empty()
    }

    /// Whether an agent with `id` is currently tracked.
    pub fn contains(&self, id: impl Into<AgentId>) -> bool {
        self.agents.contains_key(&id.into())
    }

    /// Take ownership of the shared fan-in receiver (callable once).
    ///
    /// Returns the single `(AgentId, Event)` stream aggregating every agent's
    /// events. A second call returns [`None`] because the receiver has already
    /// been handed out. Drop this receiver to stop accepting forwarded events.
    pub fn take_events(&mut self) -> Option<mpsc::UnboundedReceiver<(AgentId, Event)>> {
        self.events_rx.take()
    }

    /// Spawn a new engine session under `id`, starting event forwarding.
    ///
    /// The session is driven by `client` + `tools`, optionally recording to
    /// `rollout`. The agent's events are immediately multiplexed into the
    /// shared fan-in channel (drained via [`take_events`](Self::take_events)),
    /// each tagged with `id`.
    ///
    /// # Errors
    /// Returns [`SupervisorError::DuplicateId`] if an agent with the same id is
    /// already tracked. The session is not started in that case.
    pub fn spawn_agent(
        &mut self,
        id: impl Into<AgentId>,
        session: Session,
        client: Arc<dyn ModelClient>,
        tools: Arc<ToolRegistry>,
        rollout: Option<RolloutRecorder>,
    ) -> Result<(), SupervisorError> {
        let id = id.into();
        if self.agents.contains_key(&id) {
            return Err(SupervisorError::DuplicateId(id));
        }

        let model = session.config().model.clone();
        let full_auto = session.config().autonomy == AutonomyMode::FullAuto;

        let mut handle = spawn_session(session, client, tools, rollout);
        // `events()` yields the receiver once; a freshly spawned handle always
        // has it, so this is the single owner of that agent's event stream.
        let mut agent_events = handle
            .events()
            .ok_or(SupervisorError::ChannelClosed)?;

        let events_tx = self.events_tx.clone();
        let forward_id = id.clone();
        let forwarder = tokio::spawn(async move {
            while let Some(event) = agent_events.recv().await {
                // The shared receiver was dropped: stop forwarding for this
                // agent. Other agents' forwarders are unaffected.
                if events_tx.send((forward_id.clone(), event)).is_err() {
                    break;
                }
            }
        });

        self.agents.insert(
            id,
            AgentEntry {
                handle,
                forwarder,
                model,
                full_auto,
            },
        );
        Ok(())
    }

    /// Route an arbitrary [`Op`] to one agent's session.
    ///
    /// The engine wraps the op in a fresh
    /// [`Submission`](aphrody_agent_proto::Submission) internally.
    ///
    /// # Errors
    /// - [`SupervisorError::UnknownAgent`] if no agent with `id` is tracked.
    /// - [`SupervisorError::Engine`] if the agent's session actor has already
    ///   stopped and can no longer accept submissions.
    pub fn submit(&self, id: impl Into<AgentId>, op: Op) -> Result<(), SupervisorError> {
        let id = id.into();
        let entry = self
            .agents
            .get(&id)
            .ok_or_else(|| SupervisorError::UnknownAgent(id.clone()))?;
        entry.handle.submit(op).map_err(|rejected| SupervisorError::Engine {
            id,
            message: format!(
                "session actor stopped; submission rejected (op: {:?})",
                rejected.op
            ),
        })
    }

    /// Send a plain-text user turn to one agent.
    ///
    /// Ergonomic wrapper that builds the
    /// [`Op::UserInput`](aphrody_agent_proto::Op::UserInput) with a single
    /// [`InputItem::Text`](aphrody_agent_proto::InputItem::Text) and routes it
    /// via [`submit`](Self::submit).
    ///
    /// # Errors
    /// Same conditions as [`submit`](Self::submit).
    pub fn send_user_text(
        &self,
        id: impl Into<AgentId>,
        text: impl Into<String>,
    ) -> Result<(), SupervisorError> {
        let op = Op::UserInput {
            items: vec![InputItem::Text { text: text.into() }],
        };
        self.submit(id, op)
    }

    /// Interrupt one agent's currently running turn.
    ///
    /// Sends [`Op::Interrupt`](aphrody_agent_proto::Op::Interrupt). The agent
    /// stays tracked; only its in-flight turn is asked to stop.
    ///
    /// # Errors
    /// Same conditions as [`submit`](Self::submit).
    pub fn interrupt(&self, id: impl Into<AgentId>) -> Result<(), SupervisorError> {
        self.submit(id, Op::Interrupt)
    }

    /// Inject steering guidance into one agent's running turn.
    ///
    /// Sends [`Op::Steer`](aphrody_agent_proto::Op::Steer) with `items`; the
    /// engine folds them into the conversation before its next model call so the
    /// agent course-corrects mid-turn without restarting. If no turn is running
    /// the op is a no-op on the engine side (the agent stays tracked).
    ///
    /// # Errors
    /// Same conditions as [`submit`](Self::submit).
    pub fn steer(
        &self,
        id: impl Into<AgentId>,
        items: Vec<InputItem>,
    ) -> Result<(), SupervisorError> {
        self.submit(id, Op::Steer { items })
    }

    /// Inject plain-text steering guidance into one agent's running turn.
    ///
    /// Ergonomic wrapper over [`steer`](Self::steer) with a single
    /// [`InputItem::Text`](aphrody_agent_proto::InputItem::Text).
    ///
    /// # Errors
    /// Same conditions as [`submit`](Self::submit).
    pub fn steer_text(
        &self,
        id: impl Into<AgentId>,
        text: impl Into<String>,
    ) -> Result<(), SupervisorError> {
        self.steer(id, vec![InputItem::Text { text: text.into() }])
    }

    /// Shut one agent down and stop tracking it.
    ///
    /// Sends [`Op::Shutdown`](aphrody_agent_proto::Op::Shutdown) so the session
    /// actor drains and exits, aborts the agent's event-forwarding task, and
    /// removes the agent from the map (so [`count`](Self::count) decrements and
    /// [`contains`](Self::contains) reports `false`).
    ///
    /// # Errors
    /// [`SupervisorError::UnknownAgent`] if no agent with `id` is tracked. A
    /// best-effort shutdown op is still sent even if the actor had already
    /// stopped; the agent is removed regardless.
    pub fn shutdown(&mut self, id: impl Into<AgentId>) -> Result<(), SupervisorError> {
        let id = id.into();
        let entry = self
            .agents
            .remove(&id)
            .ok_or_else(|| SupervisorError::UnknownAgent(id))?;
        // Best-effort: if the actor already stopped, the op is simply dropped.
        let _ = entry.handle.submit(Op::Shutdown);
        entry.forwarder.abort();
        Ok(())
    }

    /// Shut down every tracked agent and clear the map.
    ///
    /// Each agent receives [`Op::Shutdown`](aphrody_agent_proto::Op::Shutdown)
    /// and has its forwarder aborted, exactly as in [`shutdown`](Self::shutdown).
    pub fn shutdown_all(&mut self) {
        for (_, entry) in self.agents.drain() {
            let _ = entry.handle.submit(Op::Shutdown);
            entry.forwarder.abort();
        }
    }

    /// Await one agent's session actor task, recovering its final
    /// [`Session`] state, and stop tracking it.
    ///
    /// Removes the agent from the map first (so it is no longer routable), then
    /// awaits its actor task. The forwarding task is aborted once the session
    /// has joined. Call [`shutdown`](Self::shutdown) or send
    /// [`Op::Shutdown`](aphrody_agent_proto::Op::Shutdown) beforehand to ensure
    /// the actor is actually winding down, otherwise this awaits a still-live
    /// actor.
    ///
    /// # Errors
    /// - [`SupervisorError::UnknownAgent`] if no agent with `id` is tracked.
    /// - [`SupervisorError::Engine`] if the actor task panicked while joining.
    pub async fn join(&mut self, id: impl Into<AgentId>) -> Result<Session, SupervisorError> {
        let id = id.into();
        let entry = self
            .agents
            .remove(&id)
            .ok_or_else(|| SupervisorError::UnknownAgent(id.clone()))?;
        let AgentEntry {
            handle, forwarder, ..
        } = entry;
        let session = handle.join().await.map_err(|err: JoinError| SupervisorError::Engine {
            id,
            message: format!("session actor task join failed: {err}"),
        })?;
        forwarder.abort();
        Ok(session)
    }

    /// Snapshot every tracked agent.
    ///
    /// The returned vec is sorted by [`AgentId`] for deterministic ordering.
    #[must_use]
    pub fn list(&self) -> Vec<AgentStatus> {
        let mut out: Vec<AgentStatus> = self
            .agents
            .iter()
            .map(|(id, entry)| AgentStatus {
                id: id.clone(),
                model: entry.model.clone(),
                full_auto: entry.full_auto,
            })
            .collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }
}

impl Default for Supervisor {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Supervisor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut ids: Vec<&AgentId> = self.agents.keys().collect();
        ids.sort();
        f.debug_struct("Supervisor")
            .field("agents", &ids)
            .field("events_drained", &self.events_rx.is_none())
            .finish()
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
