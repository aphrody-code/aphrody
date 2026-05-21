// SPDX-License-Identifier: Apache-2.0

/// Errors produced by the `aphrody-capture` public API.
#[derive(Debug, thiserror::Error)]
pub enum CaptureError {
    /// Screen / window capture is not supported on this platform.
    ///
    /// Returned by every function on non-Windows targets.
    #[error("screen capture is not supported on this platform")]
    Unsupported,

    /// A Win32 GDI call failed.
    ///
    /// The inner string contains a description of the failing call and, where
    /// available, the Windows error code.
    #[error("Win32 GDI error: {0}")]
    Win32(String),

    /// PNG encoding failed.
    #[error("PNG encode error: {0}")]
    Encode(String),

    /// No window matching the requested title substring was found.
    #[error("no visible window with title containing '{0}' was found")]
    NotFound(String),
}
