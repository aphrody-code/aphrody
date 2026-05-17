// SPDX-License-Identifier: Apache-2.0
//! # Matrix adapter
//!
//! Implements [`MessagingChannel`] for Matrix using the Client-Server API v3
//! (<https://spec.matrix.org/v1.9/client-server-api/>).
//!
//! ## Authentication
//!
//! Reads credentials from environment variables at construction time
//! ([`MatrixChannel::from_env`]):
//! - `MATRIX_HOMESERVER` — full URL of the homeserver (e.g. `https://matrix.org`).
//! - `MATRIX_ACCESS_TOKEN` — bearer access token (obtained via login or registration).
//! - `MATRIX_USER_ID` — fully-qualified Matrix user ID (e.g. `@alice:matrix.org`).
//!
//! ## Inbox streaming
//!
//! Uses `/_matrix/client/v3/sync` with `timeout=30000` (30 s) long-poll.
//! The `since` filter token is advanced after each successful sync response
//! to receive only incremental events.  The stream yields
//! [`InboundMessage`]s from `m.room.message` timeline events.

#![deny(unsafe_code)]

use std::path::Path;

use async_trait::async_trait;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use futures::Stream;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tracing::warn;
use url::Url;

use crate::{ChannelError, InboundMessage, MessageRef, MessagingChannel, RoomInfo, RoomKind};

/// Sync long-poll timeout in milliseconds (30 s).
const SYNC_TIMEOUT_MS: u64 = 30_000;

// ── Wire types ────────────────────────────────────────────────────────────────

/// `PUT /_matrix/client/v3/rooms/{roomId}/send/{eventType}/{txnId}` response.
#[derive(Debug, Deserialize)]
struct SendEventResponse {
    event_id: String,
}

/// `POST /_matrix/media/v3/upload` response.
#[derive(Debug, Deserialize)]
struct UploadResponse {
    content_uri: String,
}

/// Joined rooms section of a `/sync` response.
#[derive(Debug, Deserialize)]
struct SyncResponse {
    next_batch: String,
    rooms: Option<SyncRooms>,
}

#[derive(Debug, Deserialize)]
struct SyncRooms {
    join: Option<std::collections::HashMap<String, JoinedRoom>>,
}

#[derive(Debug, Deserialize)]
struct JoinedRoom {
    timeline: Option<RoomTimeline>,
}

#[derive(Debug, Deserialize)]
struct RoomTimeline {
    events: Option<Vec<TimelineEvent>>,
}

#[derive(Debug, Deserialize)]
struct TimelineEvent {
    event_id: String,
    sender: String,
    #[serde(rename = "type")]
    event_type: String,
    origin_server_ts: i64,
    content: Value,
}

/// `GET /_matrix/client/v3/joined_rooms` response.
#[derive(Debug, Deserialize)]
struct JoinedRoomsResponse {
    joined_rooms: Vec<String>,
}

/// `GET /_matrix/client/v3/rooms/{roomId}/state/m.room.name` response.
#[derive(Debug, Deserialize)]
struct RoomNameEvent {
    name: Option<String>,
}

/// `GET /_matrix/client/v3/rooms/{roomId}/joined_members` response.
#[derive(Debug, Deserialize)]
struct JoinedMembersResponse {
    joined: std::collections::HashMap<String, Value>,
}

// ── Adapter ───────────────────────────────────────────────────────────────────

/// Matrix [`MessagingChannel`] adapter.
///
/// Construct via [`MatrixChannel::from_env`] or [`MatrixChannel::new`].
#[derive(Debug)]
pub struct MatrixChannel {
    homeserver: Url,
    access_token: String,
    /// Fully-qualified Matrix user ID (e.g. `@bot:matrix.org`).
    /// Stored for future routing / display logic; read in tests.
    #[allow(dead_code)]
    pub(crate) user_id: String,
    client: Client,
}

