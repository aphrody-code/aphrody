// SPDX-License-Identifier: Apache-2.0
//! Discord voice shim — delegates synthesised audio to the openclaw Gateway.
//!
//! # Design
//!
//! The aphrody crate does **not** implement the Discord voice gateway protocol
//! (UDP/SRTP/WebSocket) directly.  Instead, [`DiscordVoiceShim`] POSTs raw
//! audio bytes to a configurable openclaw Gateway HTTP endpoint which handles
//! the Discord plumbing internally.
//!
//! # Gateway URL
//!
//! The endpoint is resolved at construction time from the `OPENCLAW_GATEWAY_URL`
//! environment variable.  The caller can also supply it explicitly via
//! [`DiscordVoiceShim::new`].
//!
//! # Wire format
//!
//! ```text
//! POST <gateway_url>/tts
//! Content-Type: audio/mpeg
//! X-Voice-Id:   <voice_id>         (optional, forwarded as-is)
//! X-Channel-Id: <channel_id>       (optional, forwarded as-is)
//!
//! <raw audio bytes>
//! ```
//!
//! A `200 OK` response (body ignored) is treated as success.

use std::env;

use bytes::Bytes;

use crate::VoiceError;

/// Environment variable consulted when no explicit URL is given.
pub const GATEWAY_URL_ENV: &str = "OPENCLAW_GATEWAY_URL";

// ── DiscordVoiceShim ──────────────────────────────────────────────────────────

/// Thin HTTP client that forwards synthesised audio to the openclaw Gateway.
///
/// Construct with [`DiscordVoiceShim::from_env`] (reads `OPENCLAW_GATEWAY_URL`)
/// or [`DiscordVoiceShim::new`] (explicit URL).
#[derive(Debug, Clone)]
pub struct DiscordVoiceShim {
    gateway_url: String,
    client: reqwest::Client,
}

impl DiscordVoiceShim {
    /// Construct from an explicit `gateway_url`.
    ///
    /// Trailing slashes are stripped so that path concatenation is consistent.
    pub fn new(gateway_url: impl Into<String>) -> Self {
        let raw = gateway_url.into();
        Self { gateway_url: raw.trim_end_matches('/').to_owned(), client: reqwest::Client::new() }
    }

    /// Construct by reading `OPENCLAW_GATEWAY_URL` from the environment.
    ///
    /// # Errors
    ///
    /// Returns [`VoiceError::MissingEnv`] when the variable is absent or not
    /// valid Unicode.
    pub fn from_env() -> Result<Self, VoiceError> {
        let url = env::var(GATEWAY_URL_ENV)
            .map_err(|source| VoiceError::MissingEnv { var: GATEWAY_URL_ENV, source })?;
        Ok(Self::new(url))
    }

    /// POST `audio` to the Gateway's `/tts` endpoint.
    ///
    /// `voice_id` and `channel_id` are forwarded as `X-Voice-Id` /
    /// `X-Channel-Id` headers so the Gateway can route to the correct Discord
    /// channel and apply voice-specific processing.
    ///
    /// # Errors
    ///
    /// - [`VoiceError::Transport`] — network or TLS failure.
    /// - [`VoiceError::Api`] — Gateway replied with a non-2xx status.
    pub async fn post_audio(
        &self,
        audio: Bytes,
        voice_id: Option<&str>,
        channel_id: Option<&str>,
    ) -> Result<(), VoiceError> {
        let url = format!("{}/tts", self.gateway_url);

        let mut req =
            self.client.post(&url).header(reqwest::header::CONTENT_TYPE, "audio/mpeg").body(audio);

        if let Some(vid) = voice_id {
            req = req.header("X-Voice-Id", vid);
        }
        if let Some(cid) = channel_id {
            req = req.header("X-Channel-Id", cid);
        }

        let resp = req.send().await?;

        let status = resp.status();
        if status.is_success() {
            return Ok(());
        }

        let code = status.as_u16();
        let body = resp.text().await.unwrap_or_else(|_| String::from("<unreadable body>"));

        Err(VoiceError::Api { status: code, body })
    }
}
