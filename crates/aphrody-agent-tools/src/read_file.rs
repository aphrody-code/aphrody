// SPDX-License-Identifier: Apache-2.0
//! Local bounded file reader for agent inspection and data-mining workflows.

use std::collections::BTreeMap;
use std::path::PathBuf;

use aphrody_toolcall::{AdditionalProperties, JsonSchema, ToolDefinition, ToolError, ToolExecutor, ToolOutput};
use serde::Deserialize;

const MAX_READ_BYTES: usize = 512 * 1024;

#[derive(Debug, Deserialize)]
struct ReadArgs {
    file_path: PathBuf,
    #[serde(default)]
    cwd: Option<PathBuf>,
}

/// Reads UTF-8 text from a local path with a bounded payload.
pub struct ReadFileTool {
    definition: ToolDefinition,
}

impl ReadFileTool {
    #[must_use]
    pub fn new() -> Self {
        Self { definition: definition() }
    }
}

impl Default for ReadFileTool {
    fn default() -> Self { Self::new() }
}

#[async_trait::async_trait]
impl ToolExecutor for ReadFileTool {
    fn definition(&self) -> &ToolDefinition { &self.definition }

    async fn handle(&self, arguments: serde_json::Value) -> Result<ToolOutput, ToolError> {
        let args: ReadArgs = serde_json::from_value(arguments).map_err(|error| ToolError::InvalidArguments {
            tool: "read_file".into(), message: error.to_string(),
        })?;
        let path = args.cwd.as_deref().map_or_else(|| args.file_path.clone(), |cwd| cwd.join(&args.file_path));
        let bytes = tokio::fs::read(&path).await.map_err(|error| ToolError::Execution {
            tool: "read_file".into(), message: format!("{}: {error}", args.file_path.display()),
        })?;
        if bytes.len() > MAX_READ_BYTES {
            return Ok(ToolOutput::error(format!(
                "read_file: {} is {} bytes; limit is {MAX_READ_BYTES}",
                path.display(), bytes.len()
            )));
        }
        let text = String::from_utf8(bytes).map_err(|error| ToolError::Execution {
            tool: "read_file".into(), message: format!("{} is not UTF-8: {error}", path.display()),
        })?;
        Ok(ToolOutput::ok(text))
    }
}

fn definition() -> ToolDefinition {
    let mut properties = BTreeMap::new();
    properties.insert("file_path".into(), JsonSchema::string(Some(
        "Local UTF-8 file path to inspect. Keep reads focused and bounded.".into(),
    )));
    properties.insert("cwd".into(), JsonSchema::string(Some("Optional workspace root for relative paths.".into())));
    ToolDefinition::new(
        "read_file",
        "Read a bounded local UTF-8 file for repository inspection, RAG preparation, or data mining.",
        JsonSchema::object(properties, Some(vec!["file_path".into()]), Some(AdditionalProperties::Boolean(false))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use aphrody_toolcall::ToolExecutor;
    use serde_json::json;

    #[tokio::test]
    async fn reads_relative_path_from_cwd() {
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::write(dir.path().join("sample.txt"), "local evidence").await.unwrap();
        let output = ReadFileTool::new().handle(json!({ "file_path": "sample.txt", "cwd": dir.path() })).await.unwrap();
        assert_eq!(output.content, "local evidence");
        assert!(!output.is_error);
    }

    #[tokio::test]
    async fn rejects_oversized_payload() {
        let dir = tempfile::tempdir().unwrap();
        tokio::fs::write(dir.path().join("large.txt"), vec![b'x'; MAX_READ_BYTES + 1]).await.unwrap();
        let output = ReadFileTool::new().handle(json!({ "file_path": "large.txt", "cwd": dir.path() })).await.unwrap();
        assert!(output.is_error);
        assert!(output.content.contains("limit"));
    }

    #[tokio::test]
    async fn rejects_missing_required_path() {
        let error = ReadFileTool::new().handle(json!({})).await.unwrap_err();
        assert!(matches!(error, ToolError::InvalidArguments { .. }));
    }
}
