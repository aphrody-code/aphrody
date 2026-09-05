// SPDX-License-Identifier: Apache-2.0
//! The assembled agent: build it from a [`RuntimeConfig`] + [`ModelChoice`],
//! then either [`spawn`](AgentRuntime::spawn) it for interactive use or drive a
//! single headless turn with [`run_once`](AgentRuntime::run_once).

use std::sync::Arc;

use aphrody_agent_proto::{Event, InputItem};
use aphrody_engine::{
    EventSink, Session, SessionHandle, TurnOutcome, run_turn, spawn_session,
};
use aphrody_model_client::ModelClient;
use aphrody_rollout::RolloutRecorder;
use aphrody_toolcall::ToolRegistry;

use crate::config::RuntimeConfig;
use crate::error::RuntimeError;
use crate::model::ModelChoice;
use crate::tools::build_tool_registry;

/// The outcome of a headless [`AgentRuntime::run_once`] call.
#[derive(Debug, Clone, Default)]
pub struct RunResult {
    /// The last visible agent message produced during the turn, if any.
    pub last_agent_message: Option<String>,
    /// Every [`Event`] the engine emitted during the turn, in order.
    pub events: Vec<Event>,
    /// The engine's structured turn outcome (iteration count, interruption /
    /// iteration-cap flags).
    pub outcome: TurnOutcome,
}

/// A fully wired agent: a model client, a tool registry, an engine config, and
/// an optional rollout recorder.
///
/// Construct one through [`AgentRuntime::builder`].
pub struct AgentRuntime {
    config: RuntimeConfig,
    client: Arc<dyn ModelClient>,
    tools: Arc<ToolRegistry>,
}

impl std::fmt::Debug for AgentRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentRuntime")
            .field("config", &self.config)
            .field("model_id", &self.client.model_id())
            .field("tools", &self.tools)
            .finish()
    }
}

impl AgentRuntime {
    /// Start building a runtime.
    #[must_use]
    pub fn builder() -> AgentRuntimeBuilder {
        AgentRuntimeBuilder::default()
    }

    /// The configuration this runtime was assembled from.
    #[must_use]
    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }

    /// The tool registry the agent exposes to the model.
    #[must_use]
    pub fn tools(&self) -> &Arc<ToolRegistry> {
        &self.tools
    }

    /// Build a fresh [`RolloutRecorder`] when a rollout path is configured.
    async fn open_rollout(&self) -> Result<Option<RolloutRecorder>, RuntimeError> {
        match &self.config.rollout_path {
            Some(dir) => Ok(Some(RolloutRecorder::new(dir, None).await?)),
            None => Ok(None),
        }
    }

    /// Spawn a long-lived session actor for interactive surfaces (CLI / TUI /
    /// desktop).
    ///
    /// The returned [`SessionHandle`] accepts
    /// [`Op`](aphrody_agent_proto::Op) submissions and streams
    /// [`Event`](aphrody_agent_proto::Event)s back. The runtime is consumed; its
    /// model client and tool registry are moved into the actor.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::Rollout`] if the configured rollout file cannot
    /// be opened.
    pub async fn spawn(self) -> Result<SessionHandle, RuntimeError> {
        let rollout = self.open_rollout().await?;
        let session = Session::new(self.config.to_engine_config());
        Ok(spawn_session(session, self.client, self.tools, rollout))
    }

    /// Drive one full headless turn for `prompt` and collect the result.
    ///
    /// This appends the prompt as user input, runs the engine turn loop to
    /// completion (executing any tool calls the model requests and re-injecting
    /// their output for multi-step reasoning), drains every emitted
    /// [`Event`], and returns them alongside the final agent message and the
    /// engine's [`TurnOutcome`]. The runtime is consumed.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::Rollout`] if the configured rollout file cannot
    /// be opened, or [`RuntimeError::Engine`] if the turn loop fails (e.g. the
    /// model stream errors).
    pub async fn run_once(self, prompt: impl Into<String>) -> Result<RunResult, RuntimeError> {
        let rollout = self.open_rollout().await?;
        let mut session = Session::new(self.config.to_engine_config());
        let (sink, mut rx) = EventSink::unbounded();

        let input = vec![InputItem::Text {
            text: prompt.into(),
        }];

        // In FullAuto there is no approval gate; in Gated mode a headless caller
        // has no one to ask, so we still pass `None` and the engine denies tool
        // calls rather than blocking (engine semantics, turn.rs).
        let outcome = run_turn(
            &mut session,
            self.client.as_ref(),
            self.tools.as_ref(),
            rollout.as_ref(),
            input,
            &sink,
            None,
        )
        .await?;

        // The sink's sender is dropped with `sink` here; drain whatever the
        // engine emitted (the channel is unbounded so nothing was lost).
        drop(sink);
        let mut events = Vec::new();
        while let Ok(event) = rx.try_recv() {
            events.push(event);
        }

        if let Some(rollout) = rollout {
            rollout.shutdown().await?;
        }

        Ok(RunResult {
            last_agent_message: outcome.last_agent_message.clone(),
            events,
            outcome,
        })
    }
}

/// Builder for [`AgentRuntime`].
///
/// A [`ModelChoice`] is required; the [`RuntimeConfig`] defaults to
/// [`RuntimeConfig::default`] (which is [`AutonomyMode::FullAuto`](aphrody_engine::AutonomyMode::FullAuto)).
#[derive(Debug, Default)]
pub struct AgentRuntimeBuilder {
    config: Option<RuntimeConfig>,
    model: Option<ModelChoice>,
}

impl AgentRuntimeBuilder {
    /// Set the runtime configuration.
    #[must_use]
    pub fn config(mut self, config: RuntimeConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Set the model backend (required).
    #[must_use]
    pub fn model(mut self, model: ModelChoice) -> Self {
        self.model = Some(model);
        self
    }

    /// Assemble the runtime.
    ///
    /// # Errors
    ///
    /// Returns [`RuntimeError::Config`] if no [`ModelChoice`] was set, or
    /// [`RuntimeError::ToolRegistration`] if the tool registry could not be
    /// built.
    pub fn build(self) -> Result<AgentRuntime, RuntimeError> {
        let model = self
            .model
            .ok_or_else(|| RuntimeError::Config("a ModelChoice is required".to_string()))?;
        let config = self.config.unwrap_or_default();
        let tools = build_tool_registry(&config)?;
        Ok(AgentRuntime {
            config,
            client: model.into_client(),
            tools,
        })
    }
}
