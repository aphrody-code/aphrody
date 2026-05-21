// SPDX-License-Identifier: Apache-2.0
//! aphrody-hooks — runtime hook dispatcher for the aphrody agent stack.
//!
//! Hooks are user-defined shell commands that run on lifecycle events
//! (a tool is about to be called, a tool just returned, the user
//! submitted a prompt, the assistant stopped, a notification fired).
//! They can short-circuit tool execution by emitting a JSON decision
//! object on stdout (`{"decision": "block", "reason": "..."}`).
//!
//! The crate is intentionally protocol-shaped, not tied to any specific
//! agent runtime: it loads hook blocks from a JSON value (typically a
//! settings document under `~/.config/aphrody/settings.json` or the
//! equivalent Code-style `~/.claude/settings.json`), matches incoming
//! [`HookEvent`]s against [`HookMatcher`] globs, then spawns each
//! command via [`tokio::process::Command`] with the event JSON piped to
//! stdin and a per-hook timeout.
//!
//! ## Cohesion with the wider workspace
//!
//! * **`aphrody-cron`** — owns the periodic dispatch loop; a typical
//!   integration registers a job that ticks the hook dispatcher (e.g.
//!   for `Stop` events fired by the agent runtime).
//! * **`aphrody-skills-forge`** — uses similar serde DTO + thiserror
//!   patterns; this crate keeps the conventions aligned (PascalCase
//!   discriminants, `#[from]` IO/JSON, lints workspace-wide).
//! * **`aphrody-messaging`** — uses [`tokio::process`] async spawn
//!   patterns inspired by the SMTP connector's outbound handler.
//!
//! See `tests/hooks_smoke.rs` for the executable contract.

use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;
use std::time::{Duration, Instant};

use globset::{Glob, GlobMatcher};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::timeout;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Default per-hook execution budget when [`HookCommand::timeout_ms`]
/// is unset. Chosen to balance hook responsiveness with tolerating
/// short scripts that need to call out to local tooling.
pub const DEFAULT_HOOK_TIMEOUT_MS: u64 = 5_000;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// All recoverable failures surfaced by the dispatcher.
#[derive(Debug, Error)]
pub enum HookError {
    /// Filesystem failure (settings file unreadable, etc.).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// JSON (de)serialisation failure.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// A hook exceeded its allotted execution window.
    #[error("hook timed out after {elapsed_ms}ms: {command}")]
    Timeout {
        /// Command line that failed to return in time.
        command: String,
        /// Wall-clock elapsed time at the point the timeout fired.
        elapsed_ms: u64,
    },
    /// Failed to spawn the user-supplied command.
    #[error("failed to spawn hook command {command:?}: {source}")]
    Spawn {
        /// Command line that could not be launched.
        command: String,
        /// Underlying OS error.
        #[source]
        source: std::io::Error,
    },
    /// Failed to compile a glob matcher from a user-supplied pattern.
    #[error("invalid matcher glob {pattern:?}: {source}")]
    GlobInvalid {
        /// Pattern that failed to compile.
        pattern: String,
        /// Underlying globset error.
        #[source]
        source: globset::Error,
    },
}

// ---------------------------------------------------------------------------
// Event types — what triggers a hook.
// ---------------------------------------------------------------------------

/// Lifecycle event delivered to the dispatcher.
///
/// The serde representation is intentionally **untagged** so that an
/// event payload can round-trip through stdin without requiring the
/// hook script to dispatch on a discriminant: the script receives the
/// canonical fields directly (`tool_name`, `tool_input`, `prompt`, …).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HookEvent {
    /// A tool is about to be invoked.
    PreToolUse {
        /// Canonical tool identifier (e.g. `"Bash"`, `"Edit"`).
        tool_name: String,
        /// Opaque tool-specific arguments preserved as JSON.
        tool_input: serde_json::Value,
    },
    /// A tool has just returned.
    PostToolUse {
        /// Tool identifier (same as the matching PreToolUse event).
        tool_name: String,
        /// Original tool input, mirrored for convenience.
        tool_input: serde_json::Value,
        /// The tool's response payload as JSON.
        tool_response: serde_json::Value,
    },
    /// The user submitted a new prompt.
    UserPromptSubmit {
        /// The raw prompt string.
        prompt: String,
    },
    /// A notification was raised by the runtime.
    Notification {
        /// Human-readable notification body.
        message: String,
    },
    /// The agent loop stopped (terminal turn handed back to the user).
    Stop,
}

