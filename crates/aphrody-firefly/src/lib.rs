// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//! `aphrody-firefly` — a pure-Rust client for **Adobe Firefly Services**.
//!
//! # Why this crate exists
//!
//! aphrody already generates images through Google's Nano Banana
//! (`aphrody-images`, on the `gemini-web` Boq client). This crate adds a second,
//! commercial-grade backend — Adobe Firefly — and, just as importantly, a clean
//! Rust auth core for the **whole Firefly Services family** (the Firefly image
//! API and the cloud **Photoshop** / Lightroom APIs all authenticate through
//! the same IMS server-to-server token).
//!
//! It is the in-policy answer to the TypeScript `photoshop-mcp` server: instead
//! of driving a *locally installed* Photoshop through ExtendScript/COM (which is
//! Node-only, Windows/macOS-only and needs the app open), aphrody talks to the
//! **headless cloud API** — cross-platform, no install, pure Rust, `rustls`
//! transport, no JS runtime (CLAUDE.md §2).
//!
//! # Authentication
//!
//! OAuth **server-to-server** (client-credentials): a Developer Console project
//! supplies a `client_id` + `client_secret`, exchanged at the IMS token
//! endpoint for a 24 h bearer token. Set:
//!
//! ```text
//! FIREFLY_CLIENT_ID=<developer-console client id>
//! FIREFLY_CLIENT_SECRET=<developer-console client secret>
//! ```
//!
//! The secret is held only in memory, redacted from `Debug`, and never logged.
//!
//! # Quick start
//!
//! ```no_run
//! use aphrody_firefly::{FireflyClient, GenerateImageRequest, ContentClass, Size};
//!
//! # async fn run() -> aphrody_firefly::Result<()> {
//! // rustls provider must be installed once by the binary before this point.
//! let client = FireflyClient::from_env()?;
//! let req = GenerateImageRequest::new("a realistic illustration of a cat coding")
//!     .with_variations(2)
//!     .with_size(Size::SQUARE_2K)
//!     .with_content_class(ContentClass::Art);
//!
//! let images = client.generate_and_download(&req).await?;
//! for img in &images {
//!     img.save_to_dir(std::path::Path::new("./out"), "firefly").await?;
//! }
//! # Ok(())
//! # }
//! ```
//!
//! Live calls require real Adobe credentials; the offline test-suite covers
//! token-expiry math, request serialization, status parsing and output saving.

#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod auth;
pub mod client;
pub mod error;
pub mod models;

pub use auth::{AccessToken, ImsCredentials, FIREFLY_SCOPE, IMS_TOKEN_ENDPOINT};
pub use client::{
    FireflyClient, FireflyImage, PollConfig, FIREFLY_API_BASE, GENERATE_ASYNC_ENDPOINT,
};
pub use error::{FireflyError, Result};
pub use models::{
    AsyncJobSubmission, ContentClass, GenerateImageRequest, GenerateResult, ImageRef, JobStatus,
    JobStatusEnvelope, Output, Size,
};
