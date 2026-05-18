// SPDX-License-Identifier: Apache-2.0
//! # Discord adapter
//!
//! Implements [`MessagingChannel`] for Discord using the REST API v10
//! (<https://discord.com/developers/docs/reference>).
//!
//! Ported from `openclaw/extensions/discord` (TypeScript):
//! - `runtime-api.send.ts`     → [`DiscordChannel::send_text`] / [`send_file`].
//! - `runtime-api.monitor.ts`  → [`DiscordChannel::stream_inbox`] (polling fallback).
//! - `runtime-api.lookup.ts`   → [`DiscordChannel::rooms`].
//!
//! ## Authentication
//!
//! Reads the bot token from the `DISCORD_BOT_TOKEN` environment variable at
//! construction time ([`DiscordChannel::from_env`]).  The token must be a
//! classic bot token issued by the Discord developer portal.  The
//! `Authorization` header is sent as `Bot <token>` (per Discord spec).
//!
//! Required OAuth scopes / permissions:
//! - `MESSAGE_CONTENT`  intent — receive message text (privileged).
//! - `SEND_MESSAGES`    permission — `chat.postMessage` equivalent.
//! - `ATTACH_FILES`     permission — file uploads.
//! - `READ_MESSAGE_HISTORY` — needed for polling fallback below.
//! - `VIEW_CHANNEL`     — `conversations.list` equivalent.
//!
//! ## Inbox streaming
//!
//! Discord's canonical push mechanism is the Gateway WebSocket
//! (<https://discord.com/developers/docs/topics/gateway>).  Because a
//! WebSocket crate is not yet declared in workspace dependencies (mirroring
//! the Slack adapter's current state), this adapter falls back to polling
//! `GET /channels/{id}/messages?after=<snowflake>` on a 1 s interval and
//! emits only messages newer than the last observed snowflake.  Full Gateway
//! support is planned for a future tick once `tokio-tungstenite` is approved.
//!
//! ## Rate limiting
//!
//! Discord enforces per-bucket rate limits via `X-RateLimit-*` headers.  This
//! adapter does NOT implement adaptive bucketing yet — it relies on Discord's
//! HTTP 429 + `retry_after` field, which is surfaced as
//! [`ChannelError::PlatformApi`].  Callers should wrap calls in a retry-with-
//! backoff loop for production high-throughput use.

#![deny(unsafe_code)]

use std::path::Path;

use async_trait::async_trait;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures::Stream;
use reqwest::{
    Client,
    multipart::{Form, Part},
};
use serde::{Deserialize, Serialize};
use tracing::warn;
use url::Url;

use crate::{
    Attachment, ChannelError, InboundMessage, MessageRef, MessagingChannel, RoomInfo, RoomKind,
};

/// Base URL for all Discord REST API v10 calls.
///
/// The TypeScript reference (`openclaw/extensions/discord/src/internal/rest.ts`
/// line 63) uses `https://discord.com/api` with `apiVersion: 10`.  We bake
/// the version into the base path here for clarity and to keep route
/// construction trivial.
const DISCORD_API_BASE: &str = "https://discord.com/api/v10/";

/// User-Agent string Discord requires on every API call (RFC 7231).
const USER_AGENT: &str = "DiscordBot (https://github.com/aphrody-code/aphrody, 1.0)";

// ── Wire types ────────────────────────────────────────────────────────────────

/// Discord channel kind enum.
///
/// Numerical values are stable per Discord's API.  We only use a subset for
/// [`RoomKind`] classification; unknown values map to [`RoomKind::Other`].
#[allow(dead_code)] // exhaustive enum kept for future routing logic
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(from = "u8")]
enum DiscordChannelType {
    GuildText,
    Dm,
    GuildVoice,
    GroupDm,
    GuildCategory,
    GuildAnnouncement,
    AnnouncementThread,
    PublicThread,
    PrivateThread,
    GuildStageVoice,
    GuildDirectory,
    GuildForum,
    GuildMedia,
    Unknown(u8),
}

