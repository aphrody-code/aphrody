// SPDX-License-Identifier: Apache-2.0
//! `glob` — find files by pattern, sorted by mtime (most-recent first).
//!
//! Rust port of `packages/gemini-cli/packages/core/src/tools/glob.ts`. Backed
//! by the workspace's [`globset`] crate for the matcher and [`walkdir`] for
//! the traversal. Honours a default ignore list (`.git`, `node_modules`,
//! `target`, `dist`, `build`) similar to the gemini-cli's hardcoded
//! exclusions; pass `respect_default_ignores = false` to disable.

use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    Permission, PermissionDescriptor, Tool, ToolDescriptor, ToolError,
};

/// Canonical tool name.
pub const NAME: &str = "glob";

/// Default cap on the number of matches returned. The gemini-cli ships
/// 100; we stay aligned.
pub const DEFAULT_MAX_RESULTS: usize = 100;

/// Default ignored directory components.
pub const DEFAULT_IGNORED: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    "__pycache__",
];

/// Parsed arguments for the `glob` tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobArgs {
    /// Glob pattern (e.g. `"**/*.rs"`, `"src/**/main.*"`).
    pub pattern: String,
    /// Optional directory to root the search in. Defaults to the process
    /// working directory.
    #[serde(default)]
    pub path: Option<String>,
    /// Maximum number of results. Defaults to [`DEFAULT_MAX_RESULTS`].
    #[serde(default)]
    pub max_results: Option<usize>,
    /// When `false`, do not skip default-ignored directories. Defaults to
    /// `true`.
    #[serde(default = "default_true")]
    pub respect_default_ignores: bool,
}

fn default_true() -> bool {
    true
}

/// Descriptor for the `glob` tool.
#[must_use]
pub fn descriptor() -> ToolDescriptor {
    ToolDescriptor::new(
        NAME,
        "Find files whose path matches a glob pattern (e.g. `**/*.rs`). \
         Returns matches sorted by mtime descending. Skips noisy \
         directories (`.git`, `node_modules`, `target`, `dist`, `build`, \
         `__pycache__`) by default.",
        json!({
            "type": "object",
            "properties": {
                "pattern":     {"type": "string"},
                "path":        {"type": "string", "description": "Optional root (defaults to cwd)."},
                "max_results": {"type": "integer", "minimum": 1},
                "respect_default_ignores": {"type": "boolean"}
            },
            "required": ["pattern"],
            "additionalProperties": false
        }),
    )
    .expect("glob name is valid")
    .with_tag("fs")
    .with_tag("read")
}

/// Implementation handle.
#[derive(Debug, Default, Clone, Copy)]
pub struct GlobTool;

#[async_trait]
impl Tool for GlobTool {
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
        let parsed: GlobArgs = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidArgs(e.to_string()))?;
        let pattern = parsed.pattern.clone();
        let respect_default = parsed.respect_default_ignores;
        let max = parsed.max_results.unwrap_or(DEFAULT_MAX_RESULTS);
        let root = parsed
            .path
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

        // Globset and walkdir are sync; offload to the blocking pool.
        let matches = tokio::task::spawn_blocking(move || {
            scan_glob(&root, &pattern, respect_default, max)
        })
        .await
        .map_err(|e| ToolError::Other(format!("join error: {e}")))??;

        let paths: Vec<String> =
            matches.iter().map(|p| p.display().to_string()).collect();

        Ok(json!({
            "pattern": parsed.pattern,
            "count":   paths.len(),
            "matches": paths,
        }))
    }
}

/// Walk `root`, returning paths matching `pattern` sorted by mtime
/// descending, capped at `max`.
fn scan_glob(
    root: &std::path::Path,
    pattern: &str,
    respect_default: bool,
    max: usize,
) -> Result<Vec<PathBuf>, ToolError> {
    let mut builder = globset::GlobSetBuilder::new();
    builder.add(globset::Glob::new(pattern).map_err(|e| {
        ToolError::InvalidArgs(format!("bad glob {pattern:?}: {e}"))
    })?);
    let set = builder
        .build()
        .map_err(|e| ToolError::InvalidArgs(e.to_string()))?;

    let mut entries: Vec<(PathBuf, std::time::SystemTime)> = Vec::new();
    let walker = walkdir::WalkDir::new(root).follow_links(false);
    for entry in walker.into_iter().filter_entry(|e| {
        !respect_default
            || !is_default_ignored(e.file_name().to_string_lossy().as_ref())
    }) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_file() {
            continue;
        }
        // Match against the *relative* path so `**/*.rs` works as expected.
        let rel = entry.path().strip_prefix(root).unwrap_or(entry.path());
        if !set.is_match(rel) && !set.is_match(entry.path()) {
            continue;
        }
        let mtime = entry
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .unwrap_or(std::time::UNIX_EPOCH);
        entries.push((entry.into_path(), mtime));
    }
    entries.sort_by(|a, b| b.1.cmp(&a.1));
    entries.truncate(max);
    Ok(entries.into_iter().map(|(p, _)| p).collect())
}

fn is_default_ignored(name: &str) -> bool {
    DEFAULT_IGNORED.contains(&name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn invoke_finds_matches() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        tokio::fs::write(root.join("a.rs"), "fn a() {}").await.unwrap();
        tokio::fs::write(root.join("b.rs"), "fn b() {}").await.unwrap();
        tokio::fs::write(root.join("c.txt"), "x").await.unwrap();
        let tool = GlobTool;
        let v = tool
            .invoke(json!({
                "pattern": "*.rs",
                "path": root.display().to_string()
            }))
            .await
            .unwrap();
        assert_eq!(v.get("count").and_then(Value::as_u64), Some(2));
    }

    #[tokio::test]
    async fn invoke_skips_default_ignored_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        tokio::fs::create_dir_all(root.join("node_modules")).await.unwrap();
        tokio::fs::write(root.join("node_modules/x.rs"), "x")
            .await
            .unwrap();
        tokio::fs::write(root.join("kept.rs"), "y").await.unwrap();
        let tool = GlobTool;
        let v = tool
            .invoke(json!({
                "pattern": "**/*.rs",
                "path": root.display().to_string()
            }))
            .await
            .unwrap();
        assert_eq!(v.get("count").and_then(Value::as_u64), Some(1));
    }

    #[tokio::test]
    async fn invoke_rejects_invalid_glob() {
        let tool = GlobTool;
        let err = tool
            .invoke(json!({"pattern": "[unterminated"}))
            .await
            .expect_err("invalid glob must fail");
        assert!(matches!(err, ToolError::InvalidArgs(_)));
    }
}
