// SPDX-License-Identifier: Apache-2.0
//! Connector trait + shared message types.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Outbound message payload normalised across every connector.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    /// Destination — connector-specific (channel id, chat id, email address…).
    pub to: String,
    /// Optional subject line (used by email; ignored by chat connectors).
    pub subject: Option<String>,
    /// Plain-text body of the message.
    pub body: String,
    /// Optional sender override (e.g. `From:` address; ignored by chat).
    pub from: Option<String>,
    /// Optional parse mode hint — connectors interpret if relevant
    /// (Telegram `parse_mode`, Slack message formatting flags).
    pub parse_mode: Option<ParseMode>,
}

impl Message {
    /// Build a minimal text message addressed to `to`.
    pub fn text(to: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            to: to.into(),
            subject: None,
            body: body.into(),
            from: None,
            parse_mode: None,
        }
    }

    /// Set the subject (email-only).
    pub fn with_subject(mut self, subject: impl Into<String>) -> Self {
        self.subject = Some(subject.into());
        self
    }

    /// Set the From: header (email-only) or sender label.
    pub fn with_from(mut self, from: impl Into<String>) -> Self {
        self.from = Some(from.into());
        self
    }

    /// Set the parse-mode hint (Markdown / HTML).
    pub fn with_parse_mode(mut self, mode: ParseMode) -> Self {
        self.parse_mode = Some(mode);
        self
    }
}

/// Subset of body-format hints all chat platforms understand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum ParseMode {
    /// CommonMark-ish (Slack mrkdwn, Telegram `MarkdownV2`).
    Markdown,
    /// HTML subset (Telegram `HTML`).
    Html,
}

/// Opaque platform-native identifier returned by a successful send.
///
/// - **Telegram** : decimal `message_id` (e.g. `"42"`).
/// - **Slack**    : `ts` string (e.g. `"1716000000.001234"`).
/// - **SMTP**     : RFC 5322 `Message-ID` (e.g. `"<uuid@host>"`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MessageId(pub String);

impl std::fmt::Display for MessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Common error surface for all messaging connectors.
#[derive(Debug, Error)]
pub enum MessageError {
    /// Required configuration value (env var, field…) is missing.
    #[error("missing configuration: {0}")]
    MissingConfig(&'static str),

    /// HTTP transport error (DNS, connect, TLS, decode).
    #[error("HTTP transport error: {0}")]
    Http(String),

    /// The remote API returned an explicit error envelope.
    #[error("API error: {description}")]
    ApiError {
        /// Provider-specific status string (`"channel_not_found"`, `"Bad Request"`…).
        code: String,
        /// Human-readable explanation lifted from the response.
        description: String,
    },

    /// HTTP 429 / provider rate-limit response.
    #[error("rate limited; retry after {retry_after_secs}s")]
    RateLimit {
        /// Number of seconds the server asks us to wait before retrying.
        retry_after_secs: u64,
    },

    /// JSON (de)serialisation failure.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// I/O failure (raw socket / file read).
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// SMTP protocol-level error (rejected by remote MTA).
    #[error("SMTP protocol error: {code} {message}")]
    Smtp {
        /// 3-digit SMTP reply code (e.g. `550`).
        code: u16,
        /// Server-supplied text following the code.
        message: String,
    },

    /// URL parsing failure.
    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    /// Catch-all for unexpected payload shapes.
    #[error("unexpected response: {0}")]
    Unexpected(String),
}

impl From<reqwest::Error> for MessageError {
    fn from(err: reqwest::Error) -> Self {
        Self::Http(err.to_string())
    }
}

impl From<url::ParseError> for MessageError {
    fn from(err: url::ParseError) -> Self {
        Self::InvalidUrl(err.to_string())
    }
}

/// Convenience alias.
pub type MessageResult<T> = Result<T, MessageError>;

/// Object-safe trait implemented by every messaging connector.
///
/// Dyn-compatible: `Box<dyn Connector>` and `&dyn Connector` are both legal.
#[async_trait]
pub trait Connector: Send + Sync {
    /// Stable connector identifier (`"telegram"`, `"slack"`, `"smtp"`).
    fn id(&self) -> &'static str;

    /// Send `msg`, returning the platform-native message id on success.
    async fn send(&self, msg: Message) -> MessageResult<MessageId>;

    /// Liveness check — issues a cheap read-only call to the provider.
    async fn health(&self) -> MessageResult<()>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_builder_chains() {
        let m = Message::text("@yoyo", "hello")
            .with_subject("hi")
            .with_from("bot@x")
            .with_parse_mode(ParseMode::Markdown);
        assert_eq!(m.to, "@yoyo");
        assert_eq!(m.body, "hello");
        assert_eq!(m.subject.as_deref(), Some("hi"));
        assert_eq!(m.from.as_deref(), Some("bot@x"));
        assert_eq!(m.parse_mode, Some(ParseMode::Markdown));
    }

    #[test]
    fn message_id_display_returns_inner() {
        let id = MessageId("abc.123".to_owned());
        assert_eq!(id.to_string(), "abc.123");
    }

    #[test]
    fn parse_mode_serialises_pascal_case() {
        let j = serde_json::to_string(&ParseMode::Markdown).expect("ser");
        assert_eq!(j, "\"Markdown\"");
    }
}