impl MatrixChannel {
    /// Construct from explicit credentials.
    ///
    /// # Errors
    /// Returns [`ChannelError::InvalidUrl`] if `homeserver` is not a valid URL.
    pub fn new(
        homeserver: &str,
        access_token: impl Into<String>,
        user_id: impl Into<String>,
    ) -> Result<Self, ChannelError> {
        let homeserver = Url::parse(homeserver)?;
        Ok(Self {
            homeserver,
            access_token: access_token.into(),
            user_id: user_id.into(),
            client: Client::new(),
        })
    }

    /// Construct by reading credentials from environment variables.
    ///
    /// Reads `MATRIX_HOMESERVER`, `MATRIX_ACCESS_TOKEN`, `MATRIX_USER_ID`.
    ///
    /// # Errors
    /// Returns [`ChannelError::MissingEnvVar`] if any variable is absent or
    /// empty, or [`ChannelError::InvalidUrl`] if `MATRIX_HOMESERVER` is not a
    /// valid URL.
    pub fn from_env() -> Result<Self, ChannelError> {
        let homeserver = std::env::var("MATRIX_HOMESERVER")
            .ok()
            .filter(|s| !s.is_empty())
            .ok_or(ChannelError::MissingEnvVar("MATRIX_HOMESERVER"))?;
        let access_token = std::env::var("MATRIX_ACCESS_TOKEN")
            .ok()
            .filter(|s| !s.is_empty())
            .ok_or(ChannelError::MissingEnvVar("MATRIX_ACCESS_TOKEN"))?;
        let user_id = std::env::var("MATRIX_USER_ID")
            .ok()
            .filter(|s| !s.is_empty())
            .ok_or(ChannelError::MissingEnvVar("MATRIX_USER_ID"))?;
        Self::new(&homeserver, access_token, user_id)
    }

    /// Build a `/_matrix/client/v3/<path>` URL against this homeserver.
    fn cs_url(&self, path: &str) -> Result<Url, ChannelError> {
        // Ensure the path starts with a slash for correct joining.
        let joined = self
            .homeserver
            .join(&format!("/_matrix/client/v3/{}", path.trim_start_matches('/')))?;
        Ok(joined)
    }

    /// Build a `/_matrix/media/v3/<path>` URL.
    fn media_url(&self, path: &str) -> Result<Url, ChannelError> {
        let joined =
            self.homeserver.join(&format!("/_matrix/media/v3/{}", path.trim_start_matches('/')))?;
        Ok(joined)
    }

    /// Generate a unique transaction ID for idempotent event sending.
    /// Format: `aphrody-<unix_nanos>`.
    fn new_txn_id() -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        format!("aphrody-{nanos}")
    }

    /// Convert a Matrix `origin_server_ts` (milliseconds since epoch) to
    /// [`DateTime<Utc>`].
    fn ts_to_datetime(ts_ms: i64) -> DateTime<Utc> {
        let secs = ts_ms / 1_000;
        let nanos = ((ts_ms % 1_000) * 1_000_000) as u32;
        DateTime::from_timestamp(secs, nanos).unwrap_or_default()
    }

    /// Extract `m.text` body from a timeline event `content`.
    fn extract_text_body(content: &Value) -> String {
        content.get("body").and_then(Value::as_str).unwrap_or("").to_owned()
    }

    /// Convert a [`TimelineEvent`] with type `m.room.message` to
    /// [`InboundMessage`].
    fn timeline_event_to_inbound(event: TimelineEvent, room_id: &str) -> InboundMessage {
        let text = Self::extract_text_body(&event.content);
        InboundMessage {
            id: event.event_id,
            room: room_id.to_owned(),
            sender: event.sender,
            text,
            attachments: Vec::new(), // Media events (m.image, m.file) are a follow-up.
            timestamp: Self::ts_to_datetime(event.origin_server_ts),
        }
    }
}

#[async_trait]
impl MessagingChannel for MatrixChannel {
    fn id(&self) -> &str {
        "matrix"
    }

    async fn send_text(&self, room: &str, text: &str) -> Result<MessageRef, ChannelError> {
        let txn_id = Self::new_txn_id();
        let url = self.cs_url(&format!(
            "rooms/{}/send/m.room.message/{}",
            urlencoding_encode(room),
            txn_id
        ))?;

        #[derive(Serialize)]
        struct MsgBody<'a> {
            msgtype: &'a str,
            body: &'a str,
        }

