// SPDX-License-Identifier: Apache-2.0
//! The `apply_patch` tool: parse an `*** Begin Patch ... *** End Patch`
//! document and apply it to a [`PatchFileSystem`].

use std::collections::BTreeMap;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use aphrody_patch::AppliedSummary;
use aphrody_patch::PatchFileSystem;
use aphrody_patch::StdFileSystem;
use aphrody_patch::apply_patch;
use aphrody_patch::parse_patch;
use aphrody_toolcall::AdditionalProperties;
use aphrody_toolcall::JsonSchema;
use aphrody_toolcall::ToolDefinition;
use aphrody_toolcall::ToolError;
use aphrody_toolcall::ToolExecutor;
use aphrody_toolcall::ToolOutput;
use serde::Deserialize;

/// Decoded arguments for the `apply_patch` tool.
#[derive(Debug, Deserialize)]
struct ApplyPatchToolArgs {
    /// The full patch document, framed by `*** Begin Patch` / `*** End Patch`.
    patch: String,
    /// Optional working directory that relative paths in the patch are resolved
    /// against. Defaults to the agent's current directory.
    #[serde(default)]
    cwd: Option<String>,
}

/// A [`PatchFileSystem`] that resolves relative paths against a `root` before
/// delegating to a borrowed inner filesystem.
///
/// Absolute paths are passed through unchanged. This lets the `apply_patch`
/// tool honor a per-call `cwd` without the underlying [`PatchFileSystem`] having
/// to know about it. It borrows the inner filesystem (`&'a F`) so the tool's
/// owned backend is reused rather than moved.
struct RootedFs<'a, F: PatchFileSystem> {
    root: PathBuf,
    inner: &'a F,
}

impl<F: PatchFileSystem> RootedFs<'_, F> {
    /// Join `path` onto `root` unless it is already absolute.
    fn resolve(&self, path: &Path) -> PathBuf {
        if path.is_absolute() {
            path.to_path_buf()
        } else {
            self.root.join(path)
        }
    }
}

impl<F: PatchFileSystem> PatchFileSystem for RootedFs<'_, F> {
    fn read(&self, path: &Path) -> io::Result<String> {
        self.inner.read(&self.resolve(path))
    }

    fn write(&self, path: &Path, contents: &str) -> io::Result<()> {
        self.inner.write(&self.resolve(path), contents)
    }

    fn remove(&self, path: &Path) -> io::Result<()> {
        self.inner.remove(&self.resolve(path))
    }

    fn exists(&self, path: &Path) -> bool {
        self.inner.exists(&self.resolve(path))
    }
}

/// Applies an LLM-authored `apply_patch` document to the filesystem.
///
/// The patch grammar is the one parsed by [`aphrody_patch::parse_patch`]
/// (add / delete / update-with-fuzzy-context / move). Application goes through
/// a pluggable [`PatchFileSystem`]: [`StdFileSystem`] on real disk by default,
/// or any injected backend (see [`ApplyPatchTool::with_fs`]) for hermetic tests
/// and sandboxes.
pub struct ApplyPatchTool<F: PatchFileSystem = StdFileSystem> {
    definition: ToolDefinition,
    fs: F,
}

impl<F: PatchFileSystem> std::fmt::Debug for ApplyPatchTool<F> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApplyPatchTool")
            .field("name", &self.definition.name)
            .finish_non_exhaustive()
    }
}

impl Default for ApplyPatchTool<StdFileSystem> {
    fn default() -> Self {
        Self::new()
    }
}

impl ApplyPatchTool<StdFileSystem> {
    /// Create a tool that applies patches to real disk via [`StdFileSystem`].
    #[must_use]
    pub fn new() -> Self {
        Self::with_fs(StdFileSystem)
    }
}

impl<F: PatchFileSystem> ApplyPatchTool<F> {
    /// Create a tool that applies patches through a custom [`PatchFileSystem`]
    /// (used to drive the tool against in-memory or sandboxed backends).
    #[must_use]
    pub fn with_fs(fs: F) -> Self {
        Self {
            definition: build_definition(),
            fs,
        }
    }
}

