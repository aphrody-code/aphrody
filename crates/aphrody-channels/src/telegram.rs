// SPDX-License-Identifier: Apache-2.0
//! # Telegram adapter
//!
//! Implements [`MessagingChannel`] for Telegram using the Telegram Bot API
//! (<https://core.telegram.org/bots/api>).
//!
//! ## Authentication
//!
//! Reads the bot token from the `TELEGRAM_BOT_TOKEN` environment variable at
//! construction time ([`TelegramChannel::from_env`]).  The token format is
//! `<bot_id>:<random_string>` as issued by @BotFather.
//!
//! ## Inbox streaming
//!
//! Uses `getUpdates` long-polling with a 30-second `timeout` parameter,
//! advancing the `offset` by `update_id + 1` after each successful batch to
//! acknowledge processed updates and prevent re-delivery.

#![deny(unsafe_code)]

use std::path::Path;

use async_trait::async_trait;
use chrono::DateTime;
use futures::Stream;
use reqwest::{
    multipart::{Form, Part},
    Client,
};
use serde::{Deserialize, Serialize};
use tracing::warn;

use crate::{Attachment, ChannelError, InboundMessage, MessageRef, MessagingChannel, RoomInfo};

/// Long-poll timeout in seconds for `getUpdates`.
const LONG_POLL_TIMEOUT_SECS: u64 = 30;

/// Maximum number of updates to retrieve per `getUpdates` call.
const UPDATES_LIMIT: u32 = 100;

// ── Wire types ────────────────────────────────────────────────────────────────

/// Generic Bot API response envelope.
#[derive(Debug, Deserialize)]
struct TgResponse<T> {
    ok: bool,
    result: Option<T>,
    description: Option<String>,
    error_code: Option<i64>,
}

/// `sendMessage` result.
#[derive(Debug, Deserialize)]
struct TgMessage {
    message_id: i64,
    chat: TgChat,
    text: Option<String>,
    date: i64,
    from: Option<TgUser>,
    document: Option<TgDocument>,
    photo: Option<Vec<TgPhotoSize>>,
}

/// A Telegram chat (group, channel, private, supergroup).
#[derive(Debug, Deserialize)]
#[allow(dead_code)] // wire fields reserved for future display / routing logic
struct TgChat {
    id: i64,
    #[serde(rename = "type")]
    kind: String,
    title: Option<String>,
    username: Option<String>,
    first_name: Option<String>,
}

/// A Telegram user.
#[derive(Debug, Deserialize)]
#[allow(dead_code)] // `id` reserved for future user-ID-based routing
struct TgUser {
    id: i64,
    username: Option<String>,
    first_name: String,
}

/// A document (file) attachment.
#[derive(Debug, Deserialize)]
struct TgDocument {
    file_id: String,
    file_name: Option<String>,
    mime_type: Option<String>,
    file_size: Option<u64>,
}

/// One photo size entry.
#[derive(Debug, Deserialize)]
struct TgPhotoSize {
    file_id: String,
    file_size: Option<u64>,
}

/// An update from `getUpdates`.
#[derive(Debug, Deserialize)]
struct TgUpdate {
    update_id: i64,
    message: Option<TgMessage>,
    channel_post: Option<TgMessage>,
}

// ── Adapter ───────────────────────────────────────────────────────────────────

/// Telegram [`MessagingChannel`] adapter.
///
/// Construct via [`TelegramChannel::from_env`] or [`TelegramChannel::new`].
#[derive(Debug)]
pub struct TelegramChannel {
    bot_token: String,
    client: Client,
}

impl TelegramChannel {
    /// Construct from an explicit bot token string.
    pub fn new(bot_token: impl Into<String>) -> Self {
        Self { bot_token: bot_token.into(), client: Client::new() }
    }

    /// Construct by reading `TELEGRAM_BOT_TOKEN` from the environment.
    ///
    /// # Errors
    /// Returns [`ChannelError::MissingEnvVar`] if the variable is absent or empty.
    pub fn from_env() -> Result<Self, ChannelError> {
        let token = std::env::var("TELEGRAM_BOT_TOKEN")
            .ok()
            .filter(|s| !s.is_empty())
            .ok_or(ChannelError::MissingEnvVar("TELEGRAM_BOT_TOKEN"))?;
        Ok(Self::new(token))
    }

    /// Build a Bot API method URL: `https://api.telegram.org/bot<token>/<method>`.
    fn method_url(&self, method: &str) -> String {
        format!("https://api.telegram.org/bot{}/{}", self.bot_token, method)
    }

