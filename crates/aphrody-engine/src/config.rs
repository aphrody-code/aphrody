// SPDX-License-Identifier: Apache-2.0
//! Engine configuration: model, system prompt, iteration budget, autonomy.

/// How the engine treats tool calls with respect to human approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AutonomyMode {
    /// No human in the loop (CLAUDE.md §0.1): every tool call is executed
    /// immediately without asking for approval. This is the default.
    #[default]
    FullAuto,
    /// Before executing each tool call the engine emits an
    /// [`ExecApprovalRequest`](aphrody_agent_proto::EventMsg::ExecApprovalRequest)
    /// and waits for an
    /// [`ExecApproval`](aphrody_agent_proto::Op::ExecApproval) decision via the
    /// engine's [`ApprovalGate`](crate::ApprovalGate).
    Gated,
}

/// Tunables for a single agent [`Session`](crate::Session) / turn loop.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineConfig {
    /// The model identifier the session targets (informational; the actual
    /// model is selected by the [`ModelClient`](aphrody_model_client::ModelClient)
    /// passed to [`run_turn`](crate::run_turn)).
    pub model: String,
    /// Optional system instruction prepended to every model request.
    pub system_prompt: Option<String>,
    /// Maximum number of model<->tool round trips within a single turn before
    /// the loop gives up. Defaults to `8`.
    pub max_tool_iterations: usize,
    /// Tool-call approval policy.
    pub autonomy: AutonomyMode,
}

/// Default maximum number of model<->tool round trips per turn.
pub const DEFAULT_MAX_TOOL_ITERATIONS: usize = 8;

impl Default for EngineConfig {
    fn default() -> Self {
        Self {
            model: String::new(),
            system_prompt: None,
            max_tool_iterations: DEFAULT_MAX_TOOL_ITERATIONS,
            autonomy: AutonomyMode::default(),
        }
    }
}

impl EngineConfig {
    /// Build a config targeting `model` with engine defaults otherwise.
    #[must_use]
    pub fn new(model: impl Into<String>) -> Self {
        Self {
            model: model.into(),
            ..Self::default()
        }
    }

    /// Set the system prompt (builder style).
    #[must_use]
    pub fn with_system_prompt(mut self, system_prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(system_prompt.into());
        self
    }

    /// Set the tool-iteration budget (builder style).
    #[must_use]
    pub fn with_max_tool_iterations(mut self, max: usize) -> Self {
        self.max_tool_iterations = max;
        self
    }

    /// Set the autonomy mode (builder style).
    #[must_use]
    pub fn with_autonomy(mut self, autonomy: AutonomyMode) -> Self {
        self.autonomy = autonomy;
        self
    }
}
