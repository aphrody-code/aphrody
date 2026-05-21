// SPDX-License-Identifier: Apache-2.0
//! # X (formerly Twitter) adapter
//!
//! Implements [`MessagingChannel`] for X by wrapping the standalone
//! `aphrody-x` binary (see `crates/aphrody-x-client/src/main.rs`).  The
//! wrapper invokes the binary via [`tokio::process::Command`] and parses
//! its JSON stdout into our domain types.
//!
//! ## Why wrap a binary instead of linking the library?
//!
//! Per the doc comment at the top of `crates/aphrody-x-client/src/main.rs`,
//! both `agent-twitter-client 0.1.2` (upstream cornip, repo deleted) and
//! the `distrihub` fork still have an unresolved CSRF / `ct0` signing bug
//! that makes the GraphQL endpoints reject every authenticated request with
//! `{"errors":[{"message":"Could not authenticate you","code":32}]}`.
//!
//! Until a Rust lib lands with working cookie auth (we track `rig-twitter`
//! and the `edisontim` fork), the most reliable path is to shell out to
//! `aphrody-x` which at least has the patched `.domain("x.com")` calls and
//! gives us a stable JSON contract on stdout.  The contract is whatever
//! `serde_json::to_string_pretty` emits for the underlying scraper types.
//!
//! ## Authentication
//!
//! `aphrody-x` reads cookies from (in order):
//!   1. `X_COOKIE_STRING` env (full `auth_token=...; ct0=...; ...` string).
//!   2. `X_AUTH_TOKEN` + `X_CT0` envs (composed into a minimal cookie string).
//!
//! This adapter does not parse `.env` — it relies on the parent process
//! environment.  Callers who load `.env` at startup will have it inherited.
//!
//! ## Mapping to the [`MessagingChannel`] trait
//!
//! X is not a chat platform, so the trait's "room" abstraction is bent:
//!
//! - `send_text(room="self", text)` → post a new tweet (the `room` argument
//!   is ignored; future tick may use it for thread/reply chaining).
//! - `send_file(...)` → returns [`ChannelError::UnexpectedResponse`]; media
//!   uploads require the v1.1 media endpoint which is not in `aphrody-x` yet.
//! - `rooms()` → returns a single synthetic [`RoomInfo`] with id `"self"`
//!   representing the authenticated account.
//! - `stream_inbox()` → polls the authenticated user's home timeline via
//!   `aphrody-x latest <handle>` every 30 s.  The handle is read from
//!   `X_HANDLE` env (matching `aphrody-x`'s own `whoami` behavior).

#![deny(unsafe_code)]

use std::path::Path;
use std::process::Stdio;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::Stream;
use serde::Deserialize;
use tokio::process::Command;

use crate::channels::{ChannelError, InboundMessage, MessageRef, MessagingChannel, RoomInfo, RoomKind};

/// Default binary name on `PATH`.  Overridable via [`XChannel::with_binary`].
const DEFAULT_BIN: &str = "aphrody-x";

/// Polling interval for the home-timeline inbox stream (in seconds).
const INBOX_POLL_SECS: u64 = 30;

// ── Wire types ────────────────────────────────────────────────────────────────

/// Minimal shape of what `aphrody-x profile <handle>` prints on stdout.
///
/// The full structure comes from `agent_twitter_client::profile::Profile`;
/// we only deserialise the fields we surface.  Unknown fields are ignored
/// (serde's default behaviour).
#[derive(Debug, Deserialize)]
struct XProfile {
    /// Numeric user ID (e.g. `"37252373"`).
    #[allow(dead_code)] // surfaced through public re-export, reserved field
    id: String,
    /// Display name (may be empty).
    #[serde(default)]
    name: Option<String>,
    /// Handle / screen name without the leading `@`.
    #[serde(default)]
    username: Option<String>,
}

/// Minimal shape of one tweet emitted by `aphrody-x latest <handle>`.
#[derive(Debug, Deserialize)]
struct XTweet {
    /// Tweet ID snowflake.
    id: String,
    /// Tweet body text.
    #[serde(default)]
    text: Option<String>,
    /// Author handle (may be the queried handle).
    #[serde(default)]
    username: Option<String>,
    /// Unix timestamp in seconds (some scraper variants emit `created_at`
    /// as ISO-8601 instead; we try both).
    #[serde(default)]
    timestamp: Option<i64>,
    #[serde(default, rename = "created_at")]
    created_at: Option<String>,
}

/// Response shape for `aphrody-x post <text>` (`Scraper::send_tweet`).
///
/// The underlying type is `agent_twitter_client::types::TweetSendResponse`
/// which contains nested protobuf-style fields.  We only need the new tweet
/// ID, which the scraper places at `data.create_tweet.tweet_results.result.rest_id`.
#[derive(Debug, Deserialize)]
struct XPostResponse {
    #[serde(default)]
    data: Option<XPostData>,
}