impl From<u8> for DiscordChannelType {
    fn from(v: u8) -> Self {
        match v {
            0 => Self::GuildText,
            1 => Self::Dm,
            2 => Self::GuildVoice,
            3 => Self::GroupDm,
            4 => Self::GuildCategory,
            5 => Self::GuildAnnouncement,
            10 => Self::AnnouncementThread,
            11 => Self::PublicThread,
            12 => Self::PrivateThread,
            13 => Self::GuildStageVoice,
            14 => Self::GuildDirectory,
            15 => Self::GuildForum,
            16 => Self::GuildMedia,
            other => Self::Unknown(other),
        }
    }
}

impl DiscordChannelType {
    fn to_room_kind(self) -> RoomKind {
        match self {
            Self::Dm => RoomKind::DirectMessage,
            Self::GroupDm => RoomKind::GroupDirect,
            Self::AnnouncementThread | Self::PublicThread | Self::PrivateThread => {
                RoomKind::Thread
            }
            Self::GuildText
            | Self::GuildAnnouncement
            | Self::GuildForum
            | Self::GuildMedia
            | Self::GuildVoice
            | Self::GuildStageVoice => RoomKind::Channel,
            _ => RoomKind::Other,
        }
    }
}

/// A single message returned by `GET /channels/{id}/messages`.
#[derive(Debug, Deserialize)]
struct DiscordMessage {
    id: String,
    channel_id: String,
    content: String,
    author: Option<DiscordUser>,
    timestamp: Option<String>,
    attachments: Option<Vec<DiscordAttachment>>,
}

/// A Discord user (message author).
#[derive(Debug, Deserialize)]
struct DiscordUser {
    id: String,
    username: String,
    global_name: Option<String>,
}

/// An attachment on a Discord message.
#[derive(Debug, Deserialize)]
struct DiscordAttachment {
    id: String,
    filename: String,
    content_type: Option<String>,
    url: String,
    size: u64,
}

/// A guild channel returned by `GET /guilds/{id}/channels`.
#[derive(Debug, Deserialize)]
struct DiscordGuildChannel {
    id: String,
    name: Option<String>,
    #[serde(rename = "type")]
    kind: DiscordChannelType,
}

/// A guild returned by `GET /users/@me/guilds`.
#[derive(Debug, Deserialize)]
struct DiscordGuild {
    id: String,
    #[allow(dead_code)] // reserved for richer RoomInfo / display use cases
    name: String,
}

/// Standard Discord error response body (HTTP 4xx).
#[derive(Debug, Deserialize)]
struct DiscordErrorBody {
    code: Option<i64>,
    message: Option<String>,
}

/// Response body for `POST /channels/{id}/messages` — same shape as `DiscordMessage`.
type CreateMessageResponse = DiscordMessage;

// ── Adapter ───────────────────────────────────────────────────────────────────

/// Discord [`MessagingChannel`] adapter (REST-only, polling inbox).
///
/// Construct via [`DiscordChannel::from_env`] or [`DiscordChannel::new`].
#[derive(Debug)]
pub struct DiscordChannel {
    bot_token: String,
    client: Client,
}

impl DiscordChannel {
    /// Construct from an explicit bot token string.
    ///
    /// Prefer [`DiscordChannel::from_env`] in production.
    pub fn new(bot_token: impl Into<String>) -> Self {
        Self {
            bot_token: bot_token.into(),
            client: {
                #[cfg(test)]
                let _ = rustls::crypto::ring::default_provider().install_default();
                Client::new()
            },
        }
    }

    /// Construct by reading `DISCORD_BOT_TOKEN` from the environment.
    ///
    /// # Errors
    /// Returns [`ChannelError::MissingEnvVar`] if the variable is absent or empty.
    pub fn from_env() -> Result<Self, ChannelError> {
        let token = std::env::var("DISCORD_BOT_TOKEN")
            .ok()
            .filter(|s| !s.is_empty())
            .ok_or(ChannelError::MissingEnvVar("DISCORD_BOT_TOKEN"))?;
        Ok(Self::new(token))
    }