    /// POST JSON to `method` and decode the response envelope.
    async fn call<B, R>(&self, method: &str, body: &B) -> Result<R, ChannelError>
    where
        B: Serialize,
        R: for<'de> Deserialize<'de>,
    {
        let resp = self.client.post(self.method_url(method)).json(body).send().await?;
        let envelope: TgResponse<R> = resp.json().await?;
        if !envelope.ok {
            return Err(ChannelError::PlatformApi {
                code: envelope
                    .error_code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "unknown".to_owned()),
                message: envelope
                    .description
                    .unwrap_or_else(|| "Telegram API returned ok:false".to_owned()),
            });
        }
        envelope.result.ok_or_else(|| {
            ChannelError::UnexpectedResponse("Telegram returned ok:true but no result".to_owned())
        })
    }

    /// Convert a Bot-API `message_id` + `chat_id` pair to a [`MessageRef`].
    fn to_message_ref(&self, message_id: i64, chat_id: i64) -> MessageRef {
        MessageRef {
            id: message_id.to_string(),
            room: chat_id.to_string(),
            channel_id: self.id().to_owned(),
        }
    }

    /// Extract [`Attachment`]s from a [`TgMessage`].
    fn extract_attachments(msg: &TgMessage) -> Vec<Attachment> {
        let mut out = Vec::new();

        if let Some(doc) = &msg.document {
            out.push(Attachment {
                id: doc.file_id.clone(),
                filename: doc.file_name.clone().unwrap_or_else(|| "file".to_owned()),
                mime_type: doc
                    .mime_type
                    .clone()
                    .unwrap_or_else(|| "application/octet-stream".to_owned()),
                url: None, // Requires getFile + constructing cdn URL.
                size_bytes: doc.file_size,
            });
        }

        if let Some(photos) = &msg.photo {
            // Use the last (largest) photo size.
            if let Some(photo) = photos.last() {
                out.push(Attachment {
                    id: photo.file_id.clone(),
                    filename: "photo.jpg".to_owned(),
                    mime_type: "image/jpeg".to_owned(),
                    url: None,
                    size_bytes: photo.file_size,
                });
            }
        }

        out
    }

    /// Convert a [`TgMessage`] to [`InboundMessage`].
    fn tg_msg_to_inbound(msg: TgMessage, _channel_id: &str) -> InboundMessage {
        let sender = msg
            .from
            .as_ref()
            .and_then(|u| u.username.clone())
            .or_else(|| msg.from.as_ref().map(|u| u.first_name.clone()))
            .unwrap_or_else(|| msg.chat.id.to_string());

        let attachments = Self::extract_attachments(&msg);

        InboundMessage {
            id: msg.message_id.to_string(),
            room: msg.chat.id.to_string(),
            sender,
            text: msg.text.unwrap_or_default(),
            attachments,
            timestamp: DateTime::from_timestamp(msg.date, 0).unwrap_or_default(),
        }
    }
}

#[async_trait]
impl MessagingChannel for TelegramChannel {
    fn id(&self) -> &str {
        "telegram"
    }

    async fn send_text(&self, room: &str, text: &str) -> Result<MessageRef, ChannelError> {
        #[derive(Serialize)]
        struct Body<'a> {
            chat_id: &'a str,
            text: &'a str,
        }

        let msg: TgMessage = self.call("sendMessage", &Body { chat_id: room, text }).await?;

