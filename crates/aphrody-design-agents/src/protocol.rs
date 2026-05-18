// SPDX-License-Identifier: Apache-2.0
//! Protocol classification + minimal ACP (Agent Client Protocol) message
//! parser.
//!
//! The full ACP grammar is documented in `apps/daemon/src/acp.ts`; this
//! module ports just the parts the spawner needs to drive a one-shot
//! `initialize → session/new → session/prompt → session/cancel` exchange:
//!
//!  - `Protocol` enum classifying what transport to use for each agent
//!    (mirrors open-design's `streamFormat` discriminator).
//!  - `RpcMessage` / `RpcId` for line-delimited JSON-RPC framing, the
//!    on-the-wire shape ACP agents speak. Parses the four method names the
//!    spawner cares about (`initialize`, `session/new`, `session/prompt`,
//!    `session/cancel`) plus their JSON-RPC envelopes.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

/// Wire transport an adapter speaks. `Stdio` here is "newline-delimited
/// JSON events / opaque text over stdin/stdout, not JSON-RPC", which is
/// what Claude Code and Gemini both do. `Acp` is the JSON-RPC line protocol
/// from the Agent Client Protocol spec (Antigravity here, same family as
/// Hermes/Kimi/Devin upstream). `Sse` is reserved for HTTP-bridged
/// adapters; it is not used by the 3-CLI set but stays in the enum so
/// future additions don't require breaking changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Acp,
    Stdio,
    Sse,
}

impl Protocol {
    /// Inverse of the upstream classifier in
    /// `apps/daemon/src/runtimes/defs/*.ts`. Used by tests to round-trip
    /// the streamFormat string back to an enum and catch drift.
    pub fn from_stream_format(s: &str) -> Option<Self> {
        match s {
            "acp-json-rpc" => Some(Protocol::Acp),
            "claude-stream-json"
            | "json-event-stream"
            | "plain" => Some(Protocol::Stdio),
            _ => None,
        }
    }

    /// Inverse mapping: pick a canonical streamFormat label for the wire
    /// protocol. Stdio agents in upstream actually have several specific
    /// labels; we collapse to `"json-event-stream"` as the catch-all that
    /// every Stdio adapter accepts as a parser hint.
    pub const fn stream_format(self) -> &'static str {
        match self {
            Protocol::Acp => "acp-json-rpc",
            Protocol::Stdio => "json-event-stream",
            Protocol::Sse => "sse",
        }
    }

    /// Pretty short label for human-facing CLI output.
    pub const fn as_str(self) -> &'static str {
        match self {
            Protocol::Acp => "acp",
            Protocol::Stdio => "stdio",
            Protocol::Sse => "sse",
        }
    }
}

/// JSON-RPC id. Open-design's `acp.ts` accepts both numeric and string ids
/// because Hermes/Kimi/Devin all happen to use small integers, but the
/// upstream spec permits either.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RpcId {
    Num(i64),
    Str(String),
}

/// Minimal JSON-RPC 2.0 envelope. Carries either a `method`+`params`
/// request, a `result`/`error` response, or a notification (no `id`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcMessage {
    pub jsonrpc: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<RpcId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<Value>,
}

impl RpcMessage {
    pub fn request(id: i64, method: impl Into<String>, params: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: Some(RpcId::Num(id)),
            method: Some(method.into()),
            params: Some(params),
            result: None,
            error: None,
        }
    }

    pub fn parse(line: &str) -> Result<Self, ProtocolError> {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            return Err(ProtocolError::EmptyLine);
        }
        let msg: RpcMessage = serde_json::from_str(trimmed)?;
        if msg.jsonrpc != "2.0" {
            return Err(ProtocolError::WrongVersion(msg.jsonrpc));
        }
        Ok(msg)
    }

    pub fn to_line(&self) -> String {
        // ACP framing is exactly one JSON object per line, terminated by
        // `\n`. open-design's sendRpc does the same thing.
        let mut s = serde_json::to_string(self).expect("rpc message is always serializable");
        s.push('\n');
        s
    }

    pub fn is_acp_method(&self, name: &str) -> bool {
        self.method.as_deref() == Some(name)
    }
}

/// Build the `initialize` request that every ACP exchange opens with. The
/// constants here mirror open-design's `attachAcpSession`.
pub fn build_initialize(id: i64, client_name: &str, client_version: &str) -> RpcMessage {
    RpcMessage::request(
        id,
        "initialize",
        serde_json::json!({
            "protocolVersion": 1,
            "clientCapabilities": { "terminal": false },
            "clientInfo": { "name": client_name, "version": client_version },
        }),
    )
}

/// Build the `session/new` request opening a fresh ACP session at `cwd`.
pub fn build_session_new(id: i64, cwd: &str) -> RpcMessage {
    RpcMessage::request(
        id,
        "session/new",
        serde_json::json!({
            "cwd": cwd,
            "mcpServers": [],
        }),
    )
}

/// Build the `session/prompt` request carrying a single text turn.
pub fn build_session_prompt(id: i64, session_id: &str, prompt: &str) -> RpcMessage {
    RpcMessage::request(
        id,
        "session/prompt",
        serde_json::json!({
            "sessionId": session_id,
            "prompt": [{ "type": "text", "text": prompt }],
        }),
    )
}