/// Build the model-visible definition for the `apply_patch` tool.
fn build_definition() -> ToolDefinition {
    let mut properties = BTreeMap::new();
    properties.insert(
        "patch".to_string(),
        JsonSchema::string(Some(
            "The patch document, framed by `*** Begin Patch` and `*** End \
             Patch`. Supports `*** Add File:`, `*** Delete File:`, and `*** \
             Update File:` (with `@@` context markers and `-`/`+`/space body \
             lines), plus `*** Move to:` for renames."
                .to_string(),
        )),
    );
    properties.insert(
        "cwd".to_string(),
        JsonSchema::string(Some(
            "Working directory that relative paths in the patch resolve \
             against. Defaults to the agent's current directory."
                .to_string(),
        )),
    );

    let input_schema = JsonSchema::object(
        properties,
        Some(vec!["patch".to_string()]),
        Some(AdditionalProperties::Boolean(false)),
    );

    let mut output_properties = BTreeMap::new();
    output_properties.insert(
        "added".to_string(),
        JsonSchema::array(JsonSchema::string(None), Some("Created file paths.".to_string())),
    );
    output_properties.insert(
        "modified".to_string(),
        JsonSchema::array(
            JsonSchema::string(None),
            Some("Modified or renamed file paths (rename records the destination).".to_string()),
        ),
    );
    output_properties.insert(
        "deleted".to_string(),
        JsonSchema::array(
            JsonSchema::string(None),
            Some("Removed file paths (rename records the source).".to_string()),
        ),
    );
    let output_schema = JsonSchema::object(
        output_properties,
        Some(vec![
            "added".to_string(),
            "modified".to_string(),
            "deleted".to_string(),
        ]),
        Some(AdditionalProperties::Boolean(true)),
    );

    ToolDefinition::new(
        "apply_patch",
        "Apply a patch document to the filesystem. Use this to create, delete, \
         rename, and edit files. The patch is framed by `*** Begin Patch` / \
         `*** End Patch` and uses `*** Add/Delete/Update File:` directives with \
         `@@` context and `-`/`+` body lines.",
        input_schema,
    )
    .with_output_schema(output_schema)
}

/// Render an [`AppliedSummary`] as a compact human/JSON-friendly report.
fn summarize(summary: &AppliedSummary) -> String {
    fn join(paths: &[PathBuf]) -> String {
        paths
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ")
    }

    let total = summary.added.len() + summary.modified.len() + summary.deleted.len();
    let mut body = format!("applied patch: {total} file(s) touched");
    if !summary.added.is_empty() {
        body.push_str(&format!("\nadded:    {}", join(&summary.added)));
    }
    if !summary.modified.is_empty() {
        body.push_str(&format!("\nmodified: {}", join(&summary.modified)));
    }
    if !summary.deleted.is_empty() {
        body.push_str(&format!("\ndeleted:  {}", join(&summary.deleted)));
    }
    body
}

#[async_trait::async_trait]
impl<F: PatchFileSystem + Send + Sync> ToolExecutor for ApplyPatchTool<F> {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    async fn handle(&self, arguments: serde_json::Value) -> Result<ToolOutput, ToolError> {
        let args: ApplyPatchToolArgs =
            serde_json::from_value(arguments).map_err(|err| ToolError::InvalidArguments {
                tool: "apply_patch".to_string(),
                message: err.to_string(),
            })?;

        let parsed = match parse_patch(&args.patch) {
            Ok(parsed) => parsed,
            Err(err) => {
                return Ok(ToolOutput::error(format!("apply_patch: parse error: {err}")));
            }
        };

        // Resolve relative paths against `cwd` when one is provided, otherwise
        // apply against the inner filesystem verbatim (its own notion of cwd).
        let result = match args.cwd {
            Some(cwd) => {
                let rooted = RootedFs {
                    root: PathBuf::from(cwd),
                    inner: &self.fs,
                };
                apply_patch(&parsed, &rooted)
            }
            None => apply_patch(&parsed, &self.fs),
        };

        match result {
            Ok(summary) => Ok(ToolOutput::ok(summarize(&summary))),
            Err(err) => Ok(ToolOutput::error(format!("apply_patch: {err}"))),
        }
    }
}

#[cfg(test)]
#[path = "apply_patch_tests.rs"]
mod tests;
