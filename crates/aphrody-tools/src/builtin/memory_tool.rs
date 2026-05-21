// SPDX-License-Identifier: Apache-2.0
//! `memory` — durable fact store (append-only Markdown).
//!
//! Rust port of `packages/gemini-cli/packages/core/src/tools/memoryTool.ts`.
//! The TS surface ships a single `save_memory(fact)` op that appends to a
//! `GEMINI.md` file under the user-home memory directory. We generalise it
//! to a tiny op-router: `save` (append), `list` (read raw), `clear` (reset
//! the file) — covering the common interactive memory flows without
//! pulling in the heavyweight `aphrody-memory` backend stack (HNSW / SQLite
//! / LanceDB), which lives behind a separate tool surface.
//!
//! Path resolution:
//! * If `path` is supplied, it is used verbatim.
//! * Otherwise, defaults to `$APHRODY_MEMORY_FILE` if set, else
//!   `<cwd>/MEMORY.md`.

use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::io::AsyncWriteExt;

use crate::{
    Permission, PermissionDescriptor, Tool, ToolDescriptor, ToolError,
};

/// Canonical tool name.
pub const NAME: &str = "memory";

/// Environment variable that overrides the default memory file path.
pub const ENV_MEMORY_FILE: &str = "APHRODY_MEMORY_FILE";

/// Default memory file name relative to the working directory.
pub const DEFAULT_FILENAME: &str = "MEMORY.md";

/// Section header used to fence appended facts. Mirrors the gemini-cli
/// convention so existing `GEMINI.md` files keep parsing.
pub const MEMORY_SECTION_HEADER: &str = "## aphrody-memory";

/// Parsed arguments for the `memory` tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct MemoryArgs {
    /// Operation to perform: `save`, `list`, `clear`.
    pub op: MemoryOp,
    /// Override file path. Defaults via `APHRODY_MEMORY_FILE` / `cwd/MEMORY.md`.
    #[serde(default)]
    pub path: Option<String>,
    /// Fact text. Required when `op = save`.
    #[serde(default)]
    pub fact: Option<String>,
}

/// Memory-tool operation kind.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryOp {
    /// Append a fact to the memory file under [`MEMORY_SECTION_HEADER`].
    Save,
    /// Return the raw memory file contents.
    List,
    /// Reset the memory file (zero bytes).
    Clear,
}

/// Descriptor for the `memory` tool.
#[must_use]
pub fn descriptor() -> ToolDescriptor {
    ToolDescriptor::new(
        NAME,
        "Durable fact store backed by a Markdown file. `save` appends a \
         fact under the `## aphrody-memory` section; `list` returns the \
         raw file contents; `clear` truncates the file. The path defaults \
         to `$APHRODY_MEMORY_FILE` or `<cwd>/MEMORY.md`.",
        json!({
            "type": "object",
            "properties": {
                "op":   {"type": "string", "enum": ["save", "list", "clear"]},
                "path": {"type": "string"},
                "fact": {"type": "string", "description": "Required when op=save."}
            },
            "required": ["op"],
            "additionalProperties": false
        }),
    )
    .expect("memory name is valid")
    .require_permission()
    .with_tag("memory")
}

/// Implementation handle.
#[derive(Debug, Default, Clone, Copy)]
pub struct MemoryTool;

#[async_trait]
impl Tool for MemoryTool {
    fn descriptor(&self) -> ToolDescriptor {
        descriptor()
    }

    fn permission(&self) -> PermissionDescriptor {
        PermissionDescriptor {
            tool_name: NAME.into(),
            scopes: vec!["memory:write".into(), "fs:write".into()],
            default_permission: Permission::Ask,
        }
    }