#[derive(Debug, Deserialize)]
struct XPostData {
    #[serde(default, rename = "create_tweet")]
    create_tweet: Option<XCreateTweet>,
}

#[derive(Debug, Deserialize)]
struct XCreateTweet {
    #[serde(default, rename = "tweet_results")]
    tweet_results: Option<XTweetResults>,
}

#[derive(Debug, Deserialize)]
struct XTweetResults {
    #[serde(default)]
    result: Option<XTweetResultInner>,
}

#[derive(Debug, Deserialize)]
struct XTweetResultInner {
    #[serde(default, rename = "rest_id")]
    rest_id: Option<String>,
}

// ── Adapter ───────────────────────────────────────────────────────────────────

/// X / Twitter [`MessagingChannel`] adapter wrapping the `aphrody-x` binary.
///
/// Construct via [`XChannel::from_env`] (uses `aphrody-x` on `PATH`) or
/// [`XChannel::with_binary`] for a custom path (useful in tests).
#[derive(Debug, Clone)]
pub struct XChannel {
    /// Path or name of the `aphrody-x` binary.  Resolved by the OS at exec.
    binary: String,
    /// Handle used by `stream_inbox` for the home-timeline poll target.
    /// Defaults to the value of `X_HANDLE` at construction, or `"aphrody_code"`.
    handle: String,
}

impl XChannel {
    /// Construct with a custom binary path (or name on `PATH`).
    pub fn with_binary(binary: impl Into<String>) -> Self {
        let handle = std::env::var("X_HANDLE").unwrap_or_else(|_| "aphrody_code".to_owned());
        Self { binary: binary.into(), handle }
    }

    /// Construct using the default `aphrody-x` binary on `PATH`.
    ///
    /// Verifies that at least one cookie source is present in the environment
    /// before returning; this catches misconfigured deploys at construction
    /// time rather than on first send.
    ///
    /// # Errors
    /// Returns [`ChannelError::MissingEnvVar`] when neither `X_COOKIE_STRING`
    /// nor (`X_AUTH_TOKEN` + `X_CT0`) is present.
    pub fn from_env() -> Result<Self, ChannelError> {
        let has_cookie = std::env::var("X_COOKIE_STRING").map(|s| !s.is_empty()).unwrap_or(false);
        let has_pair = std::env::var("X_AUTH_TOKEN").map(|s| !s.is_empty()).unwrap_or(false)
            && std::env::var("X_CT0").map(|s| !s.is_empty()).unwrap_or(false);
        if !has_cookie && !has_pair {
            // We report the canonical pair as the missing var — it is the
            // more user-friendly composition.
            return Err(ChannelError::MissingEnvVar("X_AUTH_TOKEN"));
        }
        Ok(Self::with_binary(DEFAULT_BIN))
    }

    /// Override the polling-target handle used by [`Self::stream_inbox`].
    pub fn with_handle(mut self, handle: impl Into<String>) -> Self {
        self.handle = handle.into();
        self
    }