        let resp = self
            .client
            .put(url)
            .bearer_auth(&self.access_token)
            .json(&MsgBody { msgtype: "m.text", body: text })
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ChannelError::PlatformApi {
                code: status.as_str().to_owned(),
                message: body,
            });
        }

        let payload: SendEventResponse = resp.json().await?;
        Ok(MessageRef {
            id: payload.event_id,
            room: room.to_owned(),
            channel_id: self.id().to_owned(),
        })
    }

    async fn send_file(
        &self,
        room: &str,
        path: &Path,
        mime: &str,
    ) -> Result<MessageRef, ChannelError> {
        let file_bytes = tokio::fs::read(path).await?;
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("file").to_owned();
        let file_size = file_bytes.len() as u64;

        // Step 1 — upload binary to media repo.
        let upload_url =
            self.media_url(&format!("upload?filename={}", urlencoding_encode(&filename)))?;
        let upload_resp = self
            .client
            .post(upload_url)
            .bearer_auth(&self.access_token)
            .header(reqwest::header::CONTENT_TYPE, mime)
            .body(Bytes::from(file_bytes))
            .send()
            .await?;

        if !upload_resp.status().is_success() {
            let status = upload_resp.status();
            let body = upload_resp.text().await.unwrap_or_default();
            return Err(ChannelError::PlatformApi {
                code: status.as_str().to_owned(),
                message: body,
            });
        }

        let upload: UploadResponse = upload_resp.json().await?;

        // Step 2 — send `m.file` event referencing the MXC URI.
        let txn_id = Self::new_txn_id();
        let send_url = self.cs_url(&format!(
            "rooms/{}/send/m.room.message/{}",
            urlencoding_encode(room),
            txn_id
        ))?;

        #[derive(Serialize)]
        struct FileBody<'a> {
            msgtype: &'a str,
            body: &'a str,
            url: &'a str,
            info: FileInfo<'a>,
        }
        #[derive(Serialize)]
        struct FileInfo<'a> {
            mimetype: &'a str,
            size: u64,
        }

        let send_resp = self
            .client
            .put(send_url)
            .bearer_auth(&self.access_token)
            .json(&FileBody {
                msgtype: "m.file",
                body: &filename,
                url: &upload.content_uri,
                info: FileInfo { mimetype: mime, size: file_size },
            })
            .send()
            .await?;

        if !send_resp.status().is_success() {
            let status = send_resp.status();
            let body = send_resp.text().await.unwrap_or_default();
            return Err(ChannelError::PlatformApi {
                code: status.as_str().to_owned(),
                message: body,
            });
        }

        let payload: SendEventResponse = send_resp.json().await?;
        Ok(MessageRef {
            id: payload.event_id,
            room: room.to_owned(),
            channel_id: self.id().to_owned(),
        })
    }

    async fn stream_inbox(
        &self,
    ) -> Result<impl Stream<Item = Result<InboundMessage, ChannelError>>, ChannelError> {
        let homeserver = self.homeserver.clone();
        let access_token = self.access_token.clone();
        let client = self.client.clone();

        let stream = async_stream::try_stream! {
            let mut since: Option<String> = None;

            loop {
                let mut sync_url = homeserver
                    .join("/_matrix/client/v3/sync")
                    .map_err(ChannelError::InvalidUrl)?;

                {
                    let mut query = sync_url.query_pairs_mut();
                    query.append_pair("timeout", &SYNC_TIMEOUT_MS.to_string());
                    query.append_pair("filter", r#"{"room":{"timeline":{"limit":50}}}"#);
                    if let Some(ref token) = since {
                        query.append_pair("since", token);
                    }
                }

                let resp = client
                    .get(sync_url)
                    .bearer_auth(&access_token)
                    .timeout(std::time::Duration::from_millis(SYNC_TIMEOUT_MS + 10_000))
                    .send()
                    .await
                    .map_err(ChannelError::Http)?;

                if !resp.status().is_success() {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    warn!(%status, error = %body, "Matrix /sync error");
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    continue;
                }

                let sync: SyncResponse = resp.json().await.map_err(ChannelError::Http)?;
                since = Some(sync.next_batch.clone());

                if let Some(rooms) = sync.rooms {
                    if let Some(joined) = rooms.join {
                        for (room_id, room) in joined {
                            let events = room
                                .timeline
                                .and_then(|t| t.events)
                                .unwrap_or_default();

                            for event in events {
                                if event.event_type != "m.room.message" {
                                    continue;
                                }
                                yield MatrixChannel::timeline_event_to_inbound(event, &room_id);
                            }
                        }
                    }
                }
            }
        };

        Ok(stream)
    }

    async fn rooms(&self) -> Result<Vec<RoomInfo>, ChannelError> {
        let url = self.cs_url("joined_rooms")?;
        let resp = self.client.get(url).bearer_auth(&self.access_token).send().await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(ChannelError::PlatformApi {
                code: status.as_str().to_owned(),
                message: body,
            });
        }

        let payload: JoinedRoomsResponse = resp.json().await?;

        // Fetch room name and member count for each joined room.
        // Errors on individual rooms are non-fatal; we return what we can.
        let mut result = Vec::with_capacity(payload.joined_rooms.len());
        for room_id in payload.joined_rooms {
            let name = self.fetch_room_name(&room_id).await.unwrap_or_default();
            let member_count = self.fetch_member_count(&room_id).await.ok();

            result.push(RoomInfo {
                id: room_id,
                name,
                kind: RoomKind::Channel, // Matrix rooms are always channel-like.
                member_count,
            });
        }

        Ok(result)
    }
}

