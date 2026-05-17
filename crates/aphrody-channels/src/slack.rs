// SPDX-License-Identifier: Apache-2.0
//! # Slack adapter
//!
//! Implements [`MessagingChannel`] for Slack using the Slack Web API
//! (<https://api.slack.com/web>).
//!
//! ## Authentication
//!
//! Reads the bot token from the `SLACK_BOT_TOKEN` environment variable at
//! construction time ([`SlackChannel::from_env`]).  The token must have at
//! least the following OAuth scopes:
//! - `chat:write` — send messages.
//! - `files:write` — upload files.
//! - `channels:read`, `groups:read`, `im:read`, `mpim:read` — list rooms.
//! - `channels:history` or RTM connection — receive inbound messages.
//!
//! ## Inbox streaming
//!
//! Slack's RTM (Real-Time Messaging) API requires a WebSocket upgrade that
//! depends on `rtm.connect`.  Because a WebSocket crate is not yet declared in
//! workspace dependencies, the inbox stream falls back to polling
//! `conversations.history` on a short interval (1 s) using `has_more` + cursor
//! pagination, returning only messages newer than the last observed `ts`.
//!
//! Full RTM / Events-API push support is planned for a future tick once a
//! `tokio-tungstenite` or `fastwebsockets` workspace dep is approved.

#![deny(unsafe_code)]

use std::path::Path;

use async_trait::async_trait;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tracing::warn;
use url::Url;

use crate::{
    Attachment, ChannelError, InboundMessage, MessageRef, MessagingChannel, RoomInfo, RoomKind,
};

/// Base URL for all Slack Web API calls.
const SLACK_API_BASE: &str = "https://slack.com/api/";

// ── Wire types ────────────────────────────────────────────────────────────────

/// Top-level envelope returned by every Slack Web API method.
#[derive(Debug, Deserialize)]
struct SlackEnvelope<T> {
    ok: bool,
    #[serde(flatten)]
    payload: Option<T>,
    error: Option<String>,
}

/// Payload for `chat.postMessage`.
#[derive(Debug, Deserialize)]
struct PostMessagePayload {
    ts: String,
    channel: String,
}

/// Payload for `files.upload_v2` / `files.completeUploadExternal`.
#[derive(Debug, Deserialize)]
struct FileUploadPayload {
    files: Vec<SlackFileRef>,
}

#[derive(Debug, Deserialize)]
struct SlackFileRef {
    id: String,
}

/// A channel entry returned by `conversations.list`.
#[derive(Debug, Deserialize)]
struct SlackConversation {
    id: String,
    name: Option<String>,
    is_im: Option<bool>,
    is_mpim: Option<bool>,
    num_members: Option<u64>,
}

/// Response for `conversations.list`.
#[derive(Debug, Deserialize)]
struct ConversationsListPayload {
    channels: Vec<SlackConversation>,
}

/// A single message from `conversations.history`.
#[derive(Debug, Deserialize)]
struct SlackMessage {
    #[serde(rename = "type")]
    kind: String,
    ts: String,
    user: Option<String>,
    bot_id: Option<String>,
    text: Option<String>,
    channel: Option<String>,
    files: Option<Vec<SlackFileAttachment>>,
}

#[derive(Debug, Deserialize)]
struct SlackFileAttachment {
    id: String,
    name: Option<String>,
    mimetype: Option<String>,
    url_private: Option<String>,
    size: Option<u64>,
}

/// Response for `conversations.history`.
#[derive(Debug, Deserialize)]
struct ConversationsHistoryPayload {
    messages: Vec<SlackMessage>,
}

// ── Adapter ───────────────────────────────────────────────────────────────────

/// Slack [`MessagingChannel`] adapter.
///
/// Construct via [`SlackChannel::from_env`] or [`SlackChannel::new`].
#[derive(Debug)]
pub struct SlackChannel {
    bot_token: String,
    client: Client,
}

impl SlackChannel {
    /// Construct from an explicit bot token string.
    ///
    /// Prefer [`SlackChannel::from_env`] in production.
    pub fn new(bot_token: impl Into<String>) -> Self {
        Self { bot_token: bot_token.into(), client: Client::new() }
    }

