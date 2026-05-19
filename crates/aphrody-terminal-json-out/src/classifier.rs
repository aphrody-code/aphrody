// SPDX-License-Identifier: Apache-2.0
//! Line classifier — decide whether a raw stdout line is plain text or
//! already-structured JSON.
//!
//! # Rules
//!
//! - Leading whitespace is tolerated (`  {"x":1}` is still classified as JSON).
//! - The line must start with `{` or `[` to be eligible for JSON classification
//!   (cheap pre-filter — skips the serde parser on the >99% non-JSON case).
//! - Bare string literals (`"foo"`), bare numbers, `true` / `false` / `null`
//!   are kept as `Text` even though they're technically valid JSON, because
//!   embedding them as `Json` would silently swallow user-facing quotes /
//!   strip the textual representation. Containers only.
//! - Malformed JSON (`{not valid`) → `Text` (never `Err`).
//!
//! # Pattern reuse
//!
//! - Mirrors the typed-event classification convention in
//!   `aphrody_terminal_llm::osc::parse_aphrody_llm_osc` (which returns `None`
//!   on parse failure rather than an error — we mirror that "fall back to
//!   plain" stance for plain stdout lines).
//! - Reuses the legacy [`crate::frame_line`] heuristic exactly so both API
//!   surfaces agree on what counts as passthrough.

use serde_json::Value;

use crate::frame::FramePayload;

/// Classify one raw line into a typed [`FramePayload`].
///
/// Never returns an error — malformed JSON, empty input, and binary garbage
/// all fall through to `FramePayload::Text(line.to_string())`.
#[must_use]
pub fn classify_line(line: &str) -> FramePayload {
    if let Some(value) = try_parse_json_container(line) {
        FramePayload::Json(value)
    } else {
        FramePayload::Text(line.to_string())
    }
}

/// Classify a byte slice. UTF-8 bytes route through [`classify_line`];
/// non-UTF-8 bytes are wrapped as [`FramePayload::Bytes`].
#[must_use]
pub fn classify_bytes(bytes: &[u8]) -> FramePayload {
    match std::str::from_utf8(bytes) {
        Ok(s) => classify_line(s),
        Err(_) => FramePayload::Bytes(bytes.to_vec()),
    }
}

/// Cheap pre-filter + serde parse. Returns `Some(value)` iff:
/// * after trimming leading whitespace the line begins with `{` or `[`, AND
/// * `serde_json::from_str` accepts the *entire* line as a JSON document.
fn try_parse_json_container(line: &str) -> Option<Value> {
    let trimmed = line.trim_start();
    let first = trimmed.as_bytes().first().copied()?;
    if first != b'{' && first != b'[' {
        return None;
    }
    serde_json::from_str::<Value>(trimmed).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_is_text() {
        assert!(matches!(classify_line("hello world"), FramePayload::Text(s) if s == "hello world"));
    }

    #[test]
    fn json_object_is_passthrough() {
        let p = classify_line(r#"{"event":"build","ok":true}"#);
        match p {
            FramePayload::Json(v) => {
                assert_eq!(v["event"], "build");
                assert_eq!(v["ok"], true);
            },
            other => panic!("expected Json, got {other:?}"),
        }
    }

    #[test]
    fn json_array_is_passthrough() {
        let p = classify_line("[1,2,3]");
        match p {
            FramePayload::Json(v) => assert_eq!(v[1], 2),
            other => panic!("expected Json, got {other:?}"),
        }
    }

    #[test]
    fn malformed_json_falls_back_to_text() {
        let p = classify_line("{not valid json");
        assert!(matches!(p, FramePayload::Text(s) if s == "{not valid json"));
    }

    #[test]
    fn bare_string_literal_is_text() {
        // `"foo"` is technically valid JSON but we keep it as plain text so the
        // quotes survive a textual round-trip.
        let p = classify_line("\"quoted\"");
        assert!(matches!(p, FramePayload::Text(s) if s == "\"quoted\""));
    }

    #[test]
    fn leading_whitespace_tolerated() {
        let p = classify_line("   {\"a\":1}");
        assert!(matches!(p, FramePayload::Json(_)));
    }

    #[test]
    fn non_utf8_bytes_are_bytes_variant() {
        let p = classify_bytes(&[0xFF, 0xFE, 0x00, 0xAB]);
        assert!(matches!(p, FramePayload::Bytes(b) if b == vec![0xFF, 0xFE, 0x00, 0xAB]));
    }
}
