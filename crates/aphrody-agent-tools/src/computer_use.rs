// SPDX-License-Identifier: Apache-2.0
//! Computer-use bridge to the local `agent-browser` CLI.

use std::collections::BTreeMap;

use aphrody_toolcall::{AdditionalProperties, JsonSchema, ToolDefinition, ToolError, ToolExecutor, ToolOutput};
use serde::Deserialize;

const MAX_OUTPUT_BYTES: usize = 128 * 1024;

#[derive(Debug, Deserialize)]
struct ComputerArgs {
    action: String,
    #[serde(default)]
    target: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default, rename = "cwd")]
    _cwd: Option<std::path::PathBuf>,
}

/// Runs explicit computer-use actions through the local browser session.
pub struct ComputerUseTool {
    definition: ToolDefinition,
}

impl ComputerUseTool {
    #[must_use]
    pub fn new() -> Self { Self { definition: definition() } }
}

impl Default for ComputerUseTool {
    fn default() -> Self { Self::new() }
}

#[async_trait::async_trait]
impl ToolExecutor for ComputerUseTool {
    fn definition(&self) -> &ToolDefinition { &self.definition }

    async fn handle(&self, arguments: serde_json::Value) -> Result<ToolOutput, ToolError> {
        let args: ComputerArgs = serde_json::from_value(arguments).map_err(|error| ToolError::InvalidArguments {
            tool: "computer_use".into(), message: error.to_string(),
        })?;
        let mut argv = Vec::new();
        match args.action.as_str() {
            "navigate" => { argv.extend(["open".into(), required(args.target, "url")?]); }
            "snapshot" => argv.push("snapshot".into()),
            "screenshot" => argv.push("screenshot".into()),
            "click" => { argv.extend(["click".into(), required(args.target, "target")?]); }
            "type" => { argv.extend(["fill".into(), required(args.target, "target")?, required(args.text, "text")?]); }
            action => return Ok(ToolOutput::error(format!("computer_use: unsupported action `{action}`"))),
        }
        let binary = std::env::var("APHRODY_AGENT_BROWSER_BIN").unwrap_or_else(|_| "agent-browser".into());
        let output = tokio::process::Command::new(binary).args(argv).output().await.map_err(|error| ToolError::Execution {
            tool: "computer_use".into(), message: format!("agent-browser unavailable: {error}"),
        })?;
        let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
        if text.len() > MAX_OUTPUT_BYTES { text.truncate(MAX_OUTPUT_BYTES); text.push_str("\n[output truncated]"); }
        if output.status.success() { Ok(ToolOutput::ok(text)) } else {
            Ok(ToolOutput::error(format!("computer_use: {}", String::from_utf8_lossy(&output.stderr))))
        }
    }
}

fn required(value: Option<String>, name: &str) -> Result<String, ToolError> {
    value.filter(|value| !value.trim().is_empty()).ok_or_else(|| ToolError::InvalidArguments {
        tool: "computer_use".into(), message: format!("{name} is required for this action"),
    })
}

fn definition() -> ToolDefinition {
    let mut properties = BTreeMap::new();
    properties.insert("action".into(), JsonSchema::string(Some("navigate, snapshot, screenshot, click, or type.".into())));
    properties.insert("target".into(), JsonSchema::string(Some("URL or accessibility element reference such as @e1.".into())));
    properties.insert("text".into(), JsonSchema::string(Some("Text for the type action.".into())));
    properties.insert("cwd".into(), JsonSchema::string(Some("Optional workspace context for the browser session.".into())));
    ToolDefinition::new(
        "computer_use",
        "Perform an explicit local browser computer-use action through agent-browser; no shell is interpreted.",
        JsonSchema::object(properties, Some(vec!["action".into()]), Some(AdditionalProperties::Boolean(false))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use aphrody_toolcall::ToolExecutor;
    use serde_json::json;

    #[test]
    fn definition_lists_explicit_actions() {
        let tool = ComputerUseTool::new();
        assert_eq!(tool.definition().name, "computer_use");
        assert!(tool.definition().description.contains("explicit"));
    }

    #[tokio::test]
async fn rejects_missing_action_arguments() {
        let error = ComputerUseTool::new()
            .handle(json!({"action": "click"}))
            .await
            .unwrap_err();
        assert!(matches!(error, ToolError::InvalidArguments { .. }));
        assert!(error.to_string().contains("target is required"));
    }

    #[tokio::test]
    async fn rejects_unknown_action_without_spawning() {
        let output = ComputerUseTool::new().handle(json!({"action": "delete_everything"})).await.unwrap();
        assert!(output.is_error);
        assert!(output.content.contains("unsupported action"));
    }
}
