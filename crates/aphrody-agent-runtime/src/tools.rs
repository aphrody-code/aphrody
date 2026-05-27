// SPDX-License-Identifier: Apache-2.0
//! Tool wiring: build a [`ToolRegistry`] preloaded with the agent's real
//! capabilities (shell-exec + apply-patch).

use std::path::PathBuf;
use std::sync::Arc;

use aphrody_agent_tools::{ApplyPatchTool, ShellExecTool};
use aphrody_toolcall::{
    ToolDefinition, ToolError, ToolExecutor, ToolOutput, ToolRegistry,
};
use serde_json::Value;

use crate::config::RuntimeConfig;

/// A [`ToolExecutor`] decorator that injects a default `cwd` argument when a
/// tool call omits one.
///
/// The engine dispatches a tool call by handing its raw JSON arguments straight
/// to the executor; it never rewrites them. Both [`ShellExecTool`] and
/// [`ApplyPatchTool`] honor a per-call `cwd` field, so this wrapper is how the
/// runtime's [`RuntimeConfig::cwd`](crate::RuntimeConfig::cwd) becomes effective:
/// if the model did not specify a working directory, we fill in the configured
/// one before delegating. A `cwd` the model *did* provide always wins.
struct CwdDefaulting {
    inner: Arc<dyn ToolExecutor>,
    cwd: PathBuf,
}

#[async_trait::async_trait]
impl ToolExecutor for CwdDefaulting {
    fn definition(&self) -> &ToolDefinition {
        self.inner.definition()
    }

    async fn handle(&self, mut arguments: Value) -> Result<ToolOutput, ToolError> {
        if let Value::Object(map) = &mut arguments {
            let needs_default = match map.get("cwd") {
                None | Some(Value::Null) => true,
                Some(Value::String(s)) => s.is_empty(),
                Some(_) => false,
            };
            if needs_default {
                map.insert(
                    "cwd".to_string(),
                    Value::String(self.cwd.to_string_lossy().into_owned()),
                );
            }
        }
        self.inner.handle(arguments).await
    }
}

/// Wrap `inner` so a missing/empty `cwd` argument defaults to `cwd`.
fn with_default_cwd(inner: Arc<dyn ToolExecutor>, cwd: Option<&PathBuf>) -> Arc<dyn ToolExecutor> {
    match cwd {
        Some(cwd) => Arc::new(CwdDefaulting {
            inner,
            cwd: cwd.clone(),
        }),
        None => inner,
    }
}

/// Build a [`ToolRegistry`] preloaded with the agent's production tools.
///
/// The registry contains:
///
/// - `shell` — [`ShellExecTool`] bounded by the config's
///   [`ShellGuardrails`](crate::ShellGuardrails) (timeout / output cap). The
///   command is run directly (no shell interpretation).
/// - `apply_patch` — [`ApplyPatchTool`] applying patches to real disk.
///
/// When [`RuntimeConfig::cwd`](crate::RuntimeConfig::cwd) is set, both tools are
/// wrapped so a tool call that omits `cwd` defaults to it.
///
/// Both tools are permissive by default (no allow/deny-listing, no path
/// jailing) and do not depend on `aphrody-guard`; the only guardrails are the
/// opt-in shell timeout and output cap.
///
/// # Errors
///
/// Returns [`ToolError`] if two tools collide on a name (they never do here, so
/// this only guards against a future addition).
pub fn build_tool_registry(config: &RuntimeConfig) -> Result<Arc<ToolRegistry>, ToolError> {
    let mut registry = ToolRegistry::new();

    let shell: Arc<dyn ToolExecutor> =
        Arc::new(ShellExecTool::with_config(config.shell.to_shell_config()));
    registry.register(with_default_cwd(shell, config.cwd.as_ref()))?;

    let patch: Arc<dyn ToolExecutor> = Arc::new(ApplyPatchTool::new());
    registry.register(with_default_cwd(patch, config.cwd.as_ref()))?;

    Ok(Arc::new(registry))
}
