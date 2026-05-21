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

    /// An I/O error occurred (loopback listener, token persistence, …).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The OAuth loopback flow failed (bad callback request, state mismatch, …).
    #[error("OAuth loopback flow error: {0}")]
    OAuthFlow(String),

    /// The OAuth loopback flow timed out waiting for the browser redirect.
    #[error("OAuth loopback flow timed out after {0} seconds")]
    OAuthTimeout(u64),

    /// The local `language_server` binary could not be located.
    #[cfg(feature = "local-ls")]
    #[error("language_server binary not found")]
    LanguageServerNotFound,

    /// Spawning or controlling the local `language_server` process failed.
    #[cfg(feature = "local-ls")]
    #[error("language_server process error: {0}")]
    Spawn(String),

    /// The `language_server` did not announce its listening port in time.
    #[cfg(feature = "local-ls")]
    #[error("timed out waiting for language_server to report its port")]
    PortTimeout,

    /// Building the pinned TLS configuration failed.
    #[cfg(feature = "local-ls")]
    #[error("TLS configuration error: {0}")]
    Tls(String),

    /// Establishing the gRPC transport channel failed.
    #[cfg(feature = "local-ls")]
    #[error("gRPC transport error: {0}")]
    Transport(String),

    /// A gRPC call returned a non-OK status.
    #[cfg(feature = "local-ls")]
    #[error("gRPC status error: {0}")]
    Grpc(String),
}

#[cfg(feature = "local-ls")]
impl From<tonic::Status> for SdkError {
    fn from(status: tonic::Status) -> Self {
        SdkError::Grpc(status.to_string())
    }
}
