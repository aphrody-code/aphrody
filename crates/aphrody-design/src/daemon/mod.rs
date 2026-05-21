// SPDX-License-Identifier: Apache-2.0
//! SQLite project store + design-systems registry daemon.

pub mod db;
pub mod registry;

pub use db::{ConversationId, MessageId, ProjectId, ProjectStore, Role, Template};
pub use registry::{DesignSystemRecord, DesignSystemRegistry, DesignSystemSurface};

use thiserror::Error;

/// Top-level structured error for the daemon's storage + registry.
#[derive(Debug, Error)]
pub enum DaemonError {
    /// SQLite operation failed.
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// JSON (de)serialisation failed.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// I/O failure.
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// Requested row not present in the database.
    #[error("not found: {entity}={id}")]
    NotFound { entity: &'static str, id: String },

    /// A required field was empty or otherwise invalid.
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
}