impl MatrixChannel {
    /// Fetch the `m.room.name` state event for a room.
    async fn fetch_room_name(&self, room_id: &str) -> Result<String, ChannelError> {
        let url =
            self.cs_url(&format!("rooms/{}/state/m.room.name", urlencoding_encode(room_id)))?;
        let resp = self.client.get(url).bearer_auth(&self.access_token).send().await?;

        if !resp.status().is_success() {
            return Ok(String::new());
        }

        let event: RoomNameEvent = resp.json().await?;
        Ok(event.name.unwrap_or_default())
    }

    /// Fetch the joined-member count for a room.
    async fn fetch_member_count(&self, room_id: &str) -> Result<u64, ChannelError> {
        let url = self.cs_url(&format!("rooms/{}/joined_members", urlencoding_encode(room_id)))?;
        let resp = self.client.get(url).bearer_auth(&self.access_token).send().await?;

        if !resp.status().is_success() {
            return Err(ChannelError::UnexpectedResponse(format!(
                "joined_members returned HTTP {}",
                resp.status()
            )));
        }

        let payload: JoinedMembersResponse = resp.json().await?;
        Ok(payload.joined.len() as u64)
    }
}

/// Percent-encode a string for safe inclusion in URL path segments.
/// Only encodes characters outside the unreserved set (RFC 3986 §2.3).
fn urlencoding_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            // Unreserved characters — pass through.
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char);
            },
            b => {
                out.push('%');
                out.push(char::from_digit((b >> 4) as u32, 16).unwrap_or('0'));
                out.push(char::from_digit((b & 0xf) as u32, 16).unwrap_or('0'));
            },
        }
    }
    out
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(unsafe_code)]
mod tests {
    use super::*;

