// SPDX-License-Identifier: Apache-2.0
//! Tool surface — re-exports + ergonomic builder helpers.
//!
//! This module is the SDK consumer's entry point for declaring tools. The
//! heavy lifting (registry, JSON-schema validation, cross-vendor exporters)
//! lives in [`aphrody_tools`]; we re-export the canonical types and add a
//! small wrapper over the `ToolDescriptor` builder that ties into our
//! [`SessionContext`].
//!
//! TS provenance: `packages/gemini-cli/packages/sdk/src/tool.ts`.

use std::sync::Arc;

use serde_json::Value;

pub use aphrody_tools::{ToolDescriptor, ToolRegistry, ToolsError};

use crate::types::{SdkError, SdkResult, SessionContext};

// ---------------------------------------------------------------------------
// Tool action
// ---------------------------------------------------------------------------

/// Action function executed when a registered tool is invoked.
///
/// Mirrors the TS `Tool.action: (params, context?) => Promise<unknown>` shape.
/// Returns a JSON `Value` that the agent serializes back to the model.
///
/// PUBLIC API — semver stable from v1.0.
pub type ToolAction = Arc<
    dyn Fn(
            Value,
            Option<SessionContext>,
        )
            -> std::pin::Pin<Box<dyn std::future::Future<Output = SdkResult<Value>> + Send>>
        + Send
        + Sync,
>;

// ---------------------------------------------------------------------------
// Tool (descriptor + action pair)
// ---------------------------------------------------------------------------

/// One callable tool: descriptor metadata + executable action.
///
/// PUBLIC API — semver stable from v1.0.
#[derive(Clone)]
pub struct Tool {
    /// Vendor-neutral descriptor (name, description, JSON schema, tags).
    pub descriptor: ToolDescriptor,
    /// Async action executed when the model invokes the tool.
    pub action: ToolAction,
    /// When `true`, errors raised by `action` are forwarded to the model as
    /// part of the conversation (mirrors the TS `sendErrorsToModel` flag).
    pub send_errors_to_model: bool,
}

impl std::fmt::Debug for Tool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Tool")
            .field("descriptor", &self.descriptor)
            .field("send_errors_to_model", &self.send_errors_to_model)
            .finish_non_exhaustive()
    }
}

impl Tool {
    /// Build a tool from a descriptor + action.
    ///
    /// PUBLIC API — semver stable from v1.0.
    #[must_use]
    pub fn new(descriptor: ToolDescriptor, action: ToolAction) -> Self {
        Self {
            descriptor,
            action,
            send_errors_to_model: false,
        }
    }

    /// Chainable: opt into forwarding action errors to the model.
    ///
    /// PUBLIC API — semver stable from v1.0.
    #[must_use]
    pub fn send_errors_to_model(mut self, on: bool) -> Self {
        self.send_errors_to_model = on;
        self
    }

    /// Execute the tool's action with the supplied arguments + context.
    ///
    /// # Errors
    /// Propagates the action's error verbatim. When `send_errors_to_model`
    /// is `true`, callers should still serialize the error into a model-
    /// visible JSON payload rather than aborting the turn loop.
    pub async fn invoke(
        &self,
        args: Value,
        context: Option<SessionContext>,
    ) -> SdkResult<Value> {
        (self.action)(args, context).await
    }
}

// ---------------------------------------------------------------------------
// tool() helper — sugar for the common case
// ---------------------------------------------------------------------------

/// Build a [`Tool`] from a name + description + schema + async closure.
///
/// Equivalent to the TS `tool({name, description, inputSchema}, async (...) =>
/// {...})` helper, sized for the most common call shape.
///
/// # Errors
/// Returns [`SdkError::Tools`] when the descriptor cannot be constructed
/// (invalid name or schema).
///
/// PUBLIC API — semver stable from v1.0.
pub fn tool<F, Fut>(
    name: impl Into<String>,
    description: impl Into<String>,
    input_schema: Value,
    action: F,
) -> SdkResult<Tool>
where
    F: Fn(Value, Option<SessionContext>) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = SdkResult<Value>> + Send + 'static,
{
    let descriptor = ToolDescriptor::new(name, description, input_schema)
        .map_err(SdkError::Tools)?;
    let action: ToolAction = Arc::new(move |v, ctx| Box::pin(action(v, ctx)));
    Ok(Tool::new(descriptor, action))
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn tool_helper_round_trips() {
        let t = tool(
            "echo",
            "Echo the input string",
            json!({"type": "object", "properties": {"msg": {"type": "string"}}, "required": ["msg"]}),
            |args, _ctx| async move {
                let msg = args
                    .get("msg")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                Ok(json!({"echo": msg}))
            },
        )
        .expect("tool builder must succeed");

        let out = t
            .invoke(json!({"msg": "hi"}), None)
            .await
            .expect("invoke");
        assert_eq!(out, json!({"echo": "hi"}));
        assert_eq!(t.descriptor.name, "echo");
    }

    #[test]
    fn tool_descriptor_re_exported() {
        // Sanity: confirm consumers can use `ToolDescriptor` straight from
        // this module (re-exported from `aphrody-tools`).
        let d = ToolDescriptor::new(
            "ping",
            "ping the system",
            json!({"type": "object", "properties": {}, "required": []}),
        )
        .expect("descriptor");
        assert_eq!(d.name, "ping");
    }
}
