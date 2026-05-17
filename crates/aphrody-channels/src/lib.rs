// SPDX-License-Identifier: Apache-2.0
//! # aphrody-channels
//!
//! Unified messaging-channel trait and production adapters for:
//! - **Slack** — Bot API (chat.postMessage, files.upload_v2, conversations.list, RTM/Events-API
//!   long-poll via getUpdates-style loop).
//! - **Telegram** — Bot API (sendMessage, sendDocument, getUpdates long-poll, getChat /
//!   getMyCommands for room listing).
//! - **Matrix** — Client-Server API v3 (/_matrix/client/v3/) with /sync long-poll.
//!
//! ## Next-tick TODOs (documented, not implemented)
//!
//! The following adapters are planned for future ticks, mirroring the full
//! openclaw extension catalogue (highest-LOC first after the starter three):
//!
//! - `discord` — Discord Bot Gateway (WebSocket + REST).
//! - `whatsapp` — WhatsApp Cloud API (Meta Business Platform).
//! - `signal` — Signal Messenger (signal-cli JSON RPC).
//! - `imessage` — iMessage (macOS-only AppleScript / BlueBubbles REST).
//! - `irc` — IRC (tokio-based plain/TLS socket, RFC 1459 + 2812).
//! - `msteams` — Microsoft Teams (Graph API + Webhook).
//! - `feishu` — Feishu / Lark (Bytedance IM REST + WebSocket).
//! - `line` — LINE Messaging API (webhook + reply token).
//! - `mattermost` — Mattermost (REST v4 + WebSocket).
//! - `nextcloud` — Nextcloud Talk (spreed API).
//! - `nostr` — Nostr protocol (NIP-01 relay WebSocket).
//! - `synology` — Synology Chat (incoming webhook + slash commands).
//! - `tlon` — Tlon / Urbit (landscape REST shim).
//! - `twitch` — Twitch Chat (IRC-over-WebSocket + EventSub).
//! - `zalo` — Zalo OA API.
//! - `wechat` — WeChat Work (WeCom) REST API.
//! - `qq` — QQ Bot (QQNT guild/channel API).
//! - `webchat` — Generic webhook-based web chat embed.

#![deny(unsafe_code)]

pub mod matrix;
pub mod slack;
pub mod telegram;

use std::path::Path;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::Stream;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ── Domain types ─────────────────────────────────────────────────────────────

/// A resolved reference to a successfully sent or retrieved message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageRef {
    /// Platform-native message identifier (e.g. Slack `ts`, Telegram
    /// `message_id`, Matrix `$eventId`).
    pub id: String,
    /// The room / channel / chat the message belongs to.
    pub room: String,
    /// The [`MessagingChannel::id`] of the adapter that produced this ref.
    pub channel_id: String,
}

/// Attachment metadata for an inbound message (file, image, audio, …).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Attachment {
    /// Platform-native file identifier.
    pub id: String,
    /// Original filename as reported by the platform.
    pub filename: String,
    /// MIME type reported by the platform (may be `application/octet-stream`).
    pub mime_type: String,
    /// Download URL, if the platform exposes one.
    pub url: Option<String>,
    /// File size in bytes, if known.
    pub size_bytes: Option<u64>,
}

/// A message received from a channel (inbox event).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InboundMessage {
    /// Platform-native message identifier.
    pub id: String,
    /// Room / channel / chat the message was sent to.
    pub room: String,
    /// Sender display name or user ID as reported by the platform.
    pub sender: String,
    /// Plain-text body of the message (may be empty for media-only messages).
    pub text: String,
    /// Any file or media attachments bundled with the message.
    pub attachments: Vec<Attachment>,
    /// UTC timestamp the platform assigned to this message.
    pub timestamp: DateTime<Utc>,
}

/// Metadata about a room / channel / chat the bot participates in.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RoomInfo {
    /// Platform-native room identifier.
    pub id: String,
    /// Human-readable room name (may be empty for DM rooms).
    pub name: String,
    /// Coarse classification of the room kind.
    pub kind: RoomKind,
    /// Member count as reported by the platform; `None` when unavailable.
    pub member_count: Option<u64>,
}

/// Coarse classification of a room type across platforms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoomKind {
    /// A public or private channel / room visible to multiple members.
    Channel,
    /// A one-on-one direct message conversation.
    DirectMessage,
    /// A group direct message with more than two participants.
    GroupDirect,
    /// A forum thread or topic sub-channel.
    Thread,
    /// Any kind not covered by the variants above.
    Other,
}

// ── Error ─────────────────────────────────────────────────────────────────────

/// Errors returned by [`MessagingChannel`] operations.
#[derive(Debug, Error)]
pub enum ChannelError {
    /// Required environment variable is absent or empty.
    #[error("missing environment variable: {0}")]
    MissingEnvVar(&'static str),

    /// HTTP transport error (connection refused, timeout, TLS failure, …).
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// The platform returned an unexpected or unparseable response body.
    #[error("unexpected response: {0}")]
    UnexpectedResponse(String),

    /// JSON (de)serialisation failure.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// URL parsing / construction error.
    #[error("invalid URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    /// File I/O error during attachment upload.
    #[error("file I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The platform returned an explicit API-level error.
    #[error("platform API error ({code}): {message}")]
    PlatformApi { code: String, message: String },
}

// ── Core trait ────────────────────────────────────────────────────────────────

/// Unified abstraction over a messaging platform (Slack, Telegram, Matrix, …).
///
/// All methods are `async` and cancel-safe: dropping the future before it
/// resolves will not leave the channel in an inconsistent state.
///
/// ## Object safety
///
/// Because `stream_inbox` returns an `impl Stream` (an opaque concrete type),
/// this trait is **not** object-safe as written.  Callers needing dynamic
/// dispatch should box the stream return value inside a concrete wrapper or use
/// an enum dispatch pattern.
#[async_trait]
pub trait MessagingChannel: Send + Sync {
    /// Stable, unique identifier for this channel instance (e.g. `"slack"`,
    /// `"telegram"`, `"matrix"`).  Used as the `channel_id` in [`MessageRef`].
    fn id(&self) -> &str;