    async fn invoke(&self, args: Value) -> Result<Value, ToolError> {
        let parsed: MemoryArgs = serde_json::from_value(args)
            .map_err(|e| ToolError::InvalidArgs(e.to_string()))?;
        let path = resolve_path(parsed.path.as_deref());
        match parsed.op {
            MemoryOp::Save => {
                let fact = parsed.fact.as_deref().ok_or_else(|| {
                    ToolError::InvalidArgs(
                        "`fact` is required when op = save".into(),
                    )
                })?;
                let fact = fact.trim();
                if fact.is_empty() {
                    return Err(ToolError::InvalidArgs(
                        "`fact` must be non-empty".into(),
                    ));
                }
                save_fact(&path, fact).await?;
                Ok(json!({
                    "path": path.display().to_string(),
                    "appended": fact,
                    "op": "save",
                }))
            }
            MemoryOp::List => {
                let body = match tokio::fs::read_to_string(&path).await {
                    Ok(s) => s,
                    Err(e) if e.kind()
                        == std::io::ErrorKind::NotFound =>
                    {
                        String::new()
                    }
                    Err(e) => return Err(ToolError::Io(e.to_string())),
                };
                let count = count_facts(&body);
                Ok(json!({
                    "path": path.display().to_string(),
                    "content": body,
                    "fact_count": count,
                    "op": "list",
                }))
            }
            MemoryOp::Clear => {
                if let Some(parent) = path.parent() {
                    if !parent.as_os_str().is_empty() {
                        tokio::fs::create_dir_all(parent).await?;
                    }
                }
                tokio::fs::write(&path, "").await?;
                Ok(json!({
                    "path": path.display().to_string(),
                    "op": "clear",
                }))
            }
        }
    }
}

fn resolve_path(override_path: Option<&str>) -> PathBuf {
    if let Some(p) = override_path {
        return PathBuf::from(p);
    }
    if let Ok(env) = std::env::var(ENV_MEMORY_FILE) {
        if !env.is_empty() {
            return PathBuf::from(env);
        }
    }
    std::env::current_dir()
        .unwrap_or_default()
        .join(DEFAULT_FILENAME)
}

async fn save_fact(
    path: &std::path::Path,
    fact: &str,
) -> Result<(), ToolError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            tokio::fs::create_dir_all(parent).await?;
        }
    }
    let existing = match tokio::fs::read_to_string(path).await {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(ToolError::Io(e.to_string())),
    };

    let mut out = existing;
    if !out.contains(MEMORY_SECTION_HEADER) {
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(MEMORY_SECTION_HEADER);
        out.push('\n');
    }
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str("- ");
    out.push_str(fact);
    out.push('\n');

    let mut f = tokio::fs::File::create(path).await?;
    f.write_all(out.as_bytes()).await?;
    f.flush().await?;
    Ok(())
}

fn count_facts(content: &str) -> usize {
    let mut in_section = false;
    let mut count = 0;
    for line in content.lines() {
        if line.starts_with("## ") {
            in_section = line.trim() == MEMORY_SECTION_HEADER;
            continue;
        }
        if in_section && line.trim_start().starts_with("- ") {
            count += 1;
        }
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn invoke_save_then_list_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mem.md");
        let tool = MemoryTool;
        tool.invoke(json!({
            "op": "save",
            "path": path.display().to_string(),
            "fact": "user prefers concise responses"
        }))
        .await
        .unwrap();
        tool.invoke(json!({
            "op": "save",
            "path": path.display().to_string(),
            "fact": "uses 4 space indent"
        }))
        .await
        .unwrap();
        let v = tool
            .invoke(json!({
                "op": "list",
                "path": path.display().to_string()
            }))
            .await
            .unwrap();
        assert_eq!(
            v.get("fact_count").and_then(Value::as_u64),
            Some(2)
        );
        let body = v.get("content").and_then(Value::as_str).unwrap();
        assert!(body.contains("user prefers concise responses"));
        assert!(body.contains("uses 4 space indent"));
        assert!(body.contains(MEMORY_SECTION_HEADER));
    }

    #[tokio::test]
    async fn invoke_clear_truncates() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("c.md");
        let tool = MemoryTool;
        tool.invoke(json!({
            "op": "save",
            "path": path.display().to_string(),
            "fact": "x"
        }))
        .await
        .unwrap();
        tool.invoke(json!({
            "op": "clear",
            "path": path.display().to_string()
        }))
        .await
        .unwrap();
        let actual = tokio::fs::read_to_string(&path).await.unwrap();
        assert!(actual.is_empty());
    }

    #[tokio::test]
    async fn invoke_save_rejects_empty_fact() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("e.md");
        let tool = MemoryTool;
        let err = tool
            .invoke(json!({
                "op": "save",
                "path": path.display().to_string(),
                "fact": "   "
            }))
            .await
            .expect_err("must fail");
        assert!(matches!(err, ToolError::InvalidArgs(_)));
    }

    #[tokio::test]
    async fn invoke_list_on_missing_file_returns_empty() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("never-existed.md");
        let tool = MemoryTool;
        let v = tool
            .invoke(json!({
                "op": "list",
                "path": path.display().to_string()
            }))
            .await
            .unwrap();
        assert_eq!(v.get("content").and_then(Value::as_str), Some(""));
        assert_eq!(v.get("fact_count").and_then(Value::as_u64), Some(0));
    }
}
