// SPDX-License-Identifier: Apache-2.0
//! Slack Web API connector (`chat.postMessage`, `auth.test`).
//!
//! Wire reference: <https://api.slack.com/methods/chat.postMessage>.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::connector::{Connector, Message, MessageError, MessageId, MessageResult};

/// Default Slack Web API base URL.
pub const DEFAULT_API_BASE: &str = "https://slack.com/api";

/// Slack Web API connector.
#[derive(Debug, Clone)]
pub struct SlackConnector {
    bot_token: String,
    api_base: String,
    http: Client,
}

impl SlackConnector {
    /// Build with explicit token (`xoxb-…`) and the production endpoint.
    pub fn new(bot_token: impl Into<String>) -> MessageResult<Self> {
        Self::with_base(bot_token, DEFAULT_API_BASE)
    }

    /// Build with custom base URL — used by tests against a local mock.
    pub fn with_base(
        bot_token: impl Into<String>,
        api_base: impl Into<String>,
    ) -> MessageResult<Self> {
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

    /// Read `SLACK_BOT_TOKEN` from the environment.
    pub fn from_env() -> MessageResult<Self> {
        let tok = std::env::var("SLACK_BOT_TOKEN")
            .ok()
            .filter(|s| !s.is_empty())
            .ok_or(MessageError::MissingConfig("SLACK_BOT_TOKEN"))?;
        Self::new(tok)
    }

    fn endpoint(&self, method: &str) -> String {
        format!("{}/{method}", self.api_base.trim_end_matches('/'))
    }
}

#[derive(Debug, Serialize)]
struct PostMessageBody<'a> {
    channel: &'a str,
    text: &'a str,
}

#[derive(Debug, Deserialize)]
struct PostMessageResp {
    ok: bool,
    #[serde(default)]
    error: Option<String>,
    #[serde(default)]
    ts: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    channel: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AuthTestResp {
    ok: bool,
    #[serde(default)]
    error: Option<String>,
}

#[async_trait]
impl Connector for SlackConnector {
    fn id(&self) -> &'static str {
        "slack"
    }

    async fn send(&self, msg: Message) -> MessageResult<MessageId> {
        let body = PostMessageBody { channel: &msg.to, text: &msg.body };
        let resp = self
            .http
            .post(self.endpoint("chat.postMessage"))
            .bearer_auth(&self.bot_token)
            .json(&body)
            .send()
            .await?;
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

        let parsed: PostMessageResp = resp.json().await?;
        if !parsed.ok {
            let err = parsed.error.unwrap_or_else(|| "ok=false".to_owned());
            // Slack returns `ratelimited` as an error code in addition to HTTP 429.
            if err == "ratelimited" {
                return Err(MessageError::RateLimit { retry_after_secs: 0 });
            }
            return Err(MessageError::ApiError {
                code: err.clone(),
                description: format!("Slack chat.postMessage error: {err}"),
            });
        }
        let ts = parsed
            .ts
            .ok_or_else(|| MessageError::Unexpected("chat.postMessage ok but no ts".to_owned()))?;
        Ok(MessageId(ts))
    }

    async fn health(&self) -> MessageResult<()> {
        let resp = self
            .http
            .post(self.endpoint("auth.test"))
            .bearer_auth(&self.bot_token)
            .send()
            .await?;
        if !resp.status().is_success() {
            return Err(MessageError::Http(format!("auth.test returned HTTP {}", resp.status())));
        }
        let parsed: AuthTestResp = resp.json().await?;
        if !parsed.ok {
            return Err(MessageError::ApiError {
                code: parsed.error.clone().unwrap_or_else(|| "ok=false".to_owned()),
                description: parsed.error.unwrap_or_else(|| "auth.test ok=false".to_owned()),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_concatenates_method() {
        let c = SlackConnector::new("xoxb-test").expect("build");
        assert_eq!(c.endpoint("chat.postMessage"), "https://slack.com/api/chat.postMessage");
    }

    #[test]
    fn id_is_slack() {
        let c = SlackConnector::new("xoxb-x").expect("build");
        assert_eq!(c.id(), "slack");
    }
}