        Ok(self.to_message_ref(msg.message_id, msg.chat.id))
    }

    async fn send_file(
        &self,
        room: &str,
        path: &Path,
        mime: &str,
    ) -> Result<MessageRef, ChannelError> {
        let file_bytes = tokio::fs::read(path).await?;
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("file").to_owned();

        let part = Part::bytes(file_bytes)
            .file_name(filename)
            .mime_str(mime)
            .map_err(|e| ChannelError::UnexpectedResponse(format!("invalid MIME type: {e}")))?;

        let form = Form::new().text("chat_id", room.to_owned()).part("document", part);

        let resp = self.client.post(self.method_url("sendDocument")).multipart(form).send().await?;

        let envelope: TgResponse<TgMessage> = resp.json().await?;
        if !envelope.ok {
            return Err(ChannelError::PlatformApi {
                code: envelope
                    .error_code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "unknown".to_owned()),
                message: envelope.description.unwrap_or_else(|| "sendDocument failed".to_owned()),
            });
        }
        let msg = envelope.result.ok_or_else(|| {
            ChannelError::UnexpectedResponse("sendDocument returned no result".to_owned())
        })?;

        Ok(self.to_message_ref(msg.message_id, msg.chat.id))
    }

    async fn stream_inbox(
        &self,
    ) -> Result<impl Stream<Item = Result<InboundMessage, ChannelError>>, ChannelError> {
        let token = self.bot_token.clone();
        let client = self.client.clone();
        let channel_id = self.id().to_owned();

        let stream = async_stream::try_stream! {
            let mut offset: i64 = 0;

            loop {
                #[derive(Serialize)]
                struct GetUpdatesParams {
                    offset: i64,
                    limit: u32,
                    timeout: u64,
                }

                let url = format!("https://api.telegram.org/bot{token}/getUpdates");
                let params = GetUpdatesParams {
                    offset,
                    limit: UPDATES_LIMIT,
                    timeout: LONG_POLL_TIMEOUT_SECS,
                };

                let resp = client
                    .post(&url)
                    .json(&params)
                    .timeout(std::time::Duration::from_secs(LONG_POLL_TIMEOUT_SECS + 5))
                    .send()
                    .await
                    .map_err(ChannelError::Http)?;

                let envelope: TgResponse<Vec<TgUpdate>> =
                    resp.json().await.map_err(ChannelError::Http)?;

                if !envelope.ok {
                    warn!(
                        description = ?envelope.description,
                        "Telegram getUpdates error"
                    );
                    // Back off briefly before retrying on persistent errors.
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    continue;
                }

                let updates = envelope.result.unwrap_or_default();

                for update in updates {
                    // Advance offset to acknowledge this update.
                    if update.update_id >= offset {
                        offset = update.update_id + 1;
                    }

                    let msg = update.message.or(update.channel_post);
                    if let Some(msg) = msg {
                        yield TelegramChannel::tg_msg_to_inbound(msg, &channel_id);
                    }
                }
            }
        };

        Ok(stream)
    }

    async fn rooms(&self) -> Result<Vec<RoomInfo>, ChannelError> {
        // The Telegram Bot API does not provide a "list all chats" method.
        // Bots can only query chats they have already interacted with, and
        // only by known chat ID.  We return an empty list here; callers that
        // need room enumeration should use `getUpdates` + track seen chat IDs
        // from incoming messages.
        //
        // A production operator-side pattern is to persist chat IDs on the
        // first inbound message and query each via `getChat`.
        Ok(Vec::new())
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    use super::*;

    #[test]
    fn from_env_fails_when_variable_absent() {
        unsafe { std::env::remove_var("TELEGRAM_BOT_TOKEN"); }
        let err = TelegramChannel::from_env().unwrap_err();
        assert!(
            matches!(err, ChannelError::MissingEnvVar("TELEGRAM_BOT_TOKEN")),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn from_env_reads_token() {
        unsafe { std::env::set_var("TELEGRAM_BOT_TOKEN", "1234567890:ABC-DEF"); }
        let ch = TelegramChannel::from_env().expect("should succeed");
        assert_eq!(ch.bot_token, "1234567890:ABC-DEF");
        unsafe { std::env::remove_var("TELEGRAM_BOT_TOKEN"); }
    }

    #[test]
    fn method_url_builds_correctly() {
        let ch = TelegramChannel::new("123:ABC");
        assert_eq!(ch.method_url("sendMessage"), "https://api.telegram.org/bot123:ABC/sendMessage");
    }

    #[test]
    fn telegram_channel_id_is_telegram() {
        let ch = TelegramChannel::new("tok");
        assert_eq!(ch.id(), "telegram");
    }

    #[test]
    fn tg_response_ok_false_deserialises() {
        let json = r#"{"ok":false,"error_code":400,"description":"Bad Request: chat not found"}"#;
        let env: TgResponse<TgMessage> = serde_json::from_str(json).expect("deserialise");
        assert!(!env.ok);
        assert_eq!(env.error_code, Some(400));
        assert_eq!(env.description.as_deref(), Some("Bad Request: chat not found"));
    }

    #[test]
    fn extract_attachments_document() {
        let msg = TgMessage {
            message_id: 1,
            chat: TgChat {
                id: 42,
                kind: "private".to_owned(),
                title: None,
                username: None,
                first_name: Some("Alice".to_owned()),
            },
            text: None,
            date: 1_716_000_000,
            from: None,
            document: Some(TgDocument {
                file_id: "FILEID".to_owned(),
                file_name: Some("report.pdf".to_owned()),
                mime_type: Some("application/pdf".to_owned()),
                file_size: Some(1024),
            }),
            photo: None,
        };
        let attachments = TelegramChannel::extract_attachments(&msg);
        assert_eq!(attachments.len(), 1);
        assert_eq!(attachments[0].filename, "report.pdf");
        assert_eq!(attachments[0].mime_type, "application/pdf");
    }

    #[test]
    fn tg_msg_to_inbound_maps_fields() {
        let msg = TgMessage {
            message_id: 99,
            chat: TgChat {
                id: -100_123_456,
                kind: "supergroup".to_owned(),
                title: Some("Dev Group".to_owned()),
                username: None,
                first_name: None,
            },
            text: Some("Hello".to_owned()),
            date: 1_716_000_000,
            from: Some(TgUser {
                id: 777,
                username: Some("alice".to_owned()),
                first_name: "Alice".to_owned(),
            }),
            document: None,
            photo: None,
        };
        let inbound = TelegramChannel::tg_msg_to_inbound(msg, "telegram");
        assert_eq!(inbound.id, "99");
        assert_eq!(inbound.room, "-100123456");
        assert_eq!(inbound.sender, "alice");
        assert_eq!(inbound.text, "Hello");
        assert_eq!(inbound.timestamp.timestamp(), 1_716_000_000);
    }
}