    #[test]
    fn from_env_fails_on_missing_homeserver() {
        unsafe { std::env::remove_var("MATRIX_HOMESERVER"); }
        unsafe { std::env::remove_var("MATRIX_ACCESS_TOKEN"); }
        unsafe { std::env::remove_var("MATRIX_USER_ID"); }
        let err = MatrixChannel::from_env().unwrap_err();
        assert!(
            matches!(err, ChannelError::MissingEnvVar("MATRIX_HOMESERVER")),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn from_env_fails_on_missing_token() {
        unsafe { std::env::set_var("MATRIX_HOMESERVER", "https://matrix.org"); }
        unsafe { std::env::remove_var("MATRIX_ACCESS_TOKEN"); }
        unsafe { std::env::remove_var("MATRIX_USER_ID"); }
        let err = MatrixChannel::from_env().unwrap_err();
        assert!(
            matches!(err, ChannelError::MissingEnvVar("MATRIX_ACCESS_TOKEN")),
            "unexpected error: {err}"
        );
        unsafe { std::env::remove_var("MATRIX_HOMESERVER"); }
    }

    #[test]
    fn from_env_reads_all_vars() {
        unsafe { std::env::set_var("MATRIX_HOMESERVER", "https://matrix.example.org"); }
        unsafe { std::env::set_var("MATRIX_ACCESS_TOKEN", "syt_token_abc"); }
        unsafe { std::env::set_var("MATRIX_USER_ID", "@bot:matrix.example.org"); }
        let ch = MatrixChannel::from_env().expect("should succeed");
        assert_eq!(ch.access_token, "syt_token_abc");
        assert_eq!(ch.user_id, "@bot:matrix.example.org");
        unsafe { std::env::remove_var("MATRIX_HOMESERVER"); }
        unsafe { std::env::remove_var("MATRIX_ACCESS_TOKEN"); }
        unsafe { std::env::remove_var("MATRIX_USER_ID"); }
    }

    #[test]
    fn matrix_channel_id_is_matrix() {
        let ch = MatrixChannel::new("https://matrix.org", "tok", "@bot:matrix.org").expect("valid");
        assert_eq!(ch.id(), "matrix");
    }

    #[test]
    fn cs_url_builds_correctly() {
        let ch = MatrixChannel::new("https://matrix.org", "tok", "@bot:matrix.org").expect("valid");
        let url = ch.cs_url("joined_rooms").expect("url");
        assert_eq!(url.as_str(), "https://matrix.org/_matrix/client/v3/joined_rooms");
    }

    #[test]
    fn cs_url_room_send_event() {
        let ch = MatrixChannel::new("https://matrix.org", "tok", "@bot:matrix.org").expect("valid");
        let url = ch.cs_url("rooms/%21roomid%3Amatrix.org/send/m.room.message/txn1").expect("url");
        assert!(url.as_str().contains("rooms/"));
        assert!(url.as_str().contains("send/m.room.message"));
    }

    #[test]
    fn ts_to_datetime_converts_ms() {
        let dt = MatrixChannel::ts_to_datetime(1_716_000_000_000);
        assert_eq!(dt.timestamp(), 1_716_000_000);
    }

    #[test]
    fn urlencoding_encode_room_id() {
        // Matrix room IDs contain '!' and ':' which must be percent-encoded.
        let encoded = urlencoding_encode("!roomid:matrix.org");
        assert_eq!(encoded, "%21roomid%3amatrix.org");
    }

    #[test]
    fn urlencoding_encode_unreserved_passthrough() {
        assert_eq!(urlencoding_encode("abc-123_FOO.bar~"), "abc-123_FOO.bar~");
    }

    #[test]
    fn extract_text_body_from_content() {
        let content = serde_json::json!({"msgtype": "m.text", "body": "Hello, Matrix!"});
        assert_eq!(MatrixChannel::extract_text_body(&content), "Hello, Matrix!");
    }

    #[test]
    fn sync_response_deserialises() {
        let json = r#"{
            "next_batch": "s72595_4483_1934",
            "rooms": {
                "join": {
                    "!726s6s6q:example.com": {
                        "timeline": {
                            "events": [
                                {
                                    "event_id": "$1476784692749PhrSn:example.com",
                                    "sender": "@alice:example.com",
                                    "type": "m.room.message",
                                    "origin_server_ts": 1476784692759,
                                    "content": {"msgtype": "m.text", "body": "hello"}
                                }
                            ]
                        }
                    }
                }
            }
        }"#;
        let sync: SyncResponse = serde_json::from_str(json).expect("deserialise");
        assert_eq!(sync.next_batch, "s72595_4483_1934");
        let joined = sync.rooms.expect("rooms").join.expect("join");
        let room = joined.get("!726s6s6q:example.com").expect("room");
        let events = room.timeline.as_ref().expect("timeline").events.as_ref().expect("events");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event_type, "m.room.message");
    }
}
