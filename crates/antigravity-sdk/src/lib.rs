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
pub mod config;
pub mod endpoints;
pub mod error;
pub mod models;
pub mod oauth;
pub mod state_sync;

/// Local `language_server` gRPC bridge (opt-in `local-ls` feature, host-only).
///
/// Spawns the Codeium-derived `language_server.exe`, parses its dynamic port,
/// pins its self-signed certificate, and speaks `exa.*` protobuf gRPC. Excluded
/// from default + wasm builds because it links `tonic`/`prost` + a native TLS
/// stack. Build with `--features local-ls`.
#[cfg(feature = "local-ls")]
pub mod local_ls;

// ---------------------------------------------------------------------------
// Convenience re-exports at crate root
// ---------------------------------------------------------------------------

pub use auth::{ANTIGRAVITY_CLIENT_ID, ANTIGRAVITY_SCOPES, OAuthToken, token_from_credential_manager};
pub use client::AntigravityClient;
pub use config::{GeminiConfig, config_path};
pub use endpoints::{CloudCodeEndpoint, GEMINI_API_HOST, VERTEX_AI_HOST};
pub use error::SdkError;
pub use models::{
    Candidate, ClientMetadata, Content, FetchAvailableModelsRequest,
    FetchAvailableModelsResponse, GenerateContentRequest, GenerateContentResponse,
    LoadCodeAssistRequest, LoadCodeAssistResponse, ModelInfo, OnboardUserRequest,
    OnboardUserResponse, Part, Tier,
};
pub use oauth::{PkcePair, authorize_url, login, persist};
pub use state_sync::{StateSyncEntry, decode_unified_state, parse_item_table};
