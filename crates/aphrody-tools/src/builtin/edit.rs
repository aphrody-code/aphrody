// SPDX-License-Identifier: Apache-2.0
//! `edit` — substring-replace inside an existing file with diff awareness.
//!
//! Rust port of `packages/gemini-cli/packages/core/src/tools/edit.ts`.
//! Implements the model-facing surface used by Gemini CLI / Claude Code:
//! exact literal replacement of `old_string` with `new_string`. By default
//! the call fails with [`ToolError::AmbiguousMatch`] if `old_string` matches
//! more than once — pass `replace_all = true` to opt into multi-replace.
//!
//! On success returns the resulting file contents, a unified diff, and the
//! post-write SHA-256 (chains naturally with the
//! [`crate::builtin::write_file`] hash field).

use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use similar::{ChangeTag, TextDiff};

use crate::builtin::write_file::sha256_hex;
use crate::{
    Permission, PermissionDescriptor, Tool, ToolDescriptor, ToolError,
};

/// Canonical tool name.
pub const NAME: &str = "edit";

/// Parsed arguments for the `edit` tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditArgs {
    /// Absolute or workspace-relative path to mutate.
    pub file_path: String,
    /// Exact substring to find. Must appear at least once. Empty string is
    /// only allowed when `replace_all = false` and the target file is
    /// empty (acts as a create-with-content).
    pub old_string: String,
    /// Replacement substring (may be empty to delete).
    pub new_string: String,
    /// When `true`, replace every occurrence. When `false` (default),
    /// reject ambiguous matches.
    #[serde(default)]
    pub replace_all: bool,
}

/// Descriptor for the `edit` tool.
#[must_use]
pub fn descriptor() -> ToolDescriptor {
    ToolDescriptor::new(
        NAME,
        "Replace `old_string` with `new_string` inside an existing file. \
         Defaults to single-occurrence replacement — pass `replace_all=true` \
         for global substitution. Returns the resulting content, a unified \
         diff and the SHA-256 of the new contents.",
        json!({
            "type": "object",
            "properties": {
                "file_path":   {"type": "string"},
                "old_string":  {"type": "string"},
                "new_string":  {"type": "string"},
                "replace_all": {"type": "boolean", "description": "Replace every occurrence (default false)."}
            },
            "required": ["file_path", "old_string", "new_string"],
            "additionalProperties": false
        }),
    )
    .expect("edit name is valid")
    .dangerous()
    .with_tag("fs")
    .with_tag("edit")
}

/// Implementation handle.
#[derive(Debug, Default, Clone, Copy)]
pub struct EditTool;

#[async_trait]
impl Tool for EditTool {
    fn descriptor(&self) -> ToolDescriptor {
        descriptor()
    }

    fn permission(&self) -> PermissionDescriptor {
        PermissionDescriptor {
            tool_name: NAME.into(),
            scopes: vec!["fs:write".into(), "fs:edit".into()],
            default_permission: Permission::Ask,
        }
    }

    async fn invoke(&self, args: Value) -> Result<Value, ToolError> {
        let parsed: EditArgs = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidArgs(e.to_string()))?;
        let path = PathBuf::from(&parsed.file_path);

        let original = tokio::fs::read_to_string(&path).await?;
        let count = if parsed.old_string.is_empty() {
            0
        } else {
            original.matches(&parsed.old_string).count()
        };

        let (new_content, replacements) = if parsed.old_string.is_empty() {
            // Empty old_string only allowed when the file is empty
            // (parity with TS surface acting as create-with-content).
            if !original.is_empty() {
                return Err(ToolError::InvalidArgs(
                    "old_string is empty but file is non-empty".into(),
                ));
            }
            (parsed.new_string.clone(), 1usize)
        } else if count == 0 {
            return Err(ToolError::NotFound(parsed.old_string.clone()));
        } else if count > 1 && !parsed.replace_all {
            return Err(ToolError::AmbiguousMatch {
                count,
                needle: parsed.old_string.clone(),
            });
        } else if parsed.replace_all {
            (
                original.replace(&parsed.old_string, &parsed.new_string),
                count,
            )
        } else {
            // Single replacement — replacen guarantees only the first match
            // is rewritten.
            (
                original.replacen(&parsed.old_string, &parsed.new_string, 1),
                1,
            )
        };