    /// Send a plain-text message to the specified room.
    ///
    /// # Errors
    /// Returns [`ChannelError`] on transport failure, rate-limit rejection, or
    /// when the room identifier is invalid.
    async fn send_text(&self, room: &str, text: &str) -> Result<MessageRef, ChannelError>;

    /// Upload a local file to the specified room.
    ///
    /// `path` must be readable.  `mime` is passed as the `Content-Type` hint
    /// to the platform (e.g. `"image/png"`, `"application/pdf"`).
    ///
    /// # Errors
    /// Returns [`ChannelError`] on I/O failure, transport failure, or when
    /// the room identifier is invalid.
    async fn send_file(
        &self,
        room: &str,
        path: &Path,
        mime: &str,
    ) -> Result<MessageRef, ChannelError>;

    /// Open a continuous stream of inbound messages for this channel.
    ///
    /// Each adapter uses its platform's idiomatic push mechanism:
    /// - **Slack**: RTM WebSocket (preferred) or Events-API long-poll fallback.
    /// - **Telegram**: `getUpdates` long-poll (Bot API).
    /// - **Matrix**: `/sync` long-poll with incremental `since` token.
    ///
    /// The returned stream yields `Result<InboundMessage, ChannelError>` items;
    /// transient errors (network blip) surface as `Err` items rather than
    /// terminating the stream, so callers can decide whether to retry.
    ///
    /// # Errors
    /// The outer `Result` is `Err` only if the stream cannot be *initialised*
    /// (e.g. missing credentials, unreachable homeserver).  Individual delivery
    /// failures are `Err` items inside the stream.
    async fn stream_inbox(
        &self,
    ) -> Result<impl Stream<Item = Result<InboundMessage, ChannelError>>, ChannelError>;

    /// List all rooms / chats / channels the bot is a member of.
    ///
    /// # Errors
    /// Returns [`ChannelError`] on transport failure or invalid credentials.
    async fn rooms(&self) -> Result<Vec<RoomInfo>, ChannelError>;
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── MessageRef serde ─────────────────────────────────────────────────────

    #[test]
    fn message_ref_round_trip() {
        let original = MessageRef {
            id: "1234567890.123456".to_owned(),
            room: "C0123456789".to_owned(),
            channel_id: "slack".to_owned(),
        };
        let json = serde_json::to_string(&original).expect("serialise MessageRef");
        let decoded: MessageRef = serde_json::from_str(&json).expect("deserialise MessageRef");
        assert_eq!(original, decoded);
    }

    // ── InboundMessage serde ─────────────────────────────────────────────────

    #[test]
    fn inbound_message_round_trip() {
        let original = InboundMessage {
            id: "msg_001".to_owned(),
            room: "!roomid:matrix.org".to_owned(),
            sender: "@alice:matrix.org".to_owned(),
            text: "Hello, world!".to_owned(),
            attachments: vec![Attachment {
                id: "att_001".to_owned(),
                filename: "photo.png".to_owned(),
                mime_type: "image/png".to_owned(),
                url: Some("https://example.org/photo.png".to_owned()),
                size_bytes: Some(204_800),
            }],
            timestamp: DateTime::from_timestamp(1_716_000_000, 0).expect("valid unix timestamp"),
        };
        let json = serde_json::to_string(&original).expect("serialise InboundMessage");
        let decoded: InboundMessage =
            serde_json::from_str(&json).expect("deserialise InboundMessage");
        assert_eq!(original, decoded);
    }

    // ── RoomInfo serde ───────────────────────────────────────────────────────

    #[test]
    fn room_info_round_trip() {
        let original = RoomInfo {
            id: "-1001234567890".to_owned(),
            name: "My Group".to_owned(),
            kind: RoomKind::Channel,
            member_count: Some(42),
        };
        let json = serde_json::to_string(&original).expect("serialise RoomInfo");
        let decoded: RoomInfo = serde_json::from_str(&json).expect("deserialise RoomInfo");
        assert_eq!(original, decoded);
    }

    // ── RoomKind serde variants ───────────────────────────────────────────────

    #[test]
    fn room_kind_serialises_as_snake_case() {
        assert_eq!(
            serde_json::to_string(&RoomKind::DirectMessage).expect("serialise"),
            "\"direct_message\""
        );
        assert_eq!(
            serde_json::to_string(&RoomKind::GroupDirect).expect("serialise"),
            "\"group_direct\""
        );
    }

    // ── ChannelError display ──────────────────────────────────────────────────

    #[test]
    fn channel_error_missing_env_display() {
        let err = ChannelError::MissingEnvVar("SLACK_BOT_TOKEN");
        assert_eq!(err.to_string(), "missing environment variable: SLACK_BOT_TOKEN");
    }

    #[test]
    fn channel_error_platform_api_display() {
        let err = ChannelError::PlatformApi {
            code: "channel_not_found".to_owned(),
            message: "No channel matching the ID was found.".to_owned(),
        };
        assert!(err.to_string().contains("channel_not_found"));
    }
}
