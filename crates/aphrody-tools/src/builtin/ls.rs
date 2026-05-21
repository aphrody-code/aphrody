// SPDX-License-Identifier: Apache-2.0
//! `ls` — directory listing with optional recursion.
//!
//! Rust port of `packages/gemini-cli/packages/core/src/tools/ls.ts`. Backed
//! by [`walkdir`]. Returns a list of entries sorted lexicographically
//! (directories first), each carrying `{name, path, is_dir, size}`.

use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    Permission, PermissionDescriptor, Tool, ToolDescriptor, ToolError,
};

/// Canonical tool name.
pub const NAME: &str = "ls";

/// Default cap on the number of entries returned (avoid blowing the model
/// context window on a 100k-entry node_modules sub-tree).
pub const DEFAULT_MAX_ENTRIES: usize = 500;

/// Parsed arguments for the `ls` tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LsArgs {
    /// Directory to list. Defaults to the process working directory.
    #[serde(default)]
    pub path: Option<String>,
    /// When `true`, recurse into subdirectories. Defaults to `false`.
    #[serde(default)]
    pub recursive: bool,
    /// Max recursion depth when `recursive == true`. Defaults to 4.
    #[serde(default)]
    pub max_depth: Option<usize>,
    /// Max entries returned. Defaults to [`DEFAULT_MAX_ENTRIES`].
    #[serde(default)]
    pub max_entries: Option<usize>,
    /// Optional list of glob patterns to ignore (e.g. `["target/*"]`).
    #[serde(default)]
    pub ignore: Vec<String>,
}

/// One entry returned by the tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LsEntry {
    /// File or directory name.
    pub name: String,
    /// Absolute path.
    pub path: String,
    /// `true` when the entry is a directory.
    pub is_dir: bool,
    /// Size in bytes (0 for directories).
    pub size: u64,
}

/// Descriptor for the `ls` tool.
#[must_use]
pub fn descriptor() -> ToolDescriptor {
    ToolDescriptor::new(
        NAME,
        "List the contents of a directory. Set `recursive=true` to descend \
         (capped by `max_depth`, default 4). Returns entries sorted with \
         directories first, lexicographic within each group.",
        json!({
            "type": "object",
            "properties": {
                "path":         {"type": "string"},
                "recursive":    {"type": "boolean"},
                "max_depth":    {"type": "integer", "minimum": 1},
                "max_entries":  {"type": "integer", "minimum": 1},
                "ignore":       {"type": "array", "items": {"type": "string"}}
            },
            "required": [],
            "additionalProperties": false
        }),
    )
    .expect("ls name is valid")
    .with_tag("fs")
    .with_tag("read")
}

/// Implementation handle.
#[derive(Debug, Default, Clone, Copy)]
pub struct LsTool;

#[async_trait]
impl Tool for LsTool {
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
        let parsed: LsArgs = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidArgs(e.to_string()))?;
        let root = parsed
            .path
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        let max_entries = parsed.max_entries.unwrap_or(DEFAULT_MAX_ENTRIES);
        let max_depth = parsed.max_depth.unwrap_or(4);
        let recursive = parsed.recursive;
        let ignore = parsed.ignore.clone();

        let entries = tokio::task::spawn_blocking(move || {
            list_dir(&root, recursive, max_depth, max_entries, &ignore)
        })
        .await
        .map_err(|e| ToolError::Other(format!("join error: {e}")))??;

        Ok(json!({
            "path":    parsed.path.unwrap_or_else(|| ".".into()),
            "count":   entries.len(),
            "entries": entries,
        }))
    }
}

fn list_dir(
    root: &std::path::Path,
    recursive: bool,
    max_depth: usize,
    max_entries: usize,
    ignore_patterns: &[String],
) -> Result<Vec<LsEntry>, ToolError> {
    if !root.exists() {
        return Err(ToolError::Io(format!(
            "no such path: {}",
            root.display()
        )));
    }
    if !root.is_dir() {
        return Err(ToolError::PathNotAllowed(format!(
            "{} is not a directory",
            root.display()
        )));
    }

    // Build glob ignore set once.
    let ignore = if ignore_patterns.is_empty() {
        None
    } else {
        let mut b = globset::GlobSetBuilder::new();
        for p in ignore_patterns {
            b.add(globset::Glob::new(p).map_err(|e| {
                ToolError::InvalidArgs(format!("bad ignore {p:?}: {e}"))
            })?);
        }
        Some(
            b.build()
                .map_err(|e| ToolError::InvalidArgs(e.to_string()))?,
        )
    };

    let depth = if recursive { max_depth } else { 1 };
    let mut out: Vec<LsEntry> = Vec::new();
    let walker = walkdir::WalkDir::new(root)
        .min_depth(1)
        .max_depth(depth)
        .follow_links(false)
        .sort_by_file_name();
    for entry in walker {
        if out.len() >= max_entries {
            break;
        }
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let path = entry.path();
        let rel = path.strip_prefix(root).unwrap_or(path);
        if let Some(gs) = &ignore {
            if gs.is_match(rel) {
                continue;
            }
        }
        let is_dir = entry.file_type().is_dir();
        let size = if is_dir {
            0
        } else {
            entry.metadata().map(|m| m.len()).unwrap_or(0)
        };
        out.push(LsEntry {
            name: entry
                .file_name()
                .to_string_lossy()
                .into_owned(),
            path: path.display().to_string(),
            is_dir,
            size,
        });
    }
    // Sort directories first, then alpha.
    out.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn invoke_lists_directory_non_recursive() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        tokio::fs::write(root.join("a.txt"), "x").await.unwrap();
        tokio::fs::write(root.join("b.txt"), "yy").await.unwrap();
        tokio::fs::create_dir(root.join("sub")).await.unwrap();
        tokio::fs::write(root.join("sub/deep.txt"), "z").await.unwrap();
        let tool = LsTool;
        let v = tool
            .invoke(json!({"path": root.display().to_string()}))
            .await
            .unwrap();
        // Non-recursive: sees `sub` and the two files = 3 entries.
        assert_eq!(v.get("count").and_then(Value::as_u64), Some(3));
    }

    #[tokio::test]
    async fn invoke_recurses() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        tokio::fs::create_dir(root.join("sub")).await.unwrap();
        tokio::fs::write(root.join("sub/deep.txt"), "z").await.unwrap();
        let tool = LsTool;
        let v = tool
            .invoke(json!({
                "path": root.display().to_string(),
                "recursive": true,
                "max_depth": 3
            }))
            .await
            .unwrap();
        // Recursive: sees `sub` + `sub/deep.txt`.
        assert_eq!(v.get("count").and_then(Value::as_u64), Some(2));
    }

    #[tokio::test]
    async fn invoke_respects_ignore_globs() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        tokio::fs::write(root.join("keep.txt"), "x").await.unwrap();
        tokio::fs::write(root.join("skip.tmp"), "y").await.unwrap();
        let tool = LsTool;
        let v = tool
            .invoke(json!({
                "path": root.display().to_string(),
                "ignore": ["*.tmp"]
            }))
            .await
            .unwrap();
        assert_eq!(v.get("count").and_then(Value::as_u64), Some(1));
    }

    #[tokio::test]
    async fn invoke_errors_on_missing_path() {
        let tool = LsTool;
        let err = tool
            .invoke(json!({"path": "/this/should/never/exist/anywhere"}))
            .await
            .expect_err("must fail");
        assert!(matches!(err, ToolError::Io(_)));
    }
}