        // Persist atomically. We reuse the helper via a fresh write to
        // avoid bringing the public `WriteFileTool` async boundary into
        // the edit call's hot path (no extra schema round-trip).
        crate::builtin::write_file::WriteFileTool
            .invoke(json!({
                "file_path": path.display().to_string(),
                "content":   new_content,
            }))
            .await?;

        let diff = unified_diff(&original, &new_content, &path);
        let sha = sha256_hex(new_content.as_bytes());

        Ok(json!({
            "path":         path.display().to_string(),
            "content":      new_content,
            "diff":         diff,
            "sha256":       sha,
            "replacements": replacements,
        }))
    }
}

/// Build a textual unified diff (3-line context) between `before` and
/// `after`. Suitable for surfacing back to the model.
fn unified_diff(
    before: &str,
    after: &str,
    path: &std::path::Path,
) -> String {
    let diff = TextDiff::from_lines(before, after);
    let p = path.display();
    let mut out = String::new();
    out.push_str(&format!("--- {p}\n"));
    out.push_str(&format!("+++ {p}\n"));
    for hunk in diff.unified_diff().context_radius(3).iter_hunks() {
        out.push_str(&hunk.header().to_string());
        out.push('\n');
        for change in hunk.iter_changes() {
            let sign = match change.tag() {
                ChangeTag::Delete => '-',
                ChangeTag::Insert => '+',
                ChangeTag::Equal => ' ',
            };
            out.push(sign);
            out.push_str(change.value().trim_end_matches('\n'));
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn invoke_replaces_single_occurrence() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("src.txt");
        tokio::fs::write(&path, "hello world\nbye\n").await.unwrap();
        let tool = EditTool;
        let v = tool
            .invoke(json!({
                "file_path":  path.display().to_string(),
                "old_string": "world",
                "new_string": "rustaceans"
            }))
            .await
            .unwrap();
        assert_eq!(v.get("replacements").and_then(Value::as_u64), Some(1));
        let actual = tokio::fs::read_to_string(&path).await.unwrap();
        assert_eq!(actual, "hello rustaceans\nbye\n");
    }

    #[tokio::test]
    async fn invoke_rejects_ambiguous_without_replace_all() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("dup.txt");
        tokio::fs::write(&path, "foo foo foo").await.unwrap();
        let tool = EditTool;
        let err = tool
            .invoke(json!({
                "file_path":  path.display().to_string(),
                "old_string": "foo",
                "new_string": "bar"
            }))
            .await
            .expect_err("must fail on ambiguous");
        assert!(matches!(
            err,
            ToolError::AmbiguousMatch { count: 3, .. }
        ));
    }

    #[tokio::test]
    async fn invoke_replace_all_works() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("all.txt");
        tokio::fs::write(&path, "foo foo foo").await.unwrap();
        let tool = EditTool;
        let v = tool
            .invoke(json!({
                "file_path":   path.display().to_string(),
                "old_string":  "foo",
                "new_string":  "bar",
                "replace_all": true
            }))
            .await
            .unwrap();
        assert_eq!(v.get("replacements").and_then(Value::as_u64), Some(3));
        let actual = tokio::fs::read_to_string(&path).await.unwrap();
        assert_eq!(actual, "bar bar bar");
    }

    #[tokio::test]
    async fn invoke_emits_unified_diff() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("d.txt");
        tokio::fs::write(&path, "line a\nline b\nline c\n").await.unwrap();
        let tool = EditTool;
        let v = tool
            .invoke(json!({
                "file_path":  path.display().to_string(),
                "old_string": "line b",
                "new_string": "LINE B!"
            }))
            .await
            .unwrap();
        let diff = v.get("diff").and_then(Value::as_str).unwrap();
        assert!(diff.contains("-line b"));
        assert!(diff.contains("+LINE B!"));
    }

    #[tokio::test]
    async fn invoke_errors_when_old_string_absent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("e.txt");
        tokio::fs::write(&path, "abc").await.unwrap();
        let tool = EditTool;
        let err = tool
            .invoke(json!({
                "file_path":  path.display().to_string(),
                "old_string": "xyz",
                "new_string": "_"
            }))
            .await
            .expect_err("must fail");
        assert!(matches!(err, ToolError::NotFound(_)));
    }
}