impl HookEvent {
    /// Canonical hook-kind name. This is the key under which matchers
    /// are stored in [`HookConfig::hooks`].
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::PreToolUse { .. } => "PreToolUse",
            Self::PostToolUse { .. } => "PostToolUse",
            Self::UserPromptSubmit { .. } => "UserPromptSubmit",
            Self::Notification { .. } => "Notification",
            Self::Stop => "Stop",
        }
    }

    /// Tool name carried by the event, if any. Used by [`HookMatcher`]
    /// to decide whether a glob applies.
    #[must_use]
    pub fn tool_name(&self) -> Option<&str> {
        match self {
            Self::PreToolUse { tool_name, .. } | Self::PostToolUse { tool_name, .. } => {
                Some(tool_name.as_str())
            }
            _ => None,
        }
    }
}

// ---------------------------------------------------------------------------
// Configuration shape — mirrors the settings.json `hooks` block.
// ---------------------------------------------------------------------------

/// Predicate deciding whether a given [`HookSpec`] applies to an event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct HookMatcher {
    /// Hook event kind this matcher binds to (e.g. `"PreToolUse"`).
    /// In the settings.json layout this is the outer map key, so it is
    /// optional on the per-spec record and filled in by the loader.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    /// Glob applied to the event's `tool_name`. `None` matches every
    /// event regardless of tool (used for tool-less events such as
    /// `UserPromptSubmit` or `Stop`, and as a wildcard for tool events).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matcher: Option<String>,
}

/// A single executable hook command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookCommand {
    /// Always `"command"` in the current schema; reserved for future
    /// inline-handler variants.
    #[serde(rename = "type", default = "default_command_type")]
    pub r#type: String,
    /// Shell-quoted command line passed to the platform shell
    /// (`cmd /c` on Windows, `sh -c` on Unix).
    pub command: String,
    /// Per-hook execution budget in milliseconds. When `None` the
    /// dispatcher applies [`DEFAULT_HOOK_TIMEOUT_MS`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

fn default_command_type() -> String {
    "command".to_string()
}

/// Group of commands sharing a single matcher predicate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HookSpec {
    /// Matcher predicate evaluated against the inbound event.
    #[serde(default, flatten)]
    pub matcher: HookMatcher,
    /// Ordered list of commands executed when the matcher fires.
    pub hooks: Vec<HookCommand>,
}

/// Root settings document subset — only the `hooks` map matters here.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HookConfig {
    /// Map keyed by hook-kind (`PreToolUse`, `PostToolUse`, …).
    /// Values are lists of [`HookSpec`]s evaluated in declaration order.
    #[serde(default)]
    pub hooks: HashMap<String, Vec<HookSpec>>,
}

// ---------------------------------------------------------------------------
// Outcomes — what a hook returns.
// ---------------------------------------------------------------------------

/// Caller-actionable verdict surfaced by a hook script. Decoded from
/// the script's stdout when it parses as JSON of shape
/// `{"decision": "block"|"allow", "reason": "..."}`. Any other stdout
/// (or an empty/non-JSON payload) maps to [`HookOutcome::Continue`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "lowercase")]
pub enum HookOutcome {
    /// Default: proceed without intervention.
    Continue,
    /// Hook actively wants the upstream operation cancelled.
    Block {
        /// Human-readable explanation surfaced to the runtime/UI.
        reason: String,
    },
    /// Hook explicitly approves the upstream operation (used by gate
    /// hooks that need to whitelist by-name).
    Allow {
        /// Human-readable explanation, mirrored back to the runtime.
        reason: String,
    },
}

impl HookOutcome {
    /// Parse a hook's stdout. Empty / non-JSON stdout => [`Self::Continue`].
    #[must_use]
    pub fn from_stdout(raw: &str) -> Self {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            return Self::Continue;
        }
        match serde_json::from_str::<Self>(trimmed) {
            Ok(out) => out,
            Err(_) => Self::Continue,
        }
    }
}

/// Per-execution result, returned by [`HookDispatcher::run_hooks`].
#[derive(Debug, Clone)]
pub struct HookExecutionResult {
    /// The literal command line that was executed.
    pub command: String,
    /// OS exit code, normalised to `-1` when the process was killed.
    pub exit_code: i32,
    /// Captured stdout (UTF-8 lossy).
    pub stdout: String,
    /// Captured stderr (UTF-8 lossy).
    pub stderr: String,
    /// Wall-clock duration of the spawn (process start to exit/kill).
    pub duration_ms: u64,
    /// Decoded outcome derived from `stdout` per [`HookOutcome::from_stdout`].
    pub outcome: HookOutcome,
}

// ---------------------------------------------------------------------------
// Dispatcher
// ---------------------------------------------------------------------------

