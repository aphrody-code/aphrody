// SPDX-License-Identifier: Apache-2.0
//! Unified error type for the antigravity-sdk crate.

use thiserror::Error;

/// All errors that can occur in the Antigravity SDK.
#[derive(Debug, Error)]
pub enum SdkError {
    /// The operation is not supported on this platform (e.g. Credential Manager
    /// on non-Windows).
    #[error("operation not supported on this platform: {0}")]
    Unsupported(&'static str),

    /// Windows Credential Manager returned an error (Win32 error code).
    #[cfg(target_os = "windows")]
    #[error("credential manager error (Win32 code {code})")]
    CredentialManager {
        /// Raw Win32 `GetLastError()` value.
        code: u32,
    },

    /// The credential exists but its blob does not contain the expected
    /// `{"token": {"access_token": "…"}}` structure.
    #[error("credential blob could not be parsed as an OAuthToken: {0}")]
    TokenParse(#[from] serde_json::Error),

    /// The credential blob was found but was empty or missing required fields.
    #[error("credential blob is empty or missing required fields")]
    EmptyCredential,

    /// An HTTP-level error occurred while refreshing the token.
    #[error("HTTP error during token refresh: {0}")]
    Http(#[from] reqwest::Error),

    /// The OAuth server returned an error response (non-2xx status).
    #[error("OAuth server error {status}: {body}")]
    OAuthServer {
        /// HTTP status code.
        status: u16,
        /// Response body from the server.
        body: String,
    },
}
