// SPDX-License-Identifier: Apache-2.0
//! Google Boq `batchexecute` request encoder + response parser.
//!
//! Wire format reference (captured from live `notebooklm.google.com` traffic):
//!
//! Request body (URL-encoded):
//! ```text
//! f.req=[[["<rpc_id>","<json_payload>",null,"generic"]]]&at=<at_token>
//! ```
//!
//! Response body (anti-XSSI prefix + length-prefixed JSON chunks):
//! ```text
//! )]}'
//! <byte_length>
//! [["wrb.fr","<rpc_id>","<inner_json_string>",null,null,null,"generic"]]
//! ```

use serde_json::Value;

use crate::error::{NotebookError, Result};

/// Anti-XSSI safety prefix every Boq response starts with.
pub const XSSI_PREFIX: &str = ")]}'\n";

/// Encode a single-RPC `f.req` body string.
pub fn encode_f_req(rpc_id: &str, payload: &Value) -> Result<String> {
    let inner = serde_json::to_string(payload)?;
    let outer = serde_json::to_string(&Value::Array(vec![Value::Array(vec![Value::Array(vec![
        Value::String(rpc_id.to_string()),
        Value::String(inner),
        Value::Null,
        Value::String("generic".to_string()),
    ])])]))?;
    Ok(outer)
}

/// Strip the leading `)]}'` safety prefix.
///
/// The prefix is sometimes followed by a stray newline; we eat at most one.
pub fn strip_xssi(raw: &str) -> &str {
    let stripped = raw.strip_prefix(")]}'\n").unwrap_or(raw);
    // Some endpoints emit `)]}'` with no trailing newline.
    stripped.strip_prefix(")]}'").unwrap_or(stripped).trim_start()
}

/// Walk the length-prefixed chunk stream and yield every JSON array chunk.
///
/// Chunks alternate between a `<length>\n` line and the JSON payload on the
/// following line (sometimes spanning multiple lines when the body itself
/// contains newlines). The parser tolerates both single-line and multi-line
/// payloads — matching the upstream `extractJsonChunks` reference impl.
pub fn extract_chunks(body: &str) -> Vec<Value> {
    let mut chunks = Vec::new();
    let lines: Vec<&str> = body.split('\n').collect();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();
        if line.is_empty() {
            i += 1;
            continue;
        }

        // Length marker: try single-line decode first.
        if line.chars().all(|c| c.is_ascii_digit()) {
            let length: usize = line.parse().unwrap_or(0);
            if i + 1 < lines.len() {
                let next = lines[i + 1].trim();
                if !next.is_empty() {
                    if let Ok(value) = serde_json::from_str::<Value>(next) {
                        chunks.push(value);
                        i += 2;
                        continue;
                    }
                }
            }
            // Multi-line fallback: accumulate up to the declared byte length.
            let mut accumulated = String::new();
            let mut j = i + 1;
            while j < lines.len() && accumulated.len() < length {
                if !accumulated.is_empty() {
                    accumulated.push('\n');
                }
                accumulated.push_str(lines[j]);
                j += 1;
            }
            if let Ok(value) = serde_json::from_str::<Value>(accumulated.trim()) {
                chunks.push(value);
            }
            i = j;
        } else if let Ok(value) = serde_json::from_str::<Value>(line) {
            // Length-less chunk (some streamed responses omit the byte count).
            chunks.push(value);
            i += 1;
        } else {
            i += 1;
        }
    }
    chunks
}

/// Iterate every `wrb.fr` envelope in a raw Boq response and return their
/// inner JSON payloads (one per RPC; usually exactly one for `batchexecute`).
pub fn parse_envelopes(raw: &str) -> Vec<Value> {
    let stripped = strip_xssi(raw);
    let chunks = extract_chunks(stripped);
    let mut results = Vec::new();
    for chunk in chunks {
        let Value::Array(envelopes) = chunk else { continue };
        for env in envelopes {
            let Value::Array(items) = env else { continue };
            if items.first().and_then(|v| v.as_str()) != Some("wrb.fr") {
                continue;
            }
            if let Some(Value::String(inner)) = items.get(2) {
                if let Ok(parsed) = serde_json::from_str::<Value>(inner) {
                    results.push(parsed);
                }
            }
        }
    }
    results
}

/// Convenience: return the first envelope or fail with [`NotebookError::Parse`].
pub fn first_envelope(raw: &str) -> Result<Value> {
    parse_envelopes(raw)
        .into_iter()
        .next()
        .ok_or_else(|| NotebookError::Parse("no wrb.fr envelope in response".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn encode_round_trip_single_string() {
        let payload = json!(["hello", 1, 2]);
        let encoded = encode_f_req("CCqFvf", &payload).unwrap();
        assert!(encoded.contains("CCqFvf"));
        assert!(encoded.contains("\\\"hello\\\""));
        assert!(encoded.contains("generic"));
    }

    #[test]
    fn strip_xssi_handles_both_variants() {
        assert_eq!(strip_xssi(")]}'\n[[1]]"), "[[1]]");
        assert_eq!(strip_xssi(")]}'[[1]]"), "[[1]]");
        assert_eq!(strip_xssi("[[1]]"), "[[1]]");
    }

    #[test]
    fn parse_envelopes_single_chunk() {
        let raw = format!(
            "{}{}\n{}",
            ")]}'\n",
            "82",
            r#"[["wrb.fr","CCqFvf","[\"nbid-1\",[\"thread-1\"]]",null,null,null,"generic"]]"#,
        );
        let envelopes = parse_envelopes(&raw);
        assert_eq!(envelopes.len(), 1);
        let inner = &envelopes[0];
        assert_eq!(inner[0].as_str(), Some("nbid-1"));
        assert_eq!(inner[1][0].as_str(), Some("thread-1"));
    }

    #[test]
    fn parse_envelopes_skips_other_kinds() {
        let raw = format!(
            "{}{}\n{}",
            ")]}'\n",
            "82",
            r#"[["di",123,"something"],["wrb.fr","wXbhsf","[[\"ok\"]]",null,null,null,"generic"]]"#,
        );
        let envelopes = parse_envelopes(&raw);
        assert_eq!(envelopes.len(), 1);
        assert_eq!(envelopes[0][0][0].as_str(), Some("ok"));
    }

    #[test]
    fn parse_envelopes_empty_when_xssi_only() {
        assert!(parse_envelopes(")]}'\n").is_empty());
        assert!(parse_envelopes("").is_empty());
    }

    #[test]
    fn extract_chunks_handles_multi_line_payload() {
        let raw = "\
)]}'\n\
42\n\
[[\"wrb.fr\",\"wXbhsf\",\"[\\n  \\\"a\\\"\\n]\",null,null,null,\"generic\"]]";
        let chunks = extract_chunks(strip_xssi(raw));
        assert_eq!(chunks.len(), 1);
    }
}
