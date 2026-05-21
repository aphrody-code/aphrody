// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//! Typed error surface for the Adobe Firefly Services client.

use std::path::PathBuf;

/// Convenience result alias used across the crate.
pub type Result<T> = std::result::Result<T, FireflyError>;

/// Errors produced by the Firefly client.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum FireflyError {
    /// A required credential was missing from the environment / config.
    #[error("missing Firefly credential: {0} (set FIREFLY_CLIENT_ID and FIREFLY_CLIENT_SECRET)")]
    MissingCredential(&'static str),

    /// The IMS token endpoint rejected the client-credentials grant.
    #[error("IMS authentication failed (HTTP {status}): {body}")]
    Auth {
        /// HTTP status returned by `ims-na1.adobelogin.com`.
        status: u16,
        /// Response body (truncated, non-secret error description).
        body: String,
    },

    /// A Firefly REST call returned a non-success HTTP status.
    #[error("Firefly API error (HTTP {status}) at {endpoint}: {body}")]
    Api {
        /// HTTP status code.
        status: u16,
        /// The endpoint path that failed.
        endpoint: String,
        /// Response body (truncated).
        body: String,
    },

    /// The async job finished in a non-success terminal state.
    #[error("Firefly job {job_id} ended with status `{status}`: {detail}")]
    JobFailed {
        /// The Firefly job id (`urn:ff:jobs:...`).
        job_id: String,
        /// The terminal status string reported by Firefly.
        status: String,
        /// Any failure detail Firefly attached to the job.
        detail: String,
    },

    /// The async job did not reach a terminal state within the poll budget.
    #[error("Firefly job {job_id} did not complete within {waited_ms} ms ({polls} polls)")]
    JobTimeout {
        /// The Firefly job id.
        job_id: String,
        /// Total time spent polling, in milliseconds.
        waited_ms: u64,
        /// Number of status polls performed.
        polls: u32,
    },

    /// A response body could not be deserialized into the expected shape.
    #[error("failed to decode Firefly response from {endpoint}: {source}")]
    Decode {
        /// The endpoint whose response failed to decode.
        endpoint: String,
        /// The underlying serde error.
        #[source]
        source: serde_json::Error,
    },

    /// Network / transport failure from `reqwest`.
    #[error("HTTP transport error: {0}")]
    Http(#[from] reqwest::Error),

    /// Filesystem error while writing a downloaded output.
    #[error("I/O error at {path}: {source}")]
    Io {
        /// The path being written.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
}