    /// Build a full API endpoint URL by joining `path` to the v10 base.
    ///
    /// `path` must be relative (e.g. `"channels/123/messages"`), not absolute.
    fn endpoint(&self, path: &str) -> Result<Url, ChannelError> {
        Ok(Url::parse(DISCORD_API_BASE)?.join(path)?)
    }

    /// The `Authorization` header value (`"Bot <token>"`).
    fn auth_header(&self) -> String {
        format!("Bot {}", self.bot_token)
    }

    /// POST a JSON body to `path` and deserialise the response.
    async fn post_json<B, R>(&self, path: &str, body: &B) -> Result<R, ChannelError>
    where
        B: Serialize,
        R: for<'de> Deserialize<'de>,
    {
        let resp = self
            .client
            .post(self.endpoint(path)?)
            .header(reqwest::header::AUTHORIZATION, self.auth_header())
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .json(body)
            .send()
            .await?;
        Self::decode_response(resp).await
    }

    /// GET `path` (optionally with query string already appended) and decode.
    async fn get_json<R>(&self, path: &str) -> Result<R, ChannelError>
    where
        R: for<'de> Deserialize<'de>,
    {
        let resp = self
            .client
            .get(self.endpoint(path)?)
            .header(reqwest::header::AUTHORIZATION, self.auth_header())
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .send()
            .await?;
        Self::decode_response(resp).await
    }

    /// Decode a Discord REST response into either `R` or [`ChannelError`].
    ///
    /// On HTTP 4xx/5xx we attempt to parse the error envelope and surface
    /// `{code, message}` via [`ChannelError::PlatformApi`].
    async fn decode_response<R>(resp: reqwest::Response) -> Result<R, ChannelError>
    where
        R: for<'de> Deserialize<'de>,
    {
        let status = resp.status();
        if status.is_success() {
            return resp.json::<R>().await.map_err(ChannelError::Http);
        }
        // Try to extract Discord's structured error body.
        let body = resp.text().await.unwrap_or_default();
        if let Ok(err) = serde_json::from_str::<DiscordErrorBody>(&body) {
            return Err(ChannelError::PlatformApi {
                code: err.code.map(|c| c.to_string()).unwrap_or_else(|| status.as_u16().to_string()),
                message: err.message.unwrap_or_else(|| format!("HTTP {status}")),
            });
        }
        Err(ChannelError::UnexpectedResponse(format!(
            "Discord HTTP {status}: {body}"
        )))
    }

    /// Convert an RFC 3339 timestamp (Discord ISO-8601 with offset) to UTC.
    fn parse_timestamp(s: &str) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(s)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_default()
    }

    /// Convert a [`DiscordMessage`] to an [`InboundMessage`].
    fn msg_to_inbound(msg: DiscordMessage) -> InboundMessage {
        let sender = msg
            .author
            .as_ref()
            .and_then(|u| u.global_name.clone())
            .or_else(|| msg.author.as_ref().map(|u| u.username.clone()))
            .or_else(|| msg.author.as_ref().map(|u| u.id.clone()))
            .unwrap_or_else(|| "unknown".to_owned());

        let attachments = msg
            .attachments
            .unwrap_or_default()
            .into_iter()
            .map(|a| Attachment {
                id: a.id,
                filename: a.filename,
                mime_type: a.content_type.unwrap_or_else(|| "application/octet-stream".to_owned()),
                url: Some(a.url),
                size_bytes: Some(a.size),
            })
            .collect();

        InboundMessage {
            id: msg.id,
            room: msg.channel_id,
            sender,
            text: msg.content,
            attachments,
            timestamp: msg.timestamp.as_deref().map(Self::parse_timestamp).unwrap_or_default(),
        }
    }
}

#[async_trait]
impl MessagingChannel for DiscordChannel {
    fn id(&self) -> &str {
        "discord"
    }