    /// Run `aphrody-x <args...>` and return its stdout on exit code 0.
    ///
    /// stderr is logged on failure but not surfaced (the binary writes
    /// human-friendly error strings; the JSON contract is stdout-only).
    async fn run(&self, args: &[&str]) -> Result<String, ChannelError> {
        let output = Command::new(&self.binary)
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(ChannelError::Io)?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            return Err(ChannelError::PlatformApi {
                code: output.status.code().map(|c| c.to_string()).unwrap_or_else(|| "signal".into()),
                message: format!("{} {args:?} failed: {stderr}", self.binary),
            });
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

#[async_trait]
impl MessagingChannel for XChannel {
    fn id(&self) -> &str {
        "x"
    }

    /// Post a new tweet.  `room` is currently ignored; reply / thread
    /// chaining is a follow-up tick.
    async fn send_text(&self, _room: &str, text: &str) -> Result<MessageRef, ChannelError> {
        let stdout = self.run(&["post", text]).await?;
        let resp: XPostResponse = serde_json::from_str(&stdout).map_err(|e| {
            ChannelError::UnexpectedResponse(format!("could not parse aphrody-x post output: {e}"))
        })?;
        let id = resp
            .data
            .and_then(|d| d.create_tweet)
            .and_then(|c| c.tweet_results)
            .and_then(|r| r.result)
            .and_then(|inner| inner.rest_id)
            .ok_or_else(|| {
                ChannelError::UnexpectedResponse(
                    "aphrody-x post response missing data.create_tweet.tweet_results.result.rest_id"
                        .to_owned(),
                )
            })?;

        Ok(MessageRef {
            id,
            room: "self".to_owned(),
            channel_id: self.id().to_owned(),
        })
    }

    /// Media upload is not yet supported by `aphrody-x`.  Returns
    /// [`ChannelError::UnexpectedResponse`] with an explicit reason.
    async fn send_file(
        &self,
        _room: &str,
        _path: &Path,
        _mime: &str,
    ) -> Result<MessageRef, ChannelError> {
        Err(ChannelError::UnexpectedResponse(
            "X adapter: media upload not implemented (aphrody-x lacks `media` subcommand)"
                .to_owned(),
        ))
    }

    /// Polls the user's recent tweets via `aphrody-x latest <handle>`.
    ///
    /// Each new tweet (snowflake ID greater than the last seen) is yielded
    /// as an [`InboundMessage`].  The poll interval is [`INBOX_POLL_SECS`].
    async fn stream_inbox(
        &self,
    ) -> Result<impl Stream<Item = Result<InboundMessage, ChannelError>>, ChannelError> {
        let binary = self.binary.clone();
        let handle = self.handle.clone();

        let stream = async_stream::try_stream! {
            let mut latest_id: Option<String> = None;

            loop {
                let output = Command::new(&binary)
                    .args(["latest", &handle, "--count", "20"])
                    .stdin(Stdio::null())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .output()
                    .await
                    .map_err(ChannelError::Io)?;

                if !output.status.success() {
                    // Surface as a transient error item; do not break the stream.
                    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                    Err(ChannelError::PlatformApi {
                        code: output.status.code().map(|c| c.to_string()).unwrap_or_else(|| "signal".into()),
                        message: format!("aphrody-x latest failed: {stderr}"),
                    })?;
                }

                let stdout = String::from_utf8_lossy(&output.stdout);
                let tweets: Vec<XTweet> = serde_json::from_str(&stdout).map_err(|e| {
                    ChannelError::UnexpectedResponse(format!("aphrody-x latest decode: {e}"))
                })?;

                // Reverse to oldest-first so the cursor advances monotonically.
                let mut sorted = tweets;
                sorted.sort_by(|a, b| a.id.cmp(&b.id));

                for t in sorted {
                    let cursor_advanced = latest_id.as_deref().is_none_or(|cur| t.id.as_str() > cur);
                    if !cursor_advanced {
                        continue;
                    }
                    latest_id = Some(t.id.clone());

                    let timestamp = t
                        .timestamp
                        .and_then(|s| DateTime::from_timestamp(s, 0))
                        .or_else(|| {
                            t.created_at.as_deref().and_then(|s| {
                                DateTime::parse_from_rfc3339(s)
                                    .map(|dt| dt.with_timezone(&Utc))
                                    .ok()
                            })
                        })
                        .unwrap_or_default();

                    yield InboundMessage {
                        id: t.id,
                        room: "self".to_owned(),
                        sender: t.username.unwrap_or_else(|| handle.clone()),
                        text: t.text.unwrap_or_default(),
                        attachments: Vec::new(),
                        timestamp,
                    };
                }

                tokio::time::sleep(std::time::Duration::from_secs(INBOX_POLL_SECS)).await;
            }
        };

        Ok(stream)
    }

    /// Returns a single synthetic room representing the authenticated account.
    ///
    /// Calls `aphrody-x profile <handle>` to populate the room name with the
    /// account's display name (or username, or id).  If the call fails we
    /// still return a usable stub so callers can iterate without breaking.
    async fn rooms(&self) -> Result<Vec<RoomInfo>, ChannelError> {
        // Best-effort profile fetch.
        let name = match self.run(&["profile", &self.handle]).await {
            Ok(stdout) => match serde_json::from_str::<XProfile>(&stdout) {
                Ok(p) => p.name.or(p.username).unwrap_or_else(|| self.handle.clone()),
                Err(_) => self.handle.clone(),
            },
            Err(_) => self.handle.clone(),
        };

        Ok(vec![RoomInfo {
            id: "self".to_owned(),
            name,
            kind: RoomKind::DirectMessage, // closest semantic match (1:1 author = audience)
            member_count: None,
        }])
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    use super::*;
    use crate::channels::test_util::ENV_LOCK;

    #[test]
    fn from_env_fails_without_cookies() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::remove_var("X_COOKIE_STRING");
            std::env::remove_var("X_AUTH_TOKEN");
            std::env::remove_var("X_CT0");
        }
        let err = XChannel::from_env().unwrap_err();
        assert!(matches!(err, ChannelError::MissingEnvVar("X_AUTH_TOKEN")));
    }

    #[test]
    fn from_env_accepts_cookie_string() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("X_COOKIE_STRING", "auth_token=abc; ct0=xyz");
            std::env::remove_var("X_AUTH_TOKEN");
            std::env::remove_var("X_CT0");
        }
        let ch = XChannel::from_env().expect("should succeed");
        assert_eq!(ch.id(), "x");
        unsafe {
            std::env::remove_var("X_COOKIE_STRING");
        }
    }

    #[test]
    fn from_env_accepts_token_pair() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::remove_var("X_COOKIE_STRING");
            std::env::set_var("X_AUTH_TOKEN", "tok");
            std::env::set_var("X_CT0", "csrf");
        }
        let ch = XChannel::from_env().expect("should succeed");
        assert_eq!(ch.id(), "x");
        unsafe {
            std::env::remove_var("X_AUTH_TOKEN");
            std::env::remove_var("X_CT0");
        }
    }

    #[test]
    fn with_handle_overrides_default() {
        let ch = XChannel::with_binary("aphrody-x").with_handle("yoyo_test");
        assert_eq!(ch.handle, "yoyo_test");
    }

    #[test]
    fn x_channel_id_is_x() {
        let ch = XChannel::with_binary("aphrody-x");
        assert_eq!(ch.id(), "x");
    }

    #[test]
    fn post_response_extracts_rest_id() {
        let json = r#"{
            "data": {
                "create_tweet": {
                    "tweet_results": {
                        "result": {
                            "rest_id": "1800000000000000001"
                        }
                    }
                }
            }
        }"#;
        let resp: XPostResponse = serde_json::from_str(json).expect("deserialise");
        let id = resp
            .data
            .and_then(|d| d.create_tweet)
            .and_then(|c| c.tweet_results)
            .and_then(|r| r.result)
            .and_then(|i| i.rest_id);
        assert_eq!(id.as_deref(), Some("1800000000000000001"));
    }

    #[test]
    fn x_tweet_deserialises_with_unix_timestamp() {
        let json = r#"{
            "id": "1800000000000000001",
            "text": "hello world",
            "username": "aphrody_code",
            "timestamp": 1779443220
        }"#;
        let t: XTweet = serde_json::from_str(json).expect("deserialise");
        assert_eq!(t.id, "1800000000000000001");
        assert_eq!(t.text.as_deref(), Some("hello world"));
        assert_eq!(t.timestamp, Some(1_779_443_220));
    }

    #[test]
    fn x_tweet_deserialises_with_created_at() {
        let json = r#"{
            "id": "1800000000000000002",
            "text": "second tweet",
            "created_at": "2026-05-18T09:47:00.000000+00:00"
        }"#;
        let t: XTweet = serde_json::from_str(json).expect("deserialise");
        assert_eq!(t.id, "1800000000000000002");
        assert_eq!(t.created_at.as_deref(), Some("2026-05-18T09:47:00.000000+00:00"));
        assert!(t.timestamp.is_none());
    }

    #[test]
    fn x_profile_deserialises_minimal() {
        let json = r#"{"id":"37252373","name":"Yoyo","username":"aphrody_code"}"#;
        let p: XProfile = serde_json::from_str(json).expect("deserialise");
        assert_eq!(p.name.as_deref(), Some("Yoyo"));
        assert_eq!(p.username.as_deref(), Some("aphrody_code"));
    }

    /// Mock-based exercise of [`XChannel::send_text`]: we invoke a binary
    /// guaranteed to fail (`/nonexistent/aphrody-x-not-real`) and assert the
    /// error path returns a [`ChannelError::Io`] rather than panicking.  This
    /// covers the `Command::output().await` plumbing without needing a live
    /// X account.
    #[tokio::test]
    async fn send_text_fails_cleanly_on_missing_binary() {
        let ch = XChannel::with_binary("/definitely/not/a/real/binary/aphrody-x-12345");
        let err = ch.send_text("self", "test tweet").await.unwrap_err();
        assert!(matches!(err, ChannelError::Io(_)), "got: {err}");
    }

    /// Mock-based exercise of [`XChannel::send_file`]: confirms the "not
    /// implemented" path returns a structured error rather than crashing.
    #[tokio::test]
    async fn send_file_returns_unimplemented() {
        let ch = XChannel::with_binary("aphrody-x");
        let err = ch
            .send_file("self", Path::new("/tmp/nope.png"), "image/png")
            .await
            .unwrap_err();
        match err {
            ChannelError::UnexpectedResponse(s) => {
                assert!(s.contains("media upload not implemented"));
            }
            other => panic!("expected UnexpectedResponse, got {other}"),
        }
    }
}
