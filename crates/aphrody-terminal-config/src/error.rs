// SPDX-License-Identifier: Apache-2.0
//! Error type for the aphrody-terminal-config loader.
//!
//! All public functions in this crate return [`Result<T, ConfigError>`]; the
//! variants cover the four classes of failure encountered by a strict config
//! loader:
//!
//! - [`ConfigError::Io`] — filesystem I/O failed (file unreadable, perms, etc.).
//! - [`ConfigError::Parse`] — the raw bytes were not syntactically valid JSON.
//! - [`ConfigError::Validation`] — the JSON was valid but did not match the
//!   strict `TerminalConfig` schema (unknown fields, wrong types, …).
//! - [`ConfigError::Schema`] — the JSON Schema itself could not be generated
//!   (currently only fires if `serde_json::to_string` fails, which is
//!   essentially unreachable on well-formed schemars output).

use std::path::PathBuf;

/// Result alias for the config crate.
pub type Result<T> = std::result::Result<T, ConfigError>;

/// Strongly-typed error surface for the config loader.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Underlying filesystem operation failed.
    #[error("io error on {path:?}: {source}")]
    Io {
        /// Path the loader attempted to read.
        path: PathBuf,
        /// Underlying OS-level error.
        #[source]
        source: std::io::Error,
    },

    /// The file contents were not syntactically valid JSON.
    #[error("parse error: {0}")]
    Parse(#[from] serde_json::Error),

    /// The JSON was syntactically valid but violated the strict schema
    /// (unknown field, wrong type, missing required key, …).
    #[error("validation error: {0}")]
    Validation(String),

    /// JSON Schema generation failed.  Practically unreachable.
    #[error("schema generation error: {0}")]
    Schema(String),
}

impl ConfigError {
    /// Build an [`ConfigError::Io`] from an arbitrary path and `std::io::Error`.
    #[must_use]
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    /// Build a [`ConfigError::Validation`] from any displayable message.
    #[must_use]
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }
}
