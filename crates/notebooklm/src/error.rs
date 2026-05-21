// SPDX-License-Identifier: Apache-2.0
//! Typed error surface for the NotebookLM RPC client.
//!
//! All public functions on `NotebookClient` return [`Result<T>`] = `Result<T, NotebookError>`.
//! Each variant carries enough context to route in callers (auth refresh, retry, surface-back).

use thiserror::Error;

/// Public alias every API function in this crate uses.
pub type Result<T> = core::result::Result<T, NotebookError>;

/// Categorised error for every RPC, transport or parsing failure.
#[derive(Debug, Error)]
pub enum NotebookError {
    /// Transport-level failure (connect / TLS / IO). Pure-HTTP failures from
    /// `reqwest` bubble up here.
    #[error("network failure: {0}")]
    Network(String),

    /// Authentication or session token problem. Triggered by 401/403 from
    /// `batchexecute`, missing cookies/OAuth token, or a `at`/`bl` mismatch
    /// the upstream rejects.
    #[error("auth failure: {0}")]
    Auth(String),

    /// The Boq RPC executed but the response carried a non-2xx HTTP status or
    /// a `UserDisplayableError` envelope.
    #[error("RPC {rpc_id} returned HTTP {status}: {message}")]
    Rpc { rpc_id: String, status: u16, message: String },

    /// Response body could not be parsed (envelope shape unexpected, inner
    /// JSON malformed, missing field on a `wrb.fr` array).
    #[error("parse failure: {0}")]
    Parse(String),

    /// Quota exhausted or rate limited. Caller should back off and retry.
    #[error("quota exceeded: {0}")]
    Quota(String),

    /// Notebook / source / artifact id not found.
    #[error("not found: {0}")]
    NotFound(String),
}

impl From<reqwest::Error> for NotebookError {
    fn from(value: reqwest::Error) -> Self {
        NotebookError::Network(value.to_string())
    }
}

impl From<serde_json::Error> for NotebookError {
    fn from(value: serde_json::Error) -> Self {
        NotebookError::Parse(value.to_string())
    }
}

impl From<std::io::Error> for NotebookError {
    fn from(value: std::io::Error) -> Self {
        NotebookError::Network(value.to_string())
    }
}
