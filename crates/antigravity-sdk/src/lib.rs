// SPDX-License-Identifier: Apache-2.0
//! # antigravity-sdk
//!
//! Native Rust SDK for **Antigravity** (Google AI Ultra / Gemini).
//!
//! ## Authentication
//!
//! The Antigravity CLI (`agy`) stores the user's Google OAuth 2.0 token as a
//! **generic credential** named `gemini:antigravity` in the Windows Credential
//! Manager.  The blob is plaintext UTF-8 JSON:
//!
//! ```json
//! {
//!   "token": {
//!     "access_token": "ya29.…",
//!     "refresh_token": "1//…",
//!     "expiry": "2026-05-21T18:30:00Z"
//!   }
//! }
//! ```
//!
//! Use [`auth::token_from_credential_manager`] to read the token on Windows,
//! or supply your own [`auth::OAuthToken`] from any source.
//!
//! ## Quick start
//!
//! ```rust,no_run
//! # #[cfg(target_os = "windows")]
//! # {
//! use antigravity_sdk::client::AntigravityClient;
//!
//! // Install a rustls CryptoProvider before the first reqwest call.
//! rustls::crypto::ring::default_provider()
//!     .install_default()
//!     .ok();
//!
//! let client = AntigravityClient::from_credential_manager().unwrap();
//! # }
//! ```
//!
//! ## Platform support
//!
//! | Platform    | `token_from_credential_manager` | HTTP client |
//! |-------------|----------------------------------|-------------|
//! | Windows     | Yes (Win32 `CredReadW`)          | Yes         |
//! | Linux/macOS | `SdkError::Unsupported`          | Yes         |
//! | wasm32      | `SdkError::Unsupported`          | Yes         |

pub mod auth;
pub mod client;
pub mod error;

// ---------------------------------------------------------------------------
// Convenience re-exports at crate root
// ---------------------------------------------------------------------------

pub use auth::{ANTIGRAVITY_CLIENT_ID, ANTIGRAVITY_SCOPES, OAuthToken, token_from_credential_manager};
pub use client::AntigravityClient;
pub use error::SdkError;
