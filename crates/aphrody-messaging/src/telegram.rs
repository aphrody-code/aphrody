// SPDX-License-Identifier: Apache-2.0
//! Telegram Bot API connector.
//!
//! Wire reference: <https://core.telegram.org/bots/api#sendmessage>.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::connector::{Connector, Message, MessageError, MessageId, MessageResult, ParseMode};

/// Default base URL used when no override is supplied.
pub const DEFAULT_API_BASE: &str = "https://api.telegram.org";

/// Telegram Bot API connector.
#[derive(Debug, Clone)]
pub struct TelegramConnector {
    bot_token: String,
    api_base: String,
    http: Client,
}

impl TelegramConnector {
    /// Build a connector with an explicit token + default endpoint.
    pub fn new(bot_token: impl Into<String>) -> MessageResult<Self> {
        Self::with_base(bot_token, DEFAULT_API_BASE)
    }

    /// Build a connector pointing at a custom base URL (used by tests + self-hosted bots).
    pub fn with_base(
        bot_token: impl Into<String>,
        api_base: impl Into<String>,
    ) -> MessageResult<Self> {
        // The CLI binary installs a rustls CryptoProvider at startup; when this
        // crate is exercised from unit tests no such global setup runs, so we
        // install the default ring provider opportunistically (the call is a
        // no-op if a provider is already installed).
        #[cfg(test)]
        let _ = rustls::crypto::ring::default_provider().install_default();
        let http = Client::builder()
            .user_agent(concat!("aphrody-messaging/", env!("CARGO_PKG_VERSION")))
            .build()?;
        Ok(Self {
            bot_token: bot_token.into(),
            api_base: api_base.into(),
            http,
        })
    }

    /// Read `TELEGRAM_BOT_TOKEN` from the environment.
    pub fn from_env() -> MessageResult<Self> {
        let tok = std::env::var("TELEGRAM_BOT_TOKEN")
            .ok()
            .filter(|s| !s.is_empty())
            .ok_or(MessageError::MissingConfig("TELEGRAM_BOT_TOKEN"))?;
        Self::new(tok)
    }

    fn endpoint(&self, method: &str) -> String {
        format!("{}/bot{}/{}", self.api_base.trim_end_matches('/'), self.bot_token, method)
    }
}

#[derive(Debug, Serialize)]
struct SendMessageBody<'a> {
    chat_id: &'a str,
    text: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    parse_mode: Option<&'static str>,
}

#[derive(Debug, Deserialize)]
struct TelegramEnvelope<T> {
    ok: bool,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    error_code: Option<i64>,
    result: Option<T>,
    #[serde(default)]
    parameters: Option<TelegramResponseParameters>,
}

#[derive(Debug, Deserialize)]
struct TelegramResponseParameters {
    #[serde(default)]
    retry_after: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct SentMessage {
    message_id: i64,
}

#[derive(Debug, Deserialize)]
struct BotUser {
    #[allow(dead_code)]
    id: i64,
    #[allow(dead_code)]
    is_bot: bool,
}

fn parse_mode_str(p: ParseMode) -> &'static str {
    match p {
        ParseMode::Markdown => "MarkdownV2",
        ParseMode::Html => "HTML",
    }
}

#[async_trait]
impl Connector for TelegramConnector {
    fn id(&self) -> &'static str {
        "telegram"
    }

    async fn send(&self, msg: Message) -> MessageResult<MessageId> {
        let body = SendMessageBody {
            chat_id: &msg.to,
            text: &msg.body,
            parse_mode: msg.parse_mode.map(parse_mode_str),
        };

        let resp = self.http.post(self.endpoint("sendMessage")).json(&body).send().await?;
        let status = resp.status();

        if status.as_u16() == 429 {
            let retry_after = resp
                .headers()
                .get(reqwest::header::RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(0);
            return Err(MessageError::RateLimit { retry_after_secs: retry_after });
        }

        let env: TelegramEnvelope<SentMessage> = resp.json().await?;
        if !env.ok {
            if let Some(p) = env.parameters {
                if let Some(retry) = p.retry_after {
                    return Err(MessageError::RateLimit { retry_after_secs: retry });
                }
            }
            return Err(MessageError::ApiError {
                code: env
                    .error_code
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "ok=false".to_owned()),
                description: env.description.unwrap_or_else(|| "Telegram returned ok:false".to_owned()),
            });
        }

        let id = env
            .result
            .map(|r| r.message_id)
            .ok_or_else(|| MessageError::Unexpected("sendMessage ok but no result".to_owned()))?;
        Ok(MessageId(id.to_string()))
    }

    async fn health(&self) -> MessageResult<()> {
        let resp = self.http.get(self.endpoint("getMe")).send().await?;
        if !resp.status().is_success() {
            return Err(MessageError::Http(format!("getMe returned HTTP {}", resp.status())));
        }
        let env: TelegramEnvelope<BotUser> = resp.json().await?;
        if !env.ok {
            return Err(MessageError::ApiError {
                code: env.error_code.map(|c| c.to_string()).unwrap_or_else(|| "0".to_owned()),
                description: env.description.unwrap_or_else(|| "getMe ok=false".to_owned()),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_path_includes_bot_token() {
        let c = TelegramConnector::new("xyz").expect("build");
        assert_eq!(c.endpoint("sendMessage"), "https://api.telegram.org/botxyz/sendMessage");
    }

    #[test]
    fn parse_mode_str_maps_correctly() {
        assert_eq!(parse_mode_str(ParseMode::Markdown), "MarkdownV2");
        assert_eq!(parse_mode_str(ParseMode::Html), "HTML");
    }

    #[test]
    fn id_is_telegram() {
        let c = TelegramConnector::new("x").expect("build");
        assert_eq!(c.id(), "telegram");
    }
}
