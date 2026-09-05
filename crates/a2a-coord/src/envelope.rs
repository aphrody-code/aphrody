// SPDX-License-Identifier: Apache-2.0
use chrono::{SecondsFormat, Utc};
use getrandom::getrandom;
use serde::{Deserialize, Serialize};

/// Envelope kind — file channel + HTTP `/msg`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EnvelopeKind {
    Ping,
    Ask,
    Fact,
    Ack,
    #[serde(rename = "error")]
    Error,
}

impl EnvelopeKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ping => "ping",
            Self::Ask => "ask",
            Self::Fact => "fact",
            Self::Ack => "ack",
            Self::Error => "error",
        }
    }
}

/// JSONL mailbox envelope (v1). Legacy `type`/`subject` aliases accepted on read.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Envelope {
    #[serde(default)]
    pub v: Option<u32>,
    #[serde(default)]
    pub ts: String,
    pub id: String,
    pub from: String,
    #[serde(default)]
    pub to: Option<String>,
    #[serde(alias = "type", default)]
    pub kind: String,
    #[serde(alias = "subject", default)]
    pub topic: String,
    #[serde(default)]
    pub body: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub re: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel_hint: Option<Vec<String>>,
}

impl Envelope {
    #[must_use]
    pub fn new(
        from: impl Into<String>,
        to: impl Into<String>,
        kind: EnvelopeKind,
        topic: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            v: Some(1),
            ts: Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
            id: new_id("env"),
            from: from.into(),
            to: Some(to.into()),
            kind: kind.as_str().to_owned(),
            topic: topic.into(),
            body: body.into(),
            re: None,
            channel_hint: Some(vec![
                "file_jsonl".to_owned(),
                "http_jsonrpc".to_owned(),
            ]),
        }
    }
}

#[must_use]
pub fn new_id(prefix: &str) -> String {
    let mut buf = [0_u8; 4];
    if getrandom(&mut buf).is_err() {
        let nanos = Utc::now().timestamp_nanos_opt().unwrap_or(0) as u64;
        buf.copy_from_slice(&nanos.to_le_bytes()[..4]);
    }
    format!("{prefix}-{}", hex::encode(buf))
}

mod hex {
    pub(super) fn encode(bytes: [u8; 4]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
}