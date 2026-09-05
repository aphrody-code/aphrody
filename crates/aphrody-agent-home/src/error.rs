// SPDX-License-Identifier: Apache-2.0
//! Error type for the agent-home crate.
//!
//! Every fallible public entry point returns [`HomeError`]. The enum is
//! `Send + Sync + 'static` so an [`crate::AgentHome`] can be shared across the
//! tokio runtime and surfaced through `miette` / `anyhow` at the CLI seam.

use std::path::PathBuf;

use thiserror::Error;

/// Recoverable failures emitted by the agent-home crate.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum HomeError {
    /// A filesystem operation failed. The path is captured for diagnostics.
    #[error("io error at {path}: {source}")]
    Io {
        /// Path the operation targeted.
        path: PathBuf,
        /// Underlying OS error.
        #[source]
        source: std::io::Error,
    },

    /// Could not resolve the agent home root (`$APHRODY_HOME` / `HOME` /
    /// `USERPROFILE` all unset).
    #[error("could not resolve home directory (APHRODY_HOME / HOME / USERPROFILE unset)")]
    NoHome,

    /// A path escaped the workspace sandbox during a guarded read.
    #[error("path escapes workspace boundary: {path} (root {root})")]
    PathEscape {
        /// The offending path.
        path: PathBuf,
        /// The workspace root it must stay within.
        root: PathBuf,
    },

    /// A bootstrap file exceeded the per-file byte cap.
    #[error("file {path} is {size} bytes, over the {cap}-byte bootstrap cap")]
    FileTooLarge {
        /// Path of the oversized file.
        path: PathBuf,
        /// Observed size in bytes.
        size: u64,
        /// The configured cap.
        cap: u64,
    },

    /// A `SOUL.md` (or other typed doc) failed an anti-pattern lint.
    #[error("soul validation failed: {0}")]
    SoulValidation(String),

    /// JSON (de)serialization of the workspace-state file failed.
    #[error("workspace-state json error: {0}")]
    Json(#[from] serde_json::Error),

    /// Refused to overwrite an existing workspace without `--force`.
    #[error("workspace already configured at {0}; pass force to overwrite")]
    AlreadyConfigured(PathBuf),

    /// A git operation failed (only reachable with the `git` feature on a
    /// host target).
    #[error("git error: {0}")]
    Git(String),

    /// The filesystem watcher failed to start or register a path.
    #[error("watch error: {0}")]
    Watch(String),
}

impl HomeError {
    /// Build an [`HomeError::Io`] from a path + error pair. Keeps call sites
    /// terse (`map_err(|e| HomeError::io(path, e))`).
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}
