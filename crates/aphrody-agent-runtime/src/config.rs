// SPDX-License-Identifier: Apache-2.0
//! Runtime configuration: the knobs a surface sets to assemble an agent.

use std::path::PathBuf;
use std::time::Duration;

use aphrody_agent_tools::{DEFAULT_MAX_OUTPUT_BYTES, DEFAULT_TIMEOUT, ShellExecConfig};
use aphrody_engine::{AutonomyMode, EngineConfig};

/// Opt-in bounds for the shell-exec tool.
///
/// Both tools assembled by the runtime are *permissive by default* (CLAUDE.md
/// §0.1: no human in the loop): there is no command allow/deny-listing and no
/// path jailing. These two fields only bound runaway commands (a wall-clock
/// timeout and an output cap) and map directly onto
/// [`ShellExecConfig`](aphrody_agent_tools::ShellExecConfig).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellGuardrails {
    /// Wall-clock timeout applied when a tool call omits its own `timeout_ms`.
    pub default_timeout: Duration,
    /// Maximum number of bytes retained from a command's combined output.
    pub max_output_bytes: usize,
}

impl Default for ShellGuardrails {
    fn default() -> Self {
        Self {
            default_timeout: DEFAULT_TIMEOUT,
            max_output_bytes: DEFAULT_MAX_OUTPUT_BYTES,
        }
    }
}

impl ShellGuardrails {
    /// Translate these guardrails into the tool crate's
    /// [`ShellExecConfig`](aphrody_agent_tools::ShellExecConfig).
    #[must_use]
    pub(crate) fn to_shell_config(&self) -> ShellExecConfig {
        ShellExecConfig {
            default_timeout: self.default_timeout,
            max_output_bytes: self.max_output_bytes,
        }
    }
}

/// Everything a surface configures to assemble a ready-to-run agent.
///
/// Build one with [`RuntimeConfig::new`] (or [`RuntimeConfig::default`]) and the
/// `with_*` builder methods, then hand it to
/// [`AgentRuntime::builder`](crate::AgentRuntime::builder).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeConfig {
    /// The model identifier the session targets (informational on the engine
    /// side; the actual model is chosen by the
    /// [`ModelChoice`](crate::ModelChoice)).
    pub model: String,
    /// Optional system instruction prepended to every model request.
    pub system_prompt: Option<String>,
    /// Tool-call approval policy. Defaults to
    /// [`AutonomyMode::FullAuto`](aphrody_engine::AutonomyMode::FullAuto).
    pub autonomy: AutonomyMode,
    /// Maximum number of model<->tool round trips within a single turn.
    pub max_tool_iterations: usize,
    /// Working directory the tools resolve relative paths against. When `None`
    /// the agent's process directory is used.
    pub cwd: Option<PathBuf>,
    /// Opt-in shell-exec guardrails (timeout / output cap).
    pub shell: ShellGuardrails,
    /// When set, every emitted event is mirrored to a rollout JSONL file under
    /// this directory (see [`RolloutRecorder`](aphrody_rollout::RolloutRecorder)).
    pub rollout_path: Option<PathBuf>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            model: String::new(),
            system_prompt: None,
            // No human in the loop by default (CLAUDE.md §0.1).
            autonomy: AutonomyMode::FullAuto,
            max_tool_iterations: aphrody_engine::DEFAULT_MAX_TOOL_ITERATIONS,
            cwd: None,
            shell: ShellGuardrails::default(),
            rollout_path: None,
        }
    }
}

impl RuntimeConfig {
    /// Build a config targeting `model` with runtime defaults otherwise.
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

    /// Set the autonomy mode (builder style).
    #[must_use]
    pub fn with_autonomy(mut self, autonomy: AutonomyMode) -> Self {
        self.autonomy = autonomy;
        self
    }

    /// Set the per-turn tool-iteration budget (builder style).
    #[must_use]
    pub fn with_max_tool_iterations(mut self, max: usize) -> Self {
        self.max_tool_iterations = max;
        self
    }

    /// Set the working directory the tools resolve relative paths against
    /// (builder style).
    #[must_use]
    pub fn with_cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    /// Set the shell-exec guardrails (builder style).
    #[must_use]
    pub fn with_shell_guardrails(mut self, shell: ShellGuardrails) -> Self {
        self.shell = shell;
        self
    }

    /// Record every emitted event to a rollout JSONL file under `dir` (builder
    /// style).
    #[must_use]
    pub fn with_rollout_path(mut self, dir: impl Into<PathBuf>) -> Self {
        self.rollout_path = Some(dir.into());
        self
    }

    /// Project this config onto the engine's [`EngineConfig`].
    #[must_use]
    pub(crate) fn to_engine_config(&self) -> EngineConfig {
        EngineConfig {
            model: self.model.clone(),
            system_prompt: self.system_prompt.clone(),
            max_tool_iterations: self.max_tool_iterations,
            autonomy: self.autonomy,
        }
    }
}
