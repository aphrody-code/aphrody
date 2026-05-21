// SPDX-License-Identifier: Apache-2.0
//! `MaZiqc` send-message payload builder + response parser.
//!
//! Wire spec (HIGH confidence; outer framing observed live, inner layout
//! cross-referenced with HanaokaYuzu/Gemini-API master — see
//! `var/data/gemini-web-recon/gemini-send-payload-spec.json` and
//! `docs/research/gemini-web-cdp-exploitation.md`):
//!
//! The value handed to [`crate::boq::encode_f_req`] for `MaZiqc` is:
//! ```text
//! [20, "<inner_json_string>", [0, null, 1]]
//! ```
//! where `inner_json_string` is the JSON-stringified message array:
//! ```text
//! [ [prompt, 0, null, file_data, null, null, 0],   // message_content
//!   [language],                                     // language_tuple
//!   [cid, rid, rcid] ]                              // chat_metadata
//! ```
//! `20` is the generate-turn discriminator; `[0, null, 1]` is the trailing
//! turn-config. On the first turn `cid`/`rid`/`rcid` are `null`.

use serde_json::{json, Value};

use crate::error::{GeminiError, Result};
use crate::types::{ChatReply, ConversationMetadata};

/// Build the `MaZiqc` `f.req` inner payload `[20, <json_string>, [0, null, 1]]`.
///
/// `language` is the `hl` locale (e.g. `"fr"`). `meta` threads a prior turn's
/// ids; pass [`ConversationMetadata::default`] for a fresh conversation.
#[must_use]
pub fn build_send_payload(prompt: &str, language: &str, meta: &ConversationMetadata) -> Value {
    let message_content = json!([prompt, 0, Value::Null, Value::Null, Value::Null, Value::Null, 0]);
    let language_tuple = json!([language]);
    let chat_metadata = json!([
        opt_str(meta.conversation_id.as_deref()),
        opt_str(meta.response_id.as_deref()),
        opt_str(meta.choice_id.as_deref()),
    ]);
    let inner_req_list = json!([message_content, language_tuple, chat_metadata]);
    // The inner array is itself JSON-stringified into slot 1 of the [20, …]
    // wrapper (double-encoding — matches the live capture).
    let inner_json_string = inner_req_list.to_string();
    json!([20, inner_json_string, [0, Value::Null, 1]])
}

fn opt_str(v: Option<&str>) -> Value {
    v.map_or(Value::Null, |s| Value::String(s.to_string()))
}

/// Parse a `MaZiqc` response envelope (the inner `wrb.fr` payload) into a
/// [`ChatReply`].
///
/// Layout (`HanaokaYuzu` master): `inner[1]` = `[cid, rid]` metadata, `inner[4]`
/// = candidate list, each candidate = `[rcid, [text, …], …]` with the reply
/// text at `candidate[1][0]` and web images at `candidate[12][1]`.
///
/// # Errors
///
/// Returns [`GeminiError::Parse`] if no candidate with reply text is found.
pub fn parse_send_response(inner: &Value) -> Result<ChatReply> {
    let candidates = inner
        .get(4)
        .and_then(Value::as_array)
        .ok_or_else(|| GeminiError::Parse("MaZiqc response: no candidate list at inner[4]".into()))?;

    let candidate_count = candidates.len();
    let first = candidates
        .first()
        .ok_or_else(|| GeminiError::Parse("MaZiqc response: empty candidate list".into()))?;

    let text = first
        .get(1)
        .and_then(Value::as_array)
        .and_then(|a| a.first())
        .and_then(Value::as_str)
        .ok_or_else(|| GeminiError::Parse("MaZiqc response: no text at candidate[1][0]".into()))?
        .to_string();

    // Metadata: cid/rid from inner[1]; rcid from the selected candidate[0].
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

    Ok(ChatReply {
        text,
        metadata: ConversationMetadata { conversation_id, response_id, choice_id },
        web_image_urls,
        candidate_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::boq::encode_f_req;

    #[test]
    fn fresh_payload_has_null_metadata_and_prompt() {
        let p = build_send_payload("hello", "fr", &ConversationMetadata::default());
        assert_eq!(p[0], 20);
        assert_eq!(p[2], json!([0, null, 1]));
        // inner[1] is a JSON string; decode it and check the prompt + nulls.
        let inner: Value = serde_json::from_str(p[1].as_str().unwrap()).unwrap();
        assert_eq!(inner[0][0], "hello");
        assert_eq!(inner[1], json!(["fr"]));
        assert_eq!(inner[2], json!([null, null, null]));
    }

    #[test]
    fn threaded_payload_carries_ids() {
        let meta = ConversationMetadata {
            conversation_id: Some("c_1".into()),
            response_id: Some("r_1".into()),
            choice_id: Some("rc_1".into()),
        };
        let p = build_send_payload("again", "en", &meta);
        let inner: Value = serde_json::from_str(p[1].as_str().unwrap()).unwrap();
        assert_eq!(inner[2], json!(["c_1", "r_1", "rc_1"]));
    }

    #[test]
    fn payload_encodes_into_batchexecute_envelope() {
        let p = build_send_payload("hi", "fr", &ConversationMetadata::default());
        let encoded = encode_f_req("MaZiqc", &p).unwrap();
        assert!(encoded.contains("MaZiqc"));
        assert!(encoded.contains("generic"));
    }

    #[test]
    fn parse_extracts_text_and_metadata() {
        // inner[1] = [cid, rid]; inner[4] = [[rcid, [text]]]
        let inner = json!([
            null,
            ["c_42", "r_99"],
            null,
            null,
            [["rc_7", ["the reply text"]]]
        ]);
        let reply = parse_send_response(&inner).unwrap();
        assert_eq!(reply.text, "the reply text");
        assert_eq!(reply.metadata.conversation_id.as_deref(), Some("c_42"));
        assert_eq!(reply.metadata.response_id.as_deref(), Some("r_99"));
        assert_eq!(reply.metadata.choice_id.as_deref(), Some("rc_7"));
        assert_eq!(reply.candidate_count, 1);
    }

    #[test]
    fn parse_fails_without_candidates() {
        let inner = json!([null, ["c", "r"], null, null, null]);
        assert!(parse_send_response(&inner).is_err());
    }
}
