// SPDX-License-Identifier: Apache-2.0
//! `StreamGenerate` send-payload builder + streamed-response parser.
//!
//! Wire spec captured live from `gemini.google.com` build
//! `boq_assistant-bard-web-server_20260520.03_p0` (2026-05-21). A user message
//! is NOT sent via `batchexecute`; it goes to
//! [`crate::rpc_ids::URL_STREAM_GENERATE`] (the `BardFrontendService` streaming
//! endpoint) with body `f.req=[null,"<inner_list_json>"]&at=<token>`.
//!
//! The `inner_list` (the JSON-stringified slot 1 of the `f.req` envelope) is:
//! ```text
//! [ [prompt, 0, null, null, null, null, 0],   // [0] message_content
//!   [language],                                 // [1] language_tuple
//!   [cid, rid, rcid] ]                          // [2] chat_metadata (null on first turn)
//! ```
//! The live web UI appends further sparse context slots (indices 3+); they are
//! optional for a text-only send and omitted here.

use serde_json::{json, Value};

use crate::boq::parse_envelopes;
use crate::error::{GeminiError, Result};
use crate::types::{ChatReply, ConversationMetadata};

/// Build the `StreamGenerate` `inner_list` for a prompt.
///
/// `language` is the `hl` locale (e.g. `"fr"`). `meta` threads a prior turn;
/// pass [`ConversationMetadata::default`] for a fresh conversation.
#[must_use]
pub fn build_send_payload(prompt: &str, language: &str, meta: &ConversationMetadata) -> Value {
    let message_content = json!([prompt, 0, Value::Null, Value::Null, Value::Null, Value::Null, 0]);
    let language_tuple = json!([language]);
    let chat_metadata = json!([
        opt_str(meta.conversation_id.as_deref()),
        opt_str(meta.response_id.as_deref()),
        opt_str(meta.choice_id.as_deref()),
    ]);
    json!([message_content, language_tuple, chat_metadata])
}

fn opt_str(v: Option<&str>) -> Value {
    v.map_or(Value::Null, |s| Value::String(s.to_string()))
}

/// Parse the raw streamed `StreamGenerate` response into a [`ChatReply`].
///
/// The body is the Boq `)]}'` + length-prefixed chunk stream; each chunk holds
/// `wrb.fr` envelopes whose inner JSON carries (progressively) the reply. We
/// take the richest envelope (longest reply text). Layout: `inner[1]` =
/// `[cid, rid]`, `inner[4]` = candidate list, candidate = `[rcid, [text, …], …]`.
///
/// # Errors
///
/// [`GeminiError::Parse`] when no envelope yields reply text.
pub fn parse_stream_response(raw: &str) -> Result<ChatReply> {
    let mut best: Option<ChatReply> = None;
    for inner in parse_envelopes(raw) {
        if let Some(reply) = extract_reply(&inner) {
            let better = best.as_ref().is_none_or(|b| reply.text.len() > b.text.len());
            if better {
                best = Some(reply);
            }
        }
    }
    best.ok_or_else(|| GeminiError::Parse("StreamGenerate: no reply candidate in stream".into()))
}

/// Extract a [`ChatReply`] from one decoded `wrb.fr` inner payload, if it holds
/// a candidate with text.
fn extract_reply(inner: &Value) -> Option<ChatReply> {
    let candidates = inner.get(4).and_then(Value::as_array)?;
    let candidate_count = candidates.len();
    let first = candidates.first()?;
    let text = first
        .get(1)
        .and_then(Value::as_array)
        .and_then(|a| a.first())
        .and_then(Value::as_str)?
        .to_string();

    let meta_arr = inner.get(1).and_then(Value::as_array);
    let conversation_id = meta_arr
        .and_then(|m| m.first())
        .and_then(Value::as_str)
        .map(str::to_string);
    let response_id = meta_arr
        .and_then(|m| m.get(1))
        .and_then(Value::as_str)
        .map(str::to_string);
    let choice_id = first.get(0).and_then(Value::as_str).map(str::to_string);

    let web_image_urls = first
        .get(12)
        .and_then(|v| v.get(1))
        .and_then(Value::as_array)
        .map(|imgs| {
            imgs.iter()
                .filter_map(|img| img.get(0).and_then(|u| u.get(0)).and_then(Value::as_str))
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();

    Some(ChatReply {
        text,
        metadata: ConversationMetadata { conversation_id, response_id, choice_id },
        web_image_urls,
        candidate_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_payload_has_prompt_lang_and_null_metadata() {
        let p = build_send_payload("hello", "fr", &ConversationMetadata::default());
        assert_eq!(p[0][0], "hello");
        assert_eq!(p[1], json!(["fr"]));
        assert_eq!(p[2], json!([null, null, null]));
    }

    #[test]
    fn threaded_payload_carries_ids() {
        let meta = ConversationMetadata {
            conversation_id: Some("c_1".into()),
            response_id: Some("r_1".into()),
            choice_id: Some("rc_1".into()),
        };
        let p = build_send_payload("again", "en", &meta);
        assert_eq!(p[2], json!(["c_1", "r_1", "rc_1"]));
    }

    #[test]
    fn parse_stream_extracts_text_and_metadata() {
        // One wrb.fr chunk; inner[1]=[cid,rid], inner[4]=[[rcid,[text]]].
        let raw = format!(
            "{}{}\n{}",
            ")]}'\n",
            "120",
            r#"[["wrb.fr","abc","[null,[\"c_42\",\"r_99\"],null,null,[[\"rc_7\",[\"the reply\"]]]]",null,null,null,"generic"]]"#,
        );
        let reply = parse_stream_response(&raw).unwrap();
        assert_eq!(reply.text, "the reply");
        assert_eq!(reply.metadata.conversation_id.as_deref(), Some("c_42"));
        assert_eq!(reply.metadata.response_id.as_deref(), Some("r_99"));
        assert_eq!(reply.metadata.choice_id.as_deref(), Some("rc_7"));
        assert_eq!(reply.candidate_count, 1);
    }

    #[test]
    fn parse_stream_picks_longest_text() {
        // Two progressive envelopes; the second is richer.
        let raw = format!(
            ")]}}'\n40\n{}\n80\n{}",
            r#"[["wrb.fr","a","[null,[\"c\",\"r\"],null,null,[[\"rc\",[\"par\"]]]]",null,null,null,"generic"]]"#,
            r#"[["wrb.fr","a","[null,[\"c\",\"r\"],null,null,[[\"rc\",[\"partial then full\"]]]]",null,null,null,"generic"]]"#,
        );
        let reply = parse_stream_response(&raw).unwrap();
        assert_eq!(reply.text, "partial then full");
    }

    #[test]
    fn parse_stream_errors_without_candidates() {
        let raw = format!("{}{}\n{}", ")]}'\n", "40", r#"[["wrb.fr","a","[null,null]",null,null,null,"generic"]]"#);
        assert!(parse_stream_response(&raw).is_err());
    }
}