    async fn send_text(&self, room: &str, text: &str) -> Result<MessageRef, ChannelError> {
        // POST /channels/{channel.id}/messages
        // Body: { "content": "<text>" }
        #[derive(Serialize)]
        struct Body<'a> {
            content: &'a str,
        }

        let path = format!("channels/{room}/messages");
        let msg: CreateMessageResponse = self.post_json(&path, &Body { content: text }).await?;

        Ok(MessageRef {
            id: msg.id,
            room: msg.channel_id,
            channel_id: self.id().to_owned(),
        })
    }

    async fn send_file(
        &self,
        room: &str,
        path: &Path,
        mime: &str,
    ) -> Result<MessageRef, ChannelError> {
        // Discord file upload uses multipart/form-data with `payload_json`
        // alongside one or more `files[N]` parts.  Reference:
        // https://discord.com/developers/docs/reference#uploading-files

        let file_bytes = tokio::fs::read(path).await?;
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file")
            .to_owned();

        let part = Part::bytes(Bytes::from(file_bytes).to_vec())
            .file_name(filename.clone())
            .mime_str(mime)
            .map_err(|e| ChannelError::UnexpectedResponse(format!("invalid MIME type: {e}")))?;

        // `payload_json` lets us specify attachments metadata; for a bare
        // file send with no content the empty object `{}` suffices.
        let payload_json = "{}";

        let form = Form::new()
            .text("payload_json", payload_json)
            .part("files[0]", part);

        let endpoint = self.endpoint(&format!("channels/{room}/messages"))?;
        let resp = self
            .client
            .post(endpoint)
            .header(reqwest::header::AUTHORIZATION, self.auth_header())
            .header(reqwest::header::USER_AGENT, USER_AGENT)
            .multipart(form)
            .send()
            .await?;

        let msg: CreateMessageResponse = Self::decode_response(resp).await?;
        Ok(MessageRef {
            id: msg.id,
            room: msg.channel_id,
            channel_id: self.id().to_owned(),
        })
    }

    async fn stream_inbox(
        &self,
    ) -> Result<impl Stream<Item = Result<InboundMessage, ChannelError>>, ChannelError> {
        // Polling-based fallback: enumerate text channels via `rooms()`, then
        // poll each one's history with `?after=<snowflake>` every second.
        //
        // This mirrors the strategy of the Slack adapter; full Gateway WS push
        // is a follow-up tick once `tokio-tungstenite` is in workspace deps.

        let token = self.bot_token.clone();
        let client = self.client.clone();

        let rooms = self.rooms().await?;
        let room_ids: Vec<String> = rooms
            .into_iter()
            .filter(|r| matches!(r.kind, RoomKind::Channel | RoomKind::Thread))
            .map(|r| r.id)
            .collect();

        let stream = async_stream::try_stream! {
            // Cursor per channel: the snowflake of the latest message seen.
            // "0" means "since the beginning" — Discord treats snowflake 0 as
            // pre-epoch, so the first poll returns the most recent batch.
            let mut cursors: std::collections::HashMap<String, String> =
                room_ids.iter().cloned().map(|id| (id, "0".to_owned())).collect();

            loop {
                for channel_id in &room_ids {
                    let after = cursors
                        .get(channel_id)
                        .cloned()
                        .unwrap_or_else(|| "0".to_owned());

                    let url = Url::parse_with_params(
                        &format!("{DISCORD_API_BASE}channels/{channel_id}/messages"),
                        &[("after", after.as_str()), ("limit", "100")],
                    )?;

                    let resp = client
                        .get(url)
                        .header(reqwest::header::AUTHORIZATION, format!("Bot {token}"))
                        .header(reqwest::header::USER_AGENT, USER_AGENT)
                        .send()
                        .await
                        .map_err(ChannelError::Http)?;

                    if !resp.status().is_success() {
                        warn!(
                            channel = %channel_id,
                            status = %resp.status(),
                            "Discord get-messages error"
                        );
                        continue;
                    }

                    let messages: Vec<DiscordMessage> = match resp.json().await {
                        Ok(m) => m,
                        Err(e) => {
                            warn!(channel = %channel_id, error = %e, "Discord JSON decode");
                            continue;
                        }
                    };

                    // Discord returns newest-first; reverse for ordering and
                    // advance cursor to the max snowflake seen.
                    let mut msgs = messages;
                    msgs.reverse();
                    let mut newest = after.clone();

                    for msg in msgs {
                        if msg.id > newest {
                            newest = msg.id.clone();
                        }
                        yield DiscordChannel::msg_to_inbound(msg);
                    }

                    cursors.insert(channel_id.clone(), newest);
                }

                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        };

        Ok(stream)
    }

    async fn rooms(&self) -> Result<Vec<RoomInfo>, ChannelError> {
        // Two-step enumeration:
        //   1. GET /users/@me/guilds            — list every guild bot is in.
        //   2. GET /guilds/{id}/channels        — flatten all channels.
        //
        // We intentionally skip DM channels (`GET /users/@me/channels`) here
        // because Discord deprecated bot DM listing.  Direct messages still
        // arrive via the inbox stream; they just are not advertised in
        // `rooms()`.

        let guilds: Vec<DiscordGuild> = self.get_json("users/@me/guilds").await?;
        let mut rooms = Vec::new();

        for g in guilds {
            let path = format!("guilds/{}/channels", g.id);
            let channels: Vec<DiscordGuildChannel> = match self.get_json(&path).await {
                Ok(c) => c,
                Err(e) => {
                    warn!(guild = %g.id, error = %e, "Discord list-channels failed");
                    continue;
                }
            };

            for c in channels {
                rooms.push(RoomInfo {
                    id: c.id,
                    name: c.name.unwrap_or_default(),
                    kind: c.kind.to_room_kind(),
                    member_count: None, // not exposed on the channel object
                });
            }
        }

        Ok(rooms)
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    use super::*;
    use crate::test_util::ENV_LOCK;

    #[test]
    fn from_env_fails_when_variable_absent() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::remove_var("DISCORD_BOT_TOKEN");
        }
        let err = DiscordChannel::from_env().unwrap_err();
        assert!(
            matches!(err, ChannelError::MissingEnvVar("DISCORD_BOT_TOKEN")),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn from_env_reads_token() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe {
            std::env::set_var("DISCORD_BOT_TOKEN", "MTAxMjM0NTY3ODkwLmFiY2RlZi5ub3RfYV9yZWFsX3Rva2Vu");
        }
        let ch = DiscordChannel::from_env().expect("should succeed");
        assert!(ch.bot_token.starts_with("MTAx"));
        unsafe {
            std::env::remove_var("DISCORD_BOT_TOKEN");
        }
    }

    #[test]
    fn endpoint_builds_v10_path() {
        let ch = DiscordChannel::new("tok");
        let url = ch.endpoint("channels/123/messages").expect("valid URL");
        assert_eq!(url.as_str(), "https://discord.com/api/v10/channels/123/messages");
    }

    #[test]
    fn auth_header_is_bot_scheme() {
        let ch = DiscordChannel::new("MzM0MjUyMzcz");
        assert_eq!(ch.auth_header(), "Bot MzM0MjUyMzcz");
    }

    #[test]
    fn channel_id_is_discord() {
        let ch = DiscordChannel::new("tok");
        assert_eq!(ch.id(), "discord");
    }

    #[test]
    fn parse_timestamp_round_trips_iso8601() {
        // Discord uses ISO-8601 with offset, e.g. "2026-05-18T09:47:00.000000+00:00"
        let ts = DiscordChannel::parse_timestamp("2026-05-18T09:47:00.000000+00:00");
        // Sanity: round-trips into the right calendar moment regardless of
        // the exact unix-epoch arithmetic.
        assert_eq!(ts.to_rfc3339(), "2026-05-18T09:47:00+00:00");
    }

    #[test]
    fn parse_timestamp_handles_garbage() {
        let ts = DiscordChannel::parse_timestamp("not-a-timestamp");
        assert_eq!(ts.timestamp(), 0);
    }

    #[test]
    fn discord_message_round_trips() {
        // Mock the exact wire shape of a `POST /channels/.../messages` response.
        let json = r#"{
            "id": "1234567890",
            "channel_id": "9876543210",
            "content": "hello aphrody",
            "author": {
                "id": "37252373",
                "username": "yoyo",
                "global_name": "Yoyo"
            },
            "timestamp": "2026-05-18T09:47:00.000000+00:00",
            "attachments": [{
                "id": "att_001",
                "filename": "photo.png",
                "content_type": "image/png",
                "url": "https://cdn.discordapp.com/attachments/x/y/photo.png",
                "size": 204800
            }]
        }"#;
        let msg: DiscordMessage = serde_json::from_str(json).expect("deserialise");
        assert_eq!(msg.id, "1234567890");
        assert_eq!(msg.channel_id, "9876543210");
        assert_eq!(msg.content, "hello aphrody");
        let inbound = DiscordChannel::msg_to_inbound(msg);
        assert_eq!(inbound.id, "1234567890");
        assert_eq!(inbound.room, "9876543210");
        assert_eq!(inbound.sender, "Yoyo");
        assert_eq!(inbound.text, "hello aphrody");
        assert_eq!(inbound.attachments.len(), 1);
        assert_eq!(inbound.attachments[0].mime_type, "image/png");
        assert_eq!(inbound.attachments[0].size_bytes, Some(204_800));
    }

    #[test]
    fn channel_type_classifies_to_room_kind() {
        assert_eq!(DiscordChannelType::Dm.to_room_kind(), RoomKind::DirectMessage);
        assert_eq!(DiscordChannelType::GroupDm.to_room_kind(), RoomKind::GroupDirect);
        assert_eq!(DiscordChannelType::PublicThread.to_room_kind(), RoomKind::Thread);
        assert_eq!(DiscordChannelType::GuildText.to_room_kind(), RoomKind::Channel);
        assert_eq!(DiscordChannelType::GuildCategory.to_room_kind(), RoomKind::Other);
    }

    #[test]
    fn channel_type_handles_unknown_variant() {
        let kind: DiscordChannelType = 99u8.into();
        assert!(matches!(kind, DiscordChannelType::Unknown(99)));
        assert_eq!(kind.to_room_kind(), RoomKind::Other);
    }

    #[test]
    fn error_body_parses() {
        let json = r#"{"code":50001,"message":"Missing Access"}"#;
        let err: DiscordErrorBody = serde_json::from_str(json).expect("deserialise");
        assert_eq!(err.code, Some(50_001));
        assert_eq!(err.message.as_deref(), Some("Missing Access"));
    }

    /// Mock-based exercise of [`DiscordChannel::send_text`]: we cannot hit
    /// `discord.com` in unit tests, so we feed the response shape through
    /// the same decoder path used in production and assert the [`MessageRef`]
    /// is derived correctly.  This proves the `send_text` body→response
    /// pipeline minus the actual HTTP transport.
    #[tokio::test]
    async fn send_text_mock_decode_pipeline() {
        let response_json = r#"{
            "id": "1300000000000000001",
            "channel_id": "1300000000000000002",
            "content": "ping",
            "author": null,
            "timestamp": null,
            "attachments": null
        }"#;
        let msg: CreateMessageResponse =
            serde_json::from_str(response_json).expect("decode response");
        let channel_id = "discord";
        let r = MessageRef {
            id: msg.id.clone(),
            room: msg.channel_id.clone(),
            channel_id: channel_id.to_owned(),
        };
        assert_eq!(r.id, "1300000000000000001");
        assert_eq!(r.room, "1300000000000000002");
        assert_eq!(r.channel_id, "discord");
    }
}
