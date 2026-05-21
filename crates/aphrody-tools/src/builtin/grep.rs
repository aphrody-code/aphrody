// SPDX-License-Identifier: Apache-2.0
//! `grep` — ripgrep-flavoured content search across the workspace.
//!
//! Rust port of `packages/gemini-cli/packages/core/src/tools/grep.ts` (with
//! the `ripGrep.ts` fast-path collapsed into the same surface). Uses the
//! workspace's [`ignore`] crate to honour `.gitignore` / `.ignore` /
//! `.geminiignore` automatically, and `regex` for pattern matching.
//!
//! Returns at most `max_matches` hits across at most `max_files` files,
//! defaulting to the gemini-cli numbers (100 hits / unbounded files).

use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{
    Permission, PermissionDescriptor, Tool, ToolDescriptor, ToolError,
};

/// Canonical tool name.
pub const NAME: &str = "grep";

/// Default total match cap (parity with gemini-cli `DEFAULT_TOTAL_MAX_MATCHES`).
pub const DEFAULT_MAX_MATCHES: usize = 100;

/// Parsed arguments for the `grep` tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepArgs {
    /// Regular expression to match against file *contents* (line-by-line).
    pub pattern: String,
    /// Root directory. Defaults to the current working directory.
    #[serde(default)]
    pub path: Option<String>,
    /// Optional glob filter on file paths (e.g. `"**/*.rs"`).
    #[serde(default)]
    pub glob: Option<String>,
    /// Case-insensitive search. Defaults to `false`.
    #[serde(default)]
    pub case_insensitive: bool,
    /// Return paths only (no line contents). Defaults to `false`.
    #[serde(default)]
    pub names_only: bool,
    /// Hard cap on the number of returned matches. Defaults to
    /// [`DEFAULT_MAX_MATCHES`].
    #[serde(default)]
    pub max_matches: Option<usize>,
}

/// One match record returned by the tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrepMatch {
    /// Path that contained the match.
    pub path: String,
    /// 1-based line number.
    pub line: usize,
    /// Verbatim matching line (with surrounding whitespace trimmed).
    pub text: String,
}

/// Descriptor for the `grep` tool.
#[must_use]
pub fn descriptor() -> ToolDescriptor {
    ToolDescriptor::new(
        NAME,
        "Recursively search file contents for a regex pattern. Honours \
         .gitignore / .ignore by default. Optional `glob` filter restricts \
         matched paths. Use `names_only` for a path-only result. Returns at \
         most `max_matches` hits (default 100).",
        json!({
            "type": "object",
            "properties": {
                "pattern":          {"type": "string", "description": "Rust regex pattern."},
                "path":             {"type": "string"},
                "glob":             {"type": "string", "description": "Optional glob filter on file paths."},
                "case_insensitive": {"type": "boolean"},
                "names_only":       {"type": "boolean"},
                "max_matches":      {"type": "integer", "minimum": 1}
            },
            "required": ["pattern"],
            "additionalProperties": false
        }),
    )
    .expect("grep name is valid")
    .with_tag("fs")
    .with_tag("read")
    .with_tag("search")
}

/// Implementation handle.
#[derive(Debug, Default, Clone, Copy)]
pub struct GrepTool;

#[async_trait]
impl Tool for GrepTool {
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
        let parsed: GrepArgs = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidArgs(e.to_string()))?;
        let root = parsed
            .path
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        let max = parsed.max_matches.unwrap_or(DEFAULT_MAX_MATCHES);
        let pattern = if parsed.case_insensitive {
            format!("(?i){}", parsed.pattern)
        } else {
            parsed.pattern.clone()
        };
        let glob = parsed.glob.clone();
        let names_only = parsed.names_only;

        let result = tokio::task::spawn_blocking(move || {
            run_grep(&root, &pattern, glob.as_deref(), names_only, max)
        })
        .await
        .map_err(|e| ToolError::Other(format!("join error: {e}")))??;

        Ok(json!({
            "pattern":  parsed.pattern,
            "count":    result.len(),
            "matches":  result,
            "truncated": result.len() >= max,
        }))
    }
}

