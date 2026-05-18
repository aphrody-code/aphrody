// SPDX-License-Identifier: Apache-2.0
//! # aphrody-design-daemon
//!
//! Rust port of the `open-design` daemon's `.od/app.sqlite` project store +
//! `design-systems/*` registry loader. Two responsibilities, one crate:
//!
//! - [`db::ProjectStore`] — durable, single-file SQLite store for projects,
//!   conversations, messages, tabs, and saved templates. Schema is a faithful
//!   transcription of `apps/daemon/src/db.ts` (minus the routine / preview-
//!   comment / deployment tables, which are out of scope for the Rust port's
//!   first cut).
//! - [`registry::DesignSystemRegistry`] — in-process catalogue of design
//!   systems loaded from a `design-systems/<id>/DESIGN.md` directory
//!   structure (the actual on-disk shape used by open-design — 149 brands at
//!   the time of writing). Supports the legacy DESIGN.md + tokens.css +
//!   components.html layout and an optional `manifest.json` sidecar for
//!   forward compatibility.
//!
//! ## Cross-platform
//!
//! `rusqlite` is pulled with `features = ["bundled"]` so libsqlite3 is
//! statically linked: no system dependency, deterministic on Linux #1 /
//! Win11 #2 (cf. `CLAUDE.md` §0). The `wasm32-*` targets are out of scope
//! for the SQLite path (browser-side persistence would route through
//! IndexedDB instead), but the registry loader is pure-Rust and could be
//! cross-compiled with cfg-gated I/O.
//!
//! ## Error handling
//!
//! All public functions return [`anyhow::Result`] for ergonomic propagation,
//! while [`DaemonError`] gives downstream callers a structured discriminant
//! when they need to distinguish "row not found" from "schema mismatch".

#![forbid(unsafe_code)]

pub mod db;
pub mod registry;

pub use db::{
    ConversationId, MessageId, ProjectId, ProjectStore, Role, Template,
};
pub use registry::{DesignSystemRecord, DesignSystemRegistry, DesignSystemSurface};

use thiserror::Error;

/// Top-level structured error returned by the daemon's storage + registry
/// helpers. Most public APIs return [`anyhow::Result`]; this enum is exposed
/// for callers (CLI, gRPC server, embedding host) that need to branch on
/// failure modes without string-matching.
#[derive(Debug, Error)]
pub enum DaemonError {
    /// SQLite operation failed (`rusqlite` returned an error).
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// JSON (de)serialisation failed for a `*_json` column or manifest.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// I/O failure (filesystem path missing, permission denied, …).
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    /// The requested row was not present in the database.
    #[error("not found: {entity}={id}")]
    NotFound { entity: &'static str, id: String },

    /// A required field was empty or otherwise invalid.
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
}