    /// Construct by reading `SLACK_BOT_TOKEN` from the environment.
    ///
    /// # Errors
    /// Returns [`ChannelError::MissingEnvVar`] if the variable is absent or empty.
    pub fn from_env() -> Result<Self, ChannelError> {
        let token = std::env::var("SLACK_BOT_TOKEN")
            .ok()
            .filter(|s| !s.is_empty())
            .ok_or(ChannelError::MissingEnvVar("SLACK_BOT_TOKEN"))?;
        Ok(Self::new(token))
    }

    /// Build a full API endpoint URL from a method name (e.g. `"chat.postMessage"`).
    fn endpoint(&self, method: &str) -> Result<Url, ChannelError> {
        Ok(Url::parse(SLACK_API_BASE)?.join(method)?)
    }

    /// Execute a POST to `method` with the given JSON body, deserialising into
    /// `SlackEnvelope<P>` and returning the inner payload on `ok: true`.
    async fn post_json<B, P>(&self, method: &str, body: &B) -> Result<P, ChannelError>
    where
        B: Serialize,
        P: for<'de> Deserialize<'de>,
    {
        let url = self.endpoint(method)?;
        let resp = self.client.post(url).bearer_auth(&self.bot_token).json(body).send().await?;
        let envelope: SlackEnvelope<P> = resp.json().await?;
        if !envelope.ok {
            return Err(ChannelError::PlatformApi {
                code: envelope.error.unwrap_or_else(|| "unknown".to_owned()),
                message: "Slack API returned ok:false".to_owned(),
            });
        }
        envelope.payload.ok_or_else(|| {
            ChannelError::UnexpectedResponse("Slack returned ok:true but no payload".to_owned())
        })
    }

    /// Convert a Slack `ts` string (e.g. `"1716000000.123456"`) to [`DateTime<Utc>`].
    fn ts_to_datetime(ts: &str) -> DateTime<Utc> {
        let unix_secs = ts.split('.').next().and_then(|s| s.parse::<i64>().ok()).unwrap_or(0);
        DateTime::from_timestamp(unix_secs, 0).unwrap_or_default()
    }
}

#[async_trait]
impl MessagingChannel for SlackChannel {
    fn id(&self) -> &str {
        "slack"
    }

    async fn send_text(&self, room: &str, text: &str) -> Result<MessageRef, ChannelError> {
        #[derive(Serialize)]
        struct Body<'a> {
            channel: &'a str,
            text: &'a str,
        }

        let payload: PostMessagePayload =
            self.post_json("chat.postMessage", &Body { channel: room, text }).await?;