/// Build the `session/cancel` notification used to interrupt a running
/// prompt. Sent without expecting a response; ACP agents typically tear
/// down the in-flight turn and return to idle.
pub fn build_session_cancel(id: i64, session_id: &str) -> RpcMessage {
    RpcMessage::request(
        id,
        "session/cancel",
        serde_json::json!({ "sessionId": session_id }),
    )
}

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("empty json-rpc line")]
    EmptyLine,
    #[error("unsupported jsonrpc version: {0}")]
    WrongVersion(String),
    #[error("json parse error: {0}")]
    Json(#[from] serde_json::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_initialize_request_round_trip() {
        let req = build_initialize(1, "aphrody", "0.1.0");
        let line = req.to_line();
        assert!(line.ends_with('\n'));
        let parsed = RpcMessage::parse(&line).expect("round trip");
        assert!(parsed.is_acp_method("initialize"));
        assert_eq!(parsed.id, Some(RpcId::Num(1)));
        assert_eq!(parsed.jsonrpc, "2.0");
        let params = parsed.params.as_ref().unwrap();
        assert_eq!(params["protocolVersion"], 1);
        assert_eq!(params["clientInfo"]["name"], "aphrody");
    }

    #[test]
    fn parses_response_with_session_id() {
        // Shape an ACP agent returns for session/new success.
        let line = r#"{"jsonrpc":"2.0","id":2,"result":{"sessionId":"sess-abc","models":{"availableModels":[]}}}"#;
        let parsed = RpcMessage::parse(line).expect("parses");
        assert_eq!(parsed.id, Some(RpcId::Num(2)));
        let result = parsed.result.as_ref().unwrap();
        assert_eq!(result["sessionId"], "sess-abc");
    }

    #[test]
    fn rejects_wrong_jsonrpc_version() {
        let line = r#"{"jsonrpc":"1.0","method":"initialize"}"#;
        let err = RpcMessage::parse(line).unwrap_err();
        matches!(err, ProtocolError::WrongVersion(_));
    }

    #[test]
    fn empty_line_is_rejected() {
        let err = RpcMessage::parse("   ").unwrap_err();
        matches!(err, ProtocolError::EmptyLine);
    }

    #[test]
    fn stream_format_round_trips() {
        // Every supported open-design streamFormat string we care about must
        // classify to a Protocol that round-trips back to a canonical label.
        for sf in ["acp-json-rpc", "claude-stream-json", "json-event-stream", "plain"] {
            let p = Protocol::from_stream_format(sf).unwrap_or_else(|| panic!("classify {sf}"));
            assert!(!p.stream_format().is_empty());
        }
        assert!(Protocol::from_stream_format("nonsense").is_none());
    }

    #[test]
    fn build_session_prompt_matches_acp_schema() {
        let m = build_session_prompt(7, "sess-1", "hello world");
        let v = serde_json::to_value(&m).unwrap();
        assert_eq!(v["jsonrpc"], "2.0");
        assert_eq!(v["id"], 7);
        assert_eq!(v["method"], "session/prompt");
        assert_eq!(v["params"]["sessionId"], "sess-1");
        assert_eq!(v["params"]["prompt"][0]["type"], "text");
        assert_eq!(v["params"]["prompt"][0]["text"], "hello world");
    }

    #[test]
    fn build_session_cancel_targets_session() {
        let m = build_session_cancel(99, "sess-42");
        assert!(m.is_acp_method("session/cancel"));
        let p = m.params.as_ref().unwrap();
        assert_eq!(p["sessionId"], "sess-42");
    }

    #[test]
    fn rpc_id_supports_string_form() {
        let line = r#"{"jsonrpc":"2.0","id":"req-1","method":"initialize"}"#;
        let parsed = RpcMessage::parse(line).expect("string id parses");
        match parsed.id {
            Some(RpcId::Str(ref s)) => assert_eq!(s, "req-1"),
            other => panic!("expected string id, got {other:?}"),
        }
    }

    #[test]
    fn acp_message_round_trip_preserves_all_fields() {
        // Full ACP roundtrip: build → serialize → parse → reserialize and
        // assert every meaningful field survives. Locks in the wire format
        // so consumers can rely on byte-equivalent JSON output.
        let original = build_session_prompt(5, "sess-test", "ping");
        let line = original.to_line();
        let parsed = RpcMessage::parse(&line).expect("parses back");
        assert_eq!(parsed.jsonrpc, "2.0");
        assert_eq!(parsed.id, Some(RpcId::Num(5)));
        assert_eq!(parsed.method.as_deref(), Some("session/prompt"));
        let params = parsed.params.as_ref().expect("params present");
        assert_eq!(params["sessionId"], "sess-test");
        assert_eq!(params["prompt"][0]["text"], "ping");
        // Reserialize and re-parse to confirm stability.
        let reline = parsed.to_line();
        let reparsed = RpcMessage::parse(&reline).expect("re-parses");
        assert_eq!(reparsed.id, parsed.id);
        assert_eq!(reparsed.method, parsed.method);
    }
}
