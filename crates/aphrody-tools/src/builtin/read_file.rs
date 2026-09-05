// SPDX-License-Identifier: Apache-2.0
//! `read_file` — read a UTF-8 file from disk with optional line slicing.
//!
//! Rust port of `packages/gemini-cli/packages/core/src/tools/read-file.ts`.
//! Schema mirrors the TS surface (`file_path`, `start_line`, `end_line`)
//! 1-based and inclusive. Adds a `limit` byte-count safety cap (default
//! 1 MiB) to avoid streaming gigabyte logs back to the model.

use std::path::{Path, PathBuf};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::io::AsyncReadExt;

use crate::{
    Permission, PermissionDescriptor, Tool, ToolDescriptor, ToolError,
};

/// Canonical tool name exposed to the model.
pub const NAME: &str = "read_file";

/// Default cap on the number of bytes returned to the caller (1 MiB).
pub const DEFAULT_BYTE_LIMIT: usize = 1 << 20;

/// Parsed arguments for the `read_file` tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadFileArgs {
    /// Absolute or workspace-relative path to read.
    pub file_path: String,
    /// 1-based start line, inclusive. Defaults to 1.
    #[serde(default)]
    pub start_line: Option<usize>,
    /// 1-based end line, inclusive. Defaults to end of file.
    #[serde(default)]
    pub end_line: Option<usize>,
    /// Hard byte cap on the returned payload. Defaults to
    /// [`DEFAULT_BYTE_LIMIT`].
    #[serde(default)]
    pub limit: Option<usize>,
}

/// Descriptor for the `read_file` tool.
#[must_use]
pub fn descriptor() -> ToolDescriptor {
    ToolDescriptor::new(
        NAME,
        "Read a UTF-8 text file from the filesystem. Returns the file \
         contents with 1-based line numbers. Optional `start_line` / \
         `end_line` arguments slice the file (inclusive). Long files are \
         truncated to `limit` bytes (default 1 MiB).",
        json!({
            "type": "object",
            "properties": {
                "file_path": {"type": "string", "description": "Absolute path or workspace-relative path."},
                "start_line": {"type": "integer", "minimum": 1, "description": "Optional 1-based first line."},
                "end_line":   {"type": "integer", "minimum": 1, "description": "Optional 1-based last line (inclusive)."},
                "limit":      {"type": "integer", "minimum": 1, "description": "Optional byte cap (default 1 MiB)."}
            },
            "required": ["file_path"],
            "additionalProperties": false
        }),
    )
    .expect("read_file name is valid")
    .with_tag("fs")
    .with_tag("read")
}

/// Implementation handle.
#[derive(Debug, Default, Clone, Copy)]
pub struct ReadFileTool;

#[async_trait]
impl Tool for ReadFileTool {
    fn descriptor(&self) -> ToolDescriptor {
        descriptor()
    }

    fn permission(&self) -> PermissionDescriptor {
        PermissionDescriptor {
            tool_name: NAME.into(),
            scopes: vec!["fs:read".into()],
            default_permission: Permission::Allow,
        }
    }

    async fn invoke(&self, args: Value) -> Result<Value, ToolError> {
        let parsed: ReadFileArgs = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidArgs(e.to_string()))?;
        let path = PathBuf::from(&parsed.file_path);
        let limit = parsed.limit.unwrap_or(DEFAULT_BYTE_LIMIT).max(1);

        let (text, truncated, total_bytes) = read_capped(&path, limit).await?;

        let total_lines = text.lines().count();
        let start = parsed.start_line.unwrap_or(1).max(1);
        let end = parsed.end_line.unwrap_or(total_lines).max(start);

        // Slice by line range (1-based inclusive).
        let sliced: String = text
            .lines()
            .skip(start.saturating_sub(1))
            .take(end.saturating_sub(start) + 1)
            .collect::<Vec<_>>()
            .join("\n");

        let returned_lines = sliced.lines().count();
        Ok(json!({
            "path":          path.display().to_string(),
            "content":       sliced,
            "start_line":    start,
            "end_line":      start + returned_lines.saturating_sub(1),
            "total_lines":   total_lines,
            "total_bytes":   total_bytes,
            "truncated":     truncated,
        }))
    }
}

/// Read up to `limit` bytes from `path` and decode as UTF-8 (lossy).
///
/// Returns `(text, truncated, total_bytes_observed)`. `truncated` is `true`
/// when the file is larger than `limit`.
async fn read_capped(
    path: &Path,
    limit: usize,
) -> Result<(String, bool, u64), ToolError> {
    let metadata = tokio::fs::metadata(path).await?;
    if metadata.is_dir() {
        return Err(ToolError::PathNotAllowed(format!(
            "{} is a directory",
            path.display()
        )));
    }
    let total = metadata.len();
    let mut file = tokio::fs::File::open(path).await?;
    let read_target = std::cmp::min(total as usize, limit);
    let mut buf = Vec::with_capacity(read_target);
    let mut handle = (&mut file).take(read_target as u64);
    handle.read_to_end(&mut buf).await?;
    let text = String::from_utf8_lossy(&buf).into_owned();
    Ok((text, (total as usize) > limit, total))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn descriptor_round_trips() {
        let d = descriptor();
        assert_eq!(d.name, NAME);
        assert!(d.input_schema.get("properties").is_some());
    }

    #[tokio::test]
    async fn invoke_reads_full_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hello.txt");
        tokio::fs::write(&path, "alpha\nbeta\ngamma\n").await.unwrap();
        let tool = ReadFileTool;
        let v = tool
            .invoke(json!({"file_path": path.display().to_string()}))
            .await
            .unwrap();
        let content = v.get("content").and_then(Value::as_str).unwrap();
        assert!(content.contains("alpha"));
        assert!(content.contains("gamma"));
        assert_eq!(v.get("total_lines").and_then(Value::as_u64), Some(3));
    }

    #[tokio::test]
    async fn invoke_slices_by_line_range() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("lines.txt");
        tokio::fs::write(&path, "L1\nL2\nL3\nL4\nL5\n").await.unwrap();
        let tool = ReadFileTool;
        let v = tool
            .invoke(json!({
                "file_path": path.display().to_string(),
                "start_line": 2,
                "end_line": 4
            }))
            .await
            .unwrap();
        let content = v.get("content").and_then(Value::as_str).unwrap();
        assert_eq!(content, "L2\nL3\nL4");
    }

    #[tokio::test]
    async fn invoke_errors_on_missing_file() {
        let tool = ReadFileTool;
        let err = tool
            .invoke(json!({"file_path": "/this/path/should/not/exist.bin"}))
            .await
            .expect_err("must fail");
        assert!(matches!(err, ToolError::Io(_)));
    }

    #[tokio::test]
    async fn invoke_truncates_large_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("big.txt");
        // 8 KiB payload, request 1 KiB cap.
        let blob = "x".repeat(8 * 1024);
        tokio::fs::write(&path, &blob).await.unwrap();
        let tool = ReadFileTool;
        let v = tool
            .invoke(json!({
                "file_path": path.display().to_string(),
                "limit": 1024
            }))
            .await
            .unwrap();
        assert_eq!(v.get("truncated").and_then(Value::as_bool), Some(true));
        assert_eq!(
            v.get("total_bytes").and_then(Value::as_u64),
            Some(8 * 1024)
        );
    }
}
