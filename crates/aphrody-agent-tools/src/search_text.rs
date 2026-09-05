// SPDX-License-Identifier: Apache-2.0
//! Structured, non-shell repository search for local agent data-mining.

use std::collections::BTreeMap;
use std::path::PathBuf;

use aphrody_toolcall::{AdditionalProperties, JsonSchema, ToolDefinition, ToolError, ToolExecutor, ToolOutput};
use serde::Deserialize;

const MAX_OUTPUT_BYTES: usize = 128 * 1024;

#[derive(Debug, Deserialize)]
struct SearchArgs {
    pattern: String,
    #[serde(default)]
    path: Option<PathBuf>,
    #[serde(default)]
    glob: Option<String>,
    #[serde(default)]
    cwd: Option<PathBuf>,
}

/// Searches local text files with ripgrep's argv interface, never a shell.
pub struct SearchTextTool {
    definition: ToolDefinition,
}

impl SearchTextTool {
    #[must_use]
    pub fn new() -> Self { Self { definition: definition() } }
}

impl Default for SearchTextTool {
    fn default() -> Self { Self::new() }
}

#[async_trait::async_trait]
impl ToolExecutor for SearchTextTool {
    fn definition(&self) -> &ToolDefinition { &self.definition }

    async fn handle(&self, arguments: serde_json::Value) -> Result<ToolOutput, ToolError> {
        let args: SearchArgs = serde_json::from_value(arguments).map_err(|error| ToolError::InvalidArguments {
            tool: "search_text".into(), message: error.to_string(),
        })?;
        if args.pattern.trim().is_empty() {
            return Ok(ToolOutput::error("search_text: pattern must not be empty"));
        }
        let mut command = tokio::process::Command::new("rg");
        command.args(["--line-number", "--color", "never", "--hidden", "--glob", "!.git"]);
        if let Some(glob) = args.glob { command.args(["--glob", &glob]); }
        command.current_dir(args.cwd.as_deref().unwrap_or_else(|| std::path::Path::new(".")));
        command.arg(&args.pattern).arg(args.path.as_deref().unwrap_or_else(|| std::path::Path::new(".")));
        let output = command.output().await.map_err(|error| ToolError::Execution {
            tool: "search_text".into(), message: format!("failed to start rg: {error}"),
        })?;
        let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
        if text.len() > MAX_OUTPUT_BYTES { text.truncate(MAX_OUTPUT_BYTES); text.push_str("\n[output truncated]"); }
        if output.status.success() || output.status.code() == Some(1) { Ok(ToolOutput::ok(text)) } else {
            Ok(ToolOutput::error(format!("search_text: rg failed: {}", String::from_utf8_lossy(&output.stderr))))
        }
    }
}

fn definition() -> ToolDefinition {
    let mut properties = BTreeMap::new();
    properties.insert("pattern".into(), JsonSchema::string(Some("Regex pattern to find.".into())));
    properties.insert("path".into(), JsonSchema::string(Some("Optional file or directory root.".into())));
    properties.insert("glob".into(), JsonSchema::string(Some("Optional file glob, for example **/*.rs.".into())));
    properties.insert("cwd".into(), JsonSchema::string(Some("Optional workspace root for relative searches.".into())));
    ToolDefinition::new(
        "search_text",
        "Search local repository text with ripgrep and return bounded line-numbered matches for data mining.",
        JsonSchema::object(properties, Some(vec!["pattern".into()]), Some(AdditionalProperties::Boolean(false))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use aphrody_toolcall::ToolExecutor;
    use serde_json::json;

    #[tokio::test]
    async fn searches_with_cwd_and_glob() {
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::write(dir.path().join("one.rs"), "needle here").await.unwrap();
        tokio::fs::write(dir.path().join("two.txt"), "needle elsewhere").await.unwrap();
        let output = SearchTextTool::new().handle(json!({ "pattern": "needle", "cwd": dir.path(), "glob": "*.rs" })).await.unwrap();
        assert!(!output.is_error, "{}", output.content);
        assert!(output.content.contains("one.rs"));
        assert!(!output.content.contains("two.txt"));
    }

    #[tokio::test]
    async fn empty_pattern_is_rejected() {
        let output = SearchTextTool::new().handle(json!({"pattern": "  "})).await.unwrap();
        assert!(output.is_error);
        assert!(output.content.contains("must not be empty"));
    }
}