/// Compiled, ready-to-fire hook dispatcher.
///
/// Holds a pre-compiled [`GlobMatcher`] per matcher pattern so that
/// repeated [`Self::match_hooks`] calls do not re-parse the glob.
#[derive(Debug, Default)]
pub struct HookDispatcher {
    config: HookConfig,
    /// Pre-built glob matchers, indexed `(event_kind, spec_index)` to
    /// preserve order while keeping lookups cheap.
    compiled: HashMap<String, Vec<Option<GlobMatcher>>>,
}

impl HookDispatcher {
    /// Build a dispatcher from a parsed [`HookConfig`]. Compiles every
    /// matcher glob eagerly and surfaces a single [`HookError::GlobInvalid`]
    /// if any pattern is malformed.
    pub fn new(config: HookConfig) -> Result<Self, HookError> {
        let mut compiled: HashMap<String, Vec<Option<GlobMatcher>>> = HashMap::new();
        for (kind, specs) in &config.hooks {
            let mut row = Vec::with_capacity(specs.len());
            for spec in specs {
                row.push(match &spec.matcher.matcher {
                    Some(pat) => {
                        let glob = Glob::new(pat).map_err(|source| HookError::GlobInvalid {
                            pattern: pat.clone(),
                            source,
                        })?;
                        Some(glob.compile_matcher())
                    }
                    None => None,
                });
            }
            compiled.insert(kind.clone(), row);
        }
        Ok(Self { config, compiled })
    }

    /// Load a config from a `serde_json::Value`. The dispatcher accepts
    /// both `{ "hooks": { ... } }` (a full settings document) and a
    /// bare `{ ... }` map (the `hooks` value itself) for ergonomic
    /// embedding.
    pub fn load_from_json(value: &serde_json::Value) -> Result<Self, HookError> {
        let config: HookConfig = if value.get("hooks").is_some() {
            serde_json::from_value(value.clone())?
        } else {
            HookConfig {
                hooks: serde_json::from_value(value.clone())?,
            }
        };
        Self::new(config)
    }

    /// Load a config from a JSON file on disk. Uses async IO so the
    /// dispatcher can be initialised from inside an `async fn` without
    /// blocking the runtime.
    pub async fn load_from_path(path: &Path) -> Result<Self, HookError> {
        let raw = tokio::fs::read(path).await?;
        let value: serde_json::Value = serde_json::from_slice(&raw)?;
        Self::load_from_json(&value)
    }

    /// Borrow the underlying configuration (useful for diagnostics).
    #[must_use]
    pub fn config(&self) -> &HookConfig {
        &self.config
    }

    /// Return every [`HookSpec`] that applies to `event`, in
    /// declaration order. A matcher with no `matcher` glob applies
    /// universally to its kind; otherwise the event must carry a
    /// `tool_name` matching the compiled glob.
    #[must_use]
    pub fn match_hooks(&self, event: &HookEvent) -> Vec<&HookSpec> {
        let kind = event.kind();
        let Some(specs) = self.config.hooks.get(kind) else {
            return Vec::new();
        };
        let compiled_row = self.compiled.get(kind);
        let mut out = Vec::new();
        for (idx, spec) in specs.iter().enumerate() {
            let compiled = compiled_row.and_then(|r| r.get(idx).and_then(Option::as_ref));
            let applies = match compiled {
                None => true,
                Some(matcher) => match event.tool_name() {
                    Some(name) => matcher.is_match(name),
                    None => false,
                },
            };
            if applies {
                out.push(spec);
            }
        }
        out
    }

    /// Spawn every matching hook, pipe the event JSON to its stdin,
    /// capture stdout/stderr/exit-code, enforce the per-hook timeout
    /// budget. The returned [`HookExecutionResult`] vector preserves
    /// the order matching commands were declared in.
    ///
    /// Note that individual hook failures are *captured*, not
    /// propagated — a missing binary surfaces as an `Err` only for
    /// catastrophic spawn errors (e.g. the platform shell itself is
    /// unavailable). Per-hook exit-code != 0 is reported through
    /// [`HookExecutionResult::exit_code`].
    pub async fn run_hooks(
        &self,
        event: &HookEvent,
    ) -> Result<Vec<HookExecutionResult>, HookError> {
        let mut out = Vec::new();
        let payload = serde_json::to_vec(event)?;
        for spec in self.match_hooks(event) {
            for hook in &spec.hooks {
                let result = run_one(hook, &payload).await?;
                tracing::debug!(
                    target: "aphrody_hooks",
                    command = %hook.command,
                    exit = result.exit_code,
                    duration_ms = result.duration_ms,
                    "hook completed"
                );
                out.push(result);
            }
        }
        Ok(out)
    }
}