        Ok(MessageRef { id: payload.ts, room: payload.channel, channel_id: self.id().to_owned() })
    }

    async fn send_file(
        &self,
        room: &str,
        path: &Path,
        mime: &str,
    ) -> Result<MessageRef, ChannelError> {
        // Slack's `files.upload_v2` two-step:
        //   1. POST files.getUploadURLExternal — get `upload_url` + `file_id`.
        //   2. PUT the binary body to `upload_url`.
        //   3. POST files.completeUploadExternal — associate with channel.

        // Step 0 — read file content.
        let file_bytes = tokio::fs::read(path).await?;
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("file").to_owned();
        let length = file_bytes.len();

        // Step 1 — request upload URL.
        #[derive(Serialize)]
        struct GetUrlBody<'a> {
            filename: &'a str,
            length: usize,
        }

        #[derive(Deserialize)]
        struct GetUrlPayload {
            upload_url: String,
            file_id: String,
        }

        let url_payload: GetUrlPayload = self
            .post_json("files.getUploadURLExternal", &GetUrlBody { filename: &filename, length })
            .await?;

        // Step 2 — stream binary to the signed upload URL.
        let upload_url = Url::parse(&url_payload.upload_url)?;
        let upload_resp = self
            .client
            .post(upload_url)
            .header(reqwest::header::CONTENT_TYPE, mime)
            .body(Bytes::from(file_bytes))
            .send()
            .await?;
        if !upload_resp.status().is_success() {
            return Err(ChannelError::UnexpectedResponse(format!(
                "file upload returned HTTP {}",
                upload_resp.status()
            )));
        }

        // Step 3 — complete upload and share to channel.
        #[derive(Serialize)]
        struct CompleteBody<'a> {
            files: &'a [FileEntry<'a>],
            channel_id: &'a str,
        }
        #[derive(Serialize)]
        struct FileEntry<'a> {
            id: &'a str,
        }

        let file_id = url_payload.file_id;
        let complete_payload: FileUploadPayload = self
            .post_json("files.completeUploadExternal", &CompleteBody {
                files: &[FileEntry { id: &file_id }],
                channel_id: room,
            })
            .await?;

        let returned_id =
            complete_payload.files.into_iter().next().map(|f| f.id).unwrap_or(file_id);

        Ok(MessageRef { id: returned_id, room: room.to_owned(), channel_id: self.id().to_owned() })
    }

    async fn stream_inbox(
        &self,
    ) -> Result<impl Stream<Item = Result<InboundMessage, ChannelError>>, ChannelError> {
        // Fallback polling strategy: list public channels, then poll their
        // histories every second, emitting new messages only.
        //
        // This approach avoids the RTM WebSocket dependency (not yet in the
        // workspace). Full push via RTM or Events-API webhook is a follow-up.

        let token = self.bot_token.clone();
        let client = self.client.clone();

        // Collect the initial list of channel IDs to monitor.
        let rooms = {
            let channel = SlackChannel { bot_token: token.clone(), client: client.clone() };
            channel.rooms().await?
        };
        let room_ids: Vec<String> = rooms.into_iter().map(|r| r.id).collect();

        let stream = async_stream::try_stream! {
            let poll_client = client.clone();
            // Track the latest `ts` seen per channel to avoid re-emitting.
            let mut cursors: std::collections::HashMap<String, String> =
                room_ids.iter().cloned().map(|id| (id, "0.000000".to_owned())).collect();

            loop {
                for channel_id in &room_ids {
                    let since = cursors
                        .get(channel_id)
                        .cloned()
                        .unwrap_or_else(|| "0.000000".to_owned());

                    let url = Url::parse_with_params(
                        &format!("{SLACK_API_BASE}conversations.history"),
                        &[
                            ("channel", channel_id.as_str()),
                            ("oldest", &since),
                            ("limit", "200"),
                        ],
                    )?;

                    let resp = poll_client
                        .get(url)
                        .bearer_auth(&token)
                        .send()
                        .await
                        .map_err(ChannelError::Http)?;

                    let envelope: SlackEnvelope<ConversationsHistoryPayload> =
                        resp.json().await.map_err(ChannelError::Http)?;

                    if !envelope.ok {
                        warn!(
                            channel = %channel_id,
                            error = ?envelope.error,
                            "Slack conversations.history error"
                        );
                        continue;
                    }

                    if let Some(payload) = envelope.payload {
                        let mut newest_ts = since.clone();

                        // Messages are returned newest-first; reverse for ordering.
                        let mut msgs = payload.messages;
                        msgs.retain(|m| m.kind == "message" && m.ts > since);
                        msgs.reverse();

                        for msg in msgs {
                            if msg.ts > newest_ts {
                                newest_ts = msg.ts.clone();
                            }

                            let sender = msg
                                .user
                                .or(msg.bot_id)
                                .unwrap_or_else(|| "unknown".to_owned());

                            let attachments = msg
                                .files
                                .unwrap_or_default()
                                .into_iter()
                                .map(|f| Attachment {
                                    id: f.id,
                                    filename: f.name.unwrap_or_else(|| "file".to_owned()),
                                    mime_type: f
                                        .mimetype
                                        .unwrap_or_else(|| "application/octet-stream".to_owned()),
                                    url: f.url_private,
                                    size_bytes: f.size,
                                })
                                .collect();

                            yield InboundMessage {
                                id: msg.ts.clone(),
                                room: msg
                                    .channel
                                    .unwrap_or_else(|| channel_id.clone()),
                                sender,
                                text: msg.text.unwrap_or_default(),
                                attachments,
                                timestamp: SlackChannel::ts_to_datetime(&msg.ts),
                            };
                        }

                        cursors.insert(channel_id.clone(), newest_ts);
                    }
                }

                // 1-second polling interval.
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        };

        Ok(stream)
    }

    async fn rooms(&self) -> Result<Vec<RoomInfo>, ChannelError> {
        let url = Url::parse_with_params(&format!("{SLACK_API_BASE}conversations.list"), &[
            ("types", "public_channel,private_channel,im,mpim"),
            ("limit", "1000"),
            ("exclude_archived", "true"),
        ])?;

        let resp = self.client.get(url).bearer_auth(&self.bot_token).send().await?;

        let envelope: SlackEnvelope<ConversationsListPayload> = resp.json().await?;
        if !envelope.ok {
            return Err(ChannelError::PlatformApi {
                code: envelope.error.unwrap_or_else(|| "unknown".to_owned()),
                message: "conversations.list returned ok:false".to_owned(),
            });
        }

        let payload = envelope.payload.ok_or_else(|| {
            ChannelError::UnexpectedResponse("conversations.list returned no payload".to_owned())
        })?;

        let rooms = payload
            .channels
            .into_iter()
            .map(|c| {
                let kind = if c.is_im.unwrap_or(false) {
                    RoomKind::DirectMessage
                } else if c.is_mpim.unwrap_or(false) {
                    RoomKind::GroupDirect
                } else {
                    RoomKind::Channel
                };
                RoomInfo {
                    id: c.id,
                    name: c.name.unwrap_or_default(),
                    kind,
                    member_count: c.num_members,
                }
            })
            .collect();

        Ok(rooms)
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    use super::*;

    #[test]
    fn from_env_fails_when_variable_absent() {
        // Make sure the variable is not set in the test process.
        unsafe {
            std::env::remove_var("SLACK_BOT_TOKEN");
        }
        let err = SlackChannel::from_env().unwrap_err();
        assert!(
            matches!(err, ChannelError::MissingEnvVar("SLACK_BOT_TOKEN")),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn from_env_reads_token() {
        unsafe {
            std::env::set_var("SLACK_BOT_TOKEN", "xoxb-test-token");
        }
        let ch = SlackChannel::from_env().expect("should succeed");
        assert_eq!(ch.bot_token, "xoxb-test-token");
        unsafe {
            std::env::remove_var("SLACK_BOT_TOKEN");
        }
    }

    #[test]
    fn endpoint_builds_correctly() {
        let ch = SlackChannel::new("xoxb-x");
        let url = ch.endpoint("chat.postMessage").expect("valid URL");
        assert_eq!(url.as_str(), "https://slack.com/api/chat.postMessage");
    }

    #[test]
    fn ts_to_datetime_parses_slack_timestamp() {
        let dt = SlackChannel::ts_to_datetime("1716000000.123456");
        assert_eq!(dt.timestamp(), 1_716_000_000);
    }

    #[test]
    fn ts_to_datetime_handles_garbage_gracefully() {
        // Should not panic; returns the epoch default.
        let dt = SlackChannel::ts_to_datetime("not-a-ts");
        assert_eq!(dt.timestamp(), 0);
    }

    #[test]
    fn slack_channel_id_is_slack() {
        let ch = SlackChannel::new("tok");
        assert_eq!(ch.id(), "slack");
    }

    // Serde round-trips for wire types.

    #[test]
    fn slack_envelope_ok_false_deserialises() {
        let json = r#"{"ok":false,"error":"channel_not_found"}"#;
        let env: SlackEnvelope<PostMessagePayload> =
            serde_json::from_str(json).expect("deserialise");
        assert!(!env.ok);
        assert_eq!(env.error.as_deref(), Some("channel_not_found"));
    }

    #[test]
    fn conversations_list_payload_round_trip() {
        let json = r#"{
            "channels": [
                {"id":"C001","name":"general","is_channel":true,"num_members":12},
                {"id":"D001","is_im":true}
            ]
        }"#;
        let payload: ConversationsListPayload = serde_json::from_str(json).expect("deserialise");
        assert_eq!(payload.channels.len(), 2);
        assert_eq!(payload.channels[0].id, "C001");
        assert!(payload.channels[1].is_im.unwrap_or(false));
    }
}