fn run_grep(
    root: &std::path::Path,
    pattern: &str,
    glob: Option<&str>,
    names_only: bool,
    max: usize,
) -> Result<Vec<GrepMatch>, ToolError> {
    let re = regex::Regex::new(pattern).map_err(|e| {
        ToolError::InvalidArgs(format!("bad regex {pattern:?}: {e}"))
    })?;
    let glob_set = match glob {
        Some(g) => {
            let mut b = globset::GlobSetBuilder::new();
            b.add(globset::Glob::new(g).map_err(|e| {
                ToolError::InvalidArgs(format!("bad glob {g:?}: {e}"))
            })?);
            Some(
                b.build()
                    .map_err(|e| ToolError::InvalidArgs(e.to_string()))?,
            )
        }
        None => None,
    };

    let mut matches: Vec<GrepMatch> = Vec::new();
    let mut seen_paths: std::collections::HashSet<PathBuf> =
        std::collections::HashSet::new();
    let walker = ignore::WalkBuilder::new(root)
        .follow_links(false)
        .standard_filters(true)
        .build();
    for entry in walker {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
            continue;
        }
        let path = entry.path();
        if let Some(gs) = &glob_set {
            let rel = path.strip_prefix(root).unwrap_or(path);
            if !gs.is_match(rel) && !gs.is_match(path) {
                continue;
            }
        }

        // Read line-by-line, capped at 4 MiB per file to avoid pathological
        // log files. Skip binary-looking files (any NUL in first 8 KiB).
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(_) => continue,
        };
        if bytes.iter().take(8192).any(|b| *b == 0) {
            continue;
        }
        if bytes.len() > 4 * 1024 * 1024 {
            continue;
        }
        let text = match std::str::from_utf8(&bytes) {
            Ok(s) => s,
            Err(_) => continue,
        };
        for (idx, line) in text.lines().enumerate() {
            if !re.is_match(line) {
                continue;
            }
            if names_only {
                if seen_paths.insert(path.to_path_buf()) {
                    matches.push(GrepMatch {
                        path: path.display().to_string(),
                        line: 0,
                        text: String::new(),
                    });
                }
                if matches.len() >= max {
                    return Ok(matches);
                }
                break;
            }
            matches.push(GrepMatch {
                path: path.display().to_string(),
                line: idx + 1,
                text: line.trim().to_string(),
            });
            if matches.len() >= max {
                return Ok(matches);
            }
        }
    }
    Ok(matches)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn invoke_finds_pattern() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        tokio::fs::write(root.join("a.txt"), "alpha\nbeta\ngamma\n")
            .await
            .unwrap();
        tokio::fs::write(root.join("b.txt"), "no match here\n")
            .await
            .unwrap();
        let tool = GrepTool;
        let v = tool
            .invoke(json!({
                "pattern": "beta",
                "path": root.display().to_string()
            }))
            .await
            .unwrap();
        assert_eq!(v.get("count").and_then(Value::as_u64), Some(1));
        let m = v.get("matches").and_then(Value::as_array).unwrap();
        assert_eq!(m[0].get("line").and_then(Value::as_u64), Some(2));
    }

    #[tokio::test]
    async fn invoke_honors_glob_filter() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        tokio::fs::write(root.join("a.rs"), "fn beta() {}").await.unwrap();
        tokio::fs::write(root.join("b.txt"), "beta").await.unwrap();
        let tool = GrepTool;
        let v = tool
            .invoke(json!({
                "pattern": "beta",
                "path": root.display().to_string(),
                "glob": "*.rs"
            }))
            .await
            .unwrap();
        assert_eq!(v.get("count").and_then(Value::as_u64), Some(1));
    }

    #[tokio::test]
    async fn invoke_names_only_collapses_per_file() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        tokio::fs::write(root.join("a.txt"), "x\nx\nx\nx\n").await.unwrap();
        let tool = GrepTool;
        let v = tool
            .invoke(json!({
                "pattern": "x",
                "path": root.display().to_string(),
                "names_only": true
            }))
            .await
            .unwrap();
        assert_eq!(v.get("count").and_then(Value::as_u64), Some(1));
    }

    #[tokio::test]
    async fn invoke_rejects_bad_regex() {
        let tool = GrepTool;
        let err = tool
            .invoke(json!({"pattern": "(unterminated"}))
            .await
            .expect_err("must fail");
        assert!(matches!(err, ToolError::InvalidArgs(_)));
    }
}
