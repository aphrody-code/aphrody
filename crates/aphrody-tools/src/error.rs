// SPDX-License-Identifier: Apache-2.0
//! Runtime error surface for builtin tools.
//!
//! The [`ToolsError`](crate::ToolsError) type lives in `lib.rs` and covers
//! *registry-level* failures (duplicate registration, schema validation,
//! vendor projection). [`ToolError`] is the runtime counterpart returned by
//! [`Tool::invoke`](crate::Tool::invoke) when a tool executes but cannot
//! complete its task (I/O, HTTP, JSON parse, OS not supported…).
//!
//! The two are intentionally distinct so callers can decide whether to
//! surface the error back to the model verbatim (`ToolError`) or treat it
//! as a configuration bug to abort the agent loop (`ToolsError`).

use thiserror::Error;

/// Failures emitted by [`Tool::invoke`](crate::Tool::invoke).
#[derive(Debug, Error)]
pub enum ToolError {
    /// JSON argument shape did not match the declared input schema, or a
    /// required field was missing / typed wrong.
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),

    /// The supplied path resolves outside the workspace root or violates a
    /// permission rule.
    #[error("path not allowed: {0}")]
    PathNotAllowed(String),

    /// File system I/O failure (read, write, metadata, rename, …).
    #[error("io error: {0}")]
    Io(String),

    /// HTTP / network failure during a web-fetch style call.
    #[error("network error: {0}")]
    Network(String),

    /// A target shell command or external binary exited non-zero.
    #[error("execution failed (exit {code}): {stderr}")]
    Exec {
        /// Exit code returned by the child process.
        code: i32,
        /// Captured stderr (truncated to a sensible head limit).
        stderr: String,
    },

    /// A required substring (for edit) was not found in the target file.
    #[error("string not found in file: {0}")]
    NotFound(String),

    /// A required substring (for edit) matched more than once and the call
    /// did not opt into multi-replace.
    #[error("ambiguous match: {count} occurrences of {needle:?}")]
    AmbiguousMatch {
        /// Number of matches observed.
        count: usize,
        /// The needle the caller tried to replace.
        needle: String,
    },

    /// Operation is not supported on the current target. The most common
    /// case is filesystem / network access in `wasm32-unknown-unknown`.
    #[error("operation not supported on this target: {0}")]
    WasmNotSupported(&'static str),

    /// Underlying serde_json (de)serialisation failure.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// A timeout fired while waiting on an external resource.
    #[error("timeout: {0}")]
    Timeout(String),

    /// Generic wrapper for ad-hoc failures that don't fit any other variant.
    #[error("{0}")]
    Other(String),
}

impl ToolError {
    /// Convenience constructor for an [`ToolError::Other`] from anything
    /// that can be converted to a [`String`].
    pub fn other(message: impl Into<String>) -> Self {
        Self::Other(message.into())
    }

    /// Convenience constructor for [`ToolError::InvalidArgs`].
    pub fn invalid_args(message: impl Into<String>) -> Self {
        Self::InvalidArgs(message.into())
    }

    /// Convenience constructor for [`ToolError::Io`].
    pub fn io(message: impl Into<String>) -> Self {
        Self::Io(message.into())
    }
}

impl From<std::io::Error> for ToolError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_messages_are_descriptive() {
        let e = ToolError::InvalidArgs("missing path".into());
        assert!(format!("{e}").contains("invalid arguments"));
        let e = ToolError::PathNotAllowed("/etc/shadow".into());
        assert!(format!("{e}").contains("not allowed"));
        let e = ToolError::AmbiguousMatch {
            count: 3,
            needle: "foo".into(),
        };
        assert!(format!("{e}").contains("ambiguous"));
    }

    #[test]
    fn io_error_conversion() {
        let raw =
            std::io::Error::new(std::io::ErrorKind::NotFound, "no such file");
        let e: ToolError = raw.into();
        assert!(matches!(e, ToolError::Io(_)));
    }

    #[test]
    fn json_error_conversion() {
        let raw = serde_json::from_str::<serde_json::Value>("not json")
            .expect_err("bad json");
        let e: ToolError = raw.into();
        assert!(matches!(e, ToolError::Json(_)));
    }
}