// ---------------------------------------------------------------------------
// Process execution
// ---------------------------------------------------------------------------

async fn run_one(hook: &HookCommand, stdin_payload: &[u8]) -> Result<HookExecutionResult, HookError> {
    let started = Instant::now();
    let budget = Duration::from_millis(hook.timeout_ms.unwrap_or(DEFAULT_HOOK_TIMEOUT_MS));

    let mut cmd = build_shell_command(&hook.command);
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = cmd.spawn().map_err(|source| HookError::Spawn {
        command: hook.command.clone(),
        source,
    })?;

    if let Some(mut stdin) = child.stdin.take() {
        // Best-effort: piping to a script that closed stdin early is not
        // a hook failure; surface only catastrophic IO errors below.
        if let Err(err) = stdin.write_all(stdin_payload).await {
            tracing::debug!(target: "aphrody_hooks", error = %err, "stdin write failed (ignored)");
        }
        drop(stdin);
    }

    match timeout(budget, child.wait_with_output()).await {
        Ok(Ok(output)) => {
            let exit_code = output.status.code().unwrap_or(-1);
            let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
            let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
            let outcome = HookOutcome::from_stdout(&stdout);
            Ok(HookExecutionResult {
                command: hook.command.clone(),
                exit_code,
                stdout,
                stderr,
                duration_ms: started.elapsed().as_millis() as u64,
                outcome,
            })
        }
        Ok(Err(err)) => Err(HookError::Spawn {
            command: hook.command.clone(),
            source: err,
        }),
        Err(_elapsed) => {
            // The child is now orphaned; we cannot reliably reap it
            // here without breaking borrow rules on `child` (consumed
            // by `wait_with_output`). The OS will reclaim the process
            // group when the parent exits; for long-lived parents,
            // tools wrap the hook in their own watchdog.
            Err(HookError::Timeout {
                command: hook.command.clone(),
                elapsed_ms: started.elapsed().as_millis() as u64,
            })
        }
    }
}

#[cfg(target_os = "windows")]
fn build_shell_command(cmdline: &str) -> Command {
    let mut cmd = Command::new("cmd");
    cmd.args(["/d", "/s", "/c", cmdline]);
    cmd
}

#[cfg(not(target_os = "windows"))]
fn build_shell_command(cmdline: &str) -> Command {
    let mut cmd = Command::new("sh");
    cmd.args(["-c", cmdline]);
    cmd
}

// ---------------------------------------------------------------------------
// Unit tests (pure logic, no subprocess)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hook_outcome_continue_on_empty_stdout() {
        assert_eq!(HookOutcome::from_stdout(""), HookOutcome::Continue);
        assert_eq!(HookOutcome::from_stdout("   \n  "), HookOutcome::Continue);
    }

    #[test]
    fn hook_outcome_parses_block_payload() {
        let out = HookOutcome::from_stdout(r#"{"decision":"block","reason":"nope"}"#);
        assert_eq!(out, HookOutcome::Block { reason: "nope".into() });
    }

    #[test]
    fn hook_outcome_parses_allow_payload() {
        let out = HookOutcome::from_stdout(r#"{"decision":"allow","reason":"safe"}"#);
        assert_eq!(out, HookOutcome::Allow { reason: "safe".into() });
    }

    #[test]
    fn hook_outcome_continue_on_unrelated_json() {
        let out = HookOutcome::from_stdout(r#"{"foo":1}"#);
        assert_eq!(out, HookOutcome::Continue);
    }

    #[test]
    fn event_kind_round_trip() {
        let e = HookEvent::PreToolUse {
            tool_name: "Bash".into(),
            tool_input: serde_json::json!({"command": "ls"}),
        };
        assert_eq!(e.kind(), "PreToolUse");
        assert_eq!(e.tool_name(), Some("Bash"));

        let e = HookEvent::Stop;
        assert_eq!(e.kind(), "Stop");
        assert_eq!(e.tool_name(), None);
    }

    #[test]
    fn dispatcher_rejects_invalid_glob() {
        let mut hooks = HashMap::new();
        hooks.insert(
            "PreToolUse".to_string(),
            vec![HookSpec {
                matcher: HookMatcher { event: None, matcher: Some("[invalid".into()) },
                hooks: vec![HookCommand {
                    r#type: "command".into(),
                    command: "true".into(),
                    timeout_ms: None,
                }],
            }],
        );
        let err = HookDispatcher::new(HookConfig { hooks }).unwrap_err();
        assert!(matches!(err, HookError::GlobInvalid { .. }));
    }
}
