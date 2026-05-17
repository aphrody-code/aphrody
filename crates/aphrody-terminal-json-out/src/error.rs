// SPDX-License-Identifier: Apache-2.0
//! Error types for the JSONL framer.

use thiserror::Error;

/// Errors emitted by the JSONL framing pipeline.
#[derive(Debug, Error)]
pub enum JsonOutError {
    /// Failed to spawn the child process.
    #[error("failed to spawn child process: {0}")]
    Spawn(#[source] std::io::Error),

    /// Failed to await child process completion.
    #[error("failed to wait on child process: {0}")]
    Wait(#[source] std::io::Error),

    /// The requested stdio pipe (`stdout` or `stderr`) was not captured. This
    /// usually means the caller pre-set `Stdio::null()` before invocation.
    #[error("child process is missing the {0} pipe handle")]
    MissingPipe(&'static str),

    /// Envelope serialization failed (unreachable in practice — every field
    /// is a string or a `serde_json::Value`).
    #[error("failed to serialize envelope: {0}")]
    Serialize(#[source] serde_json::Error),

    /// IO error while reading framed lines from a downstream sink.
    #[error("IO error while framing: {0}")]
    Io(#[from] std::io::Error),
}
