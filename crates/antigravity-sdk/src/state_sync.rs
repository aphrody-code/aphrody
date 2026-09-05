// SPDX-License-Identifier: Apache-2.0
//! Tolerant decoder for Antigravity `state.vscdb` **UnifiedStateSync** blobs.
//!
//! The Antigravity VSCode-fork persists application state in a SQLite database
//! (`state.vscdb`) whose `ItemTable` maps string keys to string values.  Keys
//! in the `antigravityUnifiedStateSync.*` namespace hold opaque payloads that
//! are usually `base64(protobuf)` with an **embedded UTF-8 JSON document** (the
//! sign-in / context state) and, for token-bearing keys, a **base32 token-info**
//! segment.  A typical decoded JSON looks like:
//!
//! ```json
//! { "state": "signedIn", "context": { "tier": "ultra" } }
//! ```
//!
//! This module performs *structural* decoding only.  It is deliberately
//! best-effort and **never panics**: malformed, truncated, or non-base64 input
//! yields partial information rather than an error wherever possible.
//!
//! # Security
//!
//! This decoder reports **structure**, not secrets.  It must never be wired to
//! emit raw token bytes.  [`decode_unified_state`] surfaces only lengths and a
//! short, character-class hint for any base32 token-info it locates — never the
//! decoded token material.  [`parse_item_table`] emits only key names, value
//! lengths, and a boolean sign-in flag.
//!
//! # Examples
//!
//! ```rust
//! use antigravity_sdk::state_sync::decode_unified_state;
//! use base64::Engine as _;
//!
//! // A synthetic blob: base64 of a small JSON document.
//! let raw = br#"prefix{"state":"signedIn"}suffix"#;
//! let b64 = base64::engine::general_purpose::STANDARD.encode(raw);
//!
//! let summary = decode_unified_state(&b64).unwrap();
//! assert_eq!(summary["embedded_json"]["state"], "signedIn");
//! assert!(summary["raw_len"].as_u64().unwrap() > 0);
//! ```

use base64::Engine as _;
use serde::Serialize;
use thiserror::Error;

/// Errors raised by [`decode_unified_state`].
///
/// The decoder is tolerant by design; an error is returned only when the input
/// cannot be turned into *any* bytes (i.e. it is neither valid standard nor
/// URL-safe base64).  Once bytes are recovered, decoding always succeeds with a
/// partial summary.
#[derive(Debug, Error)]
pub enum StateSyncError {
    /// The input string could not be decoded as standard *or* URL-safe base64.
    #[error("value is not valid base64 (standard or url-safe): {0}")]
    Base64(String),

    /// The input was empty after trimming surrounding whitespace.
    #[error("value is empty")]
    Empty,
}

/// The `antigravityUnifiedStateSync.` key prefix used by the VSCode fork.
pub const UNIFIED_STATE_SYNC_PREFIX: &str = "antigravityUnifiedStateSync.";

/// A non-sensitive structural summary of a single `ItemTable` entry.
///
/// Produced by [`parse_item_table`].  Carries no token material — only the key
/// name, the value length, and structural flags.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct StateSyncEntry {
    /// The raw `ItemTable` key.
    pub key: String,
    /// Length of the (string) value in bytes, or `0` for non-string values.
    pub value_len: usize,
    /// Whether the key belongs to the `antigravityUnifiedStateSync.*` namespace.
    pub is_unified_state_sync: bool,
    /// Best-effort flag: the value appears to encode a sign-in / auth state.
    pub looks_like_signin_state: bool,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Tolerantly decode a single UnifiedStateSync value.
///
/// Steps:
/// 1. base64-decode the value (standard alphabet first, URL-safe as fallback,
///    both with and without padding).
/// 2. scan the recovered bytes for the first balanced, valid UTF-8 JSON object
///    (`{ … }`) and parse it.
/// 3. scan for a plausible base32 token-info run and report a *hint* (decoded
///    byte length + a coarse character-class summary) — never the bytes.
///
/// Returns a [`serde_json::Value`] object of the shape:
///
/// ```json
/// {
///   "raw_len": 123,
///   "embedded_json": { "state": "signedIn", "...": "..." },   // optional
///   "base32_decoded_hint": { "encoded_len": 16, "decoded_len": 10, "kind": "binary" } // optional
/// }
/// ```
///
/// # Errors
///
/// Returns [`StateSyncError::Empty`] if the input is blank, or
/// [`StateSyncError::Base64`] if it decodes under neither base64 alphabet.
pub fn decode_unified_state(b64_value: &str) -> Result<serde_json::Value, StateSyncError> {
    let trimmed = b64_value.trim();
    if trimmed.is_empty() {
        return Err(StateSyncError::Empty);
    }

    let bytes = decode_base64_tolerant(trimmed)
        .ok_or_else(|| StateSyncError::Base64(short_preview(trimmed)))?;

    let mut summary = serde_json::Map::new();
    summary.insert("raw_len".to_owned(), serde_json::json!(bytes.len()));

    if let Some(json) = extract_embedded_json(&bytes) {
        summary.insert("embedded_json".to_owned(), json);
    }

    if let Some(hint) = extract_base32_hint(&bytes) {
        summary.insert("base32_decoded_hint".to_owned(), hint);
    }

    Ok(serde_json::Value::Object(summary))
}

/// Summarize a parsed `ItemTable` dump.
///
/// `json` is expected to be a JSON object mapping `ItemTable` keys to their
/// values (typically strings).  Returns one [`StateSyncEntry`] per key with
/// structural facts only.  Non-object inputs yield an empty vector.
///
/// Entries are returned sorted by key for deterministic output.
#[must_use]
pub fn parse_item_table(json: &serde_json::Value) -> Vec<StateSyncEntry> {
    let Some(map) = json.as_object() else {
        return Vec::new();
    };

    let mut entries: Vec<StateSyncEntry> = map
        .iter()
        .map(|(key, value)| {
            let value_str = value.as_str();
            let value_len = value_str.map_or(0, str::len);
            let is_unified_state_sync = key.starts_with(UNIFIED_STATE_SYNC_PREFIX);
            let looks_like_signin_state =
                value_str.is_some_and(value_looks_like_signin) || key_looks_like_signin(key);

            StateSyncEntry {
                key: key.clone(),
                value_len,
                is_unified_state_sync,
                looks_like_signin_state,
            }
        })
        .collect();

    entries.sort_by(|a, b| a.key.cmp(&b.key));
    entries
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Decode base64 trying multiple alphabets/padding modes; returns `None` only
/// if every variant fails.
fn decode_base64_tolerant(s: &str) -> Option<Vec<u8>> {
    use base64::engine::general_purpose::{
        STANDARD, STANDARD_NO_PAD, URL_SAFE, URL_SAFE_NO_PAD,
    };

    // Strip ASCII whitespace that frequently sneaks into stored blobs.
    let compact: String = s.chars().filter(|c| !c.is_ascii_whitespace()).collect();

    for engine in [&STANDARD, &STANDARD_NO_PAD, &URL_SAFE, &URL_SAFE_NO_PAD] {
        if let Ok(bytes) = engine.decode(compact.as_bytes()) {
            return Some(bytes);
        }
    }
    None
}

/// Scan a byte slice for the first balanced, valid-UTF-8 JSON object and parse
/// it. Returns `None` if no parseable `{ … }` region is found.
fn extract_embedded_json(bytes: &[u8]) -> Option<serde_json::Value> {
    // Find each candidate '{' and attempt a balanced scan from there. Protobuf
    // wire bytes routinely contain stray '{' (0x7B) so we must validate each
    // candidate by actually parsing it, not just by brace balance.
    let mut start = 0usize;
    while let Some(rel) = bytes[start..].iter().position(|&b| b == b'{') {
        let open = start + rel;
        if let Some(end) = balanced_object_end(bytes, open) {
            // `end` is the index just past the closing '}'.
            if let Ok(text) = std::str::from_utf8(&bytes[open..end]) {
                if let Ok(value) = serde_json::from_str::<serde_json::Value>(text) {
                    if value.is_object() {
                        return Some(value);
                    }
                }
            }
        }
        start = open + 1;
    }
    None
}

/// Given the index of an opening `{`, return the index just past the matching
/// `}`, accounting for nested objects and JSON string literals (so braces
/// inside strings do not affect balance). Returns `None` if unbalanced.
fn balanced_object_end(bytes: &[u8], open: usize) -> Option<usize> {
    debug_assert_eq!(bytes[open], b'{');
    let mut depth: i32 = 0;
    let mut in_string = false;
    let mut escaped = false;

    for (i, &b) in bytes.iter().enumerate().skip(open) {
        if in_string {
            if escaped {
                escaped = false;
            } else if b == b'\\' {
                escaped = true;
            } else if b == b'"' {
                in_string = false;
            }
            continue;
        }

        match b {
            b'"' => in_string = true,
            b'{' => depth += 1,
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i + 1);
                }
            }
            // A control byte (other than JSON whitespace) outside a string means
            // this is almost certainly protobuf noise, not JSON. Bail early to
            // avoid swallowing the rest of the buffer.
            0x00..=0x08 | 0x0B | 0x0C | 0x0E..=0x1F => return None,
            _ => {}
        }
    }
    None
}

/// Locate a plausible base32 token-info run in the bytes and, if it decodes,
/// return a non-sensitive hint describing it. Never returns the decoded bytes.
///
/// Antigravity stores token-info as RFC 4648 base32 (uppercase `A–Z2–7`,
/// optional `=` padding). We look for the longest contiguous run of base32
/// characters and try to decode it.
fn extract_base32_hint(bytes: &[u8]) -> Option<serde_json::Value> {
    let text = String::from_utf8_lossy(bytes);
    let candidate = longest_base32_run(&text)?;

    // RFC 4648 base32 requires the encoded length to be a multiple of 8 once
    // padded; data-encoding's BASE32 expects canonical padding. Try padded and
    // unpadded variants.
    let decoded = decode_base32_tolerant(candidate)?;

    let kind = classify_bytes(&decoded);
    Some(serde_json::json!({
        "encoded_len": candidate.len(),
        "decoded_len": decoded.len(),
        "kind": kind,
    }))
}

/// Find the longest contiguous run of RFC 4648 base32 characters (`A–Z`, `2–7`)
/// of length at least 8. Trailing `=` padding is included.
fn longest_base32_run(text: &str) -> Option<&str> {
    let bytes = text.as_bytes();
    let is_b32 = |b: u8| b.is_ascii_uppercase() && b != b'0' && b != b'1' || b.is_ascii_digit() && (b'2'..=b'7').contains(&b);
    let is_b32_or_pad = |b: u8| is_b32(b) || b == b'=';

    let mut best: Option<(usize, usize)> = None;
    let mut i = 0;
    while i < bytes.len() {
        if is_b32(bytes[i]) {
            let start = i;
            // Consume base32 chars, then any trailing padding.
            while i < bytes.len() && is_b32(bytes[i]) {
                i += 1;
            }
            while i < bytes.len() && bytes[i] == b'=' {
                i += 1;
            }
            // Guard: do not let stray non-padding bytes extend the run.
            let _ = is_b32_or_pad;
            let len = i - start;
            if best.is_none_or(|(_, bl)| len > bl) {
                best = Some((start, len));
            }
        } else {
            i += 1;
        }
    }

    best.and_then(|(start, len)| {
        if len >= 8 {
            text.get(start..start + len)
        } else {
            None
        }
    })
}

/// Decode a base32 candidate, tolerating missing padding by re-padding to the
/// next multiple of 8. Returns `None` if it still fails to decode.
fn decode_base32_tolerant(candidate: &str) -> Option<Vec<u8>> {
    use data_encoding::BASE32;

    let trimmed = candidate.trim_end_matches('=');
    let mut padded = trimmed.to_owned();
    let rem = padded.len() % 8;
    if rem != 0 {
        padded.push_str(&"=".repeat(8 - rem));
    }
    BASE32.decode(padded.as_bytes()).ok()
}

/// Coarse, non-sensitive classification of decoded bytes: `"text"` if all bytes
/// are printable/whitespace ASCII, otherwise `"binary"`.
fn classify_bytes(bytes: &[u8]) -> &'static str {
    if bytes
        .iter()
        .all(|&b| b == b'\t' || b == b'\n' || b == b'\r' || (0x20..=0x7E).contains(&b))
    {
        "text"
    } else {
        "binary"
    }
}

/// Heuristic: does this string value look like a sign-in / auth state document?
fn value_looks_like_signin(value: &str) -> bool {
    // Cheap substring probes that work whether the value is plain JSON or a
    // base64 wrapper whose decoded form embeds these tokens.
    const NEEDLES: [&str; 5] = ["signedIn", "signedOut", "\"state\"", "oauthToken", "authState"];
    NEEDLES.iter().any(|n| value.contains(n))
}

/// Heuristic: does this key look auth/sign-in related?
fn key_looks_like_signin(key: &str) -> bool {
    let lower = key.to_ascii_lowercase();
    lower.contains("oauth") || lower.contains("token") || lower.contains("signin") || lower.contains("auth")
}

/// A short, length-bounded preview of an input for diagnostics. Truncates so we
/// never echo a large (potentially sensitive) blob into an error message.
fn short_preview(s: &str) -> String {
    const MAX: usize = 16;
    let mut out: String = s.chars().take(MAX).collect();
    if s.chars().count() > MAX {
        out.push('…');
    }
    out
}

// ---------------------------------------------------------------------------
// state.vscdb reader (non-wasm only — requires rusqlite + bundled libsqlite3)
// ---------------------------------------------------------------------------

/// Read all rows from the `ItemTable` of a `state.vscdb` database.
///
/// Opens the database strictly read-only (`SQLITE_OPEN_READ_ONLY`) and reads
/// only the `key` + `value` columns.  Values are returned as opaque strings;
/// callers must pass them to [`parse_item_table`] / [`decode_unified_state`]
/// which enforce the security invariant (no raw token material emitted).
///
/// Returns an empty map when `ItemTable` does not exist (new or uninitialized
/// database).
///
/// # Errors
///
/// Propagates `rusqlite::Error` on open failure or query failure.  A missing
/// `ItemTable` is not an error — the map is simply empty.
///
/// # Security
///
/// This function does **not** filter values.  The caller is responsible for
/// passing the returned map only to [`parse_item_table`] (structural summary)
/// or [`decode_unified_state`] (structural decode).  Never log or print raw
/// values from this map.
#[cfg(not(target_family = "wasm"))]
pub fn read_state_db(
    db_path: &std::path::Path,
) -> Result<serde_json::Map<String, serde_json::Value>, rusqlite::Error> {
    let conn = rusqlite::Connection::open_with_flags(
        db_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;

    // Check whether ItemTable exists; gracefully return empty map if not.
    let table_exists: bool = conn
        .query_row(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name='ItemTable'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|n| n > 0)
        .unwrap_or(false);

    if !table_exists {
        return Ok(serde_json::Map::new());
    }

    let mut stmt = conn.prepare("SELECT key, value FROM ItemTable")?;
    let mut map = serde_json::Map::new();

    let rows = stmt.query_map([], |row| {
        let key: String = row.get(0)?;
        let value: Option<String> = row.get(1)?;
        Ok((key, value))
    })?;

    for row in rows.flatten() {
        let (key, value) = row;
        let json_val = match value {
            Some(s) => serde_json::Value::String(s),
            None => serde_json::Value::Null,
        };
        map.insert(key, json_val);
    }

    Ok(map)
}

// ---------------------------------------------------------------------------
// Tests (synthetic fixtures only — no real secrets)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
    use data_encoding::BASE32;

    fn b64(raw: &[u8]) -> String {
        STANDARD.encode(raw)
    }

    #[test]
    fn decode_plain_json_blob() {
        let raw = br#"{"state":"signedIn","context":{"tier":"ultra"}}"#;
        let v = decode_unified_state(&b64(raw)).unwrap();
        assert_eq!(v["raw_len"].as_u64().unwrap(), raw.len() as u64);
        assert_eq!(v["embedded_json"]["state"], "signedIn");
        assert_eq!(v["embedded_json"]["context"]["tier"], "ultra");
    }

    #[test]
    fn decode_json_with_protobuf_prefix_and_suffix() {
        // Simulate base64(protobuf) where JSON is embedded among wire bytes.
        let mut raw = vec![0x0a, 0x05, 0x08, 0x96, 0x01];
        raw.extend_from_slice(br#"{"state":"signedOut"}"#);
        raw.extend_from_slice(&[0x12, 0x03, 0xff, 0x00, 0x7b]); // stray '{'
        let v = decode_unified_state(&b64(&raw)).unwrap();
        assert_eq!(v["embedded_json"]["state"], "signedOut");
    }

    #[test]
    fn decode_url_safe_fallback() {
        // URL-safe base64 (no padding) of a JSON doc.
        let raw = br#"{"state":"signedIn"}"#;
        let encoded = URL_SAFE_NO_PAD.encode(raw);
        // Make sure it actually contains url-safe-only chars at least sometimes;
        // regardless, decode must succeed via the fallback path.
        let v = decode_unified_state(&encoded).unwrap();
        assert_eq!(v["embedded_json"]["state"], "signedIn");
    }

    #[test]
    fn decode_braces_inside_strings_are_balanced() {
        let raw = br#"{"note":"a } brace { inside","state":"signedIn"}"#;
        let v = decode_unified_state(&b64(raw)).unwrap();
        assert_eq!(v["embedded_json"]["state"], "signedIn");
        assert_eq!(v["embedded_json"]["note"], "a } brace { inside");
    }

    #[test]
    fn decode_no_json_present_still_ok() {
        let raw = &[0x0a, 0x05, 0x08, 0x96, 0x01, 0x12, 0x02];
        let v = decode_unified_state(&b64(raw)).unwrap();
        assert_eq!(v["raw_len"].as_u64().unwrap(), raw.len() as u64);
        assert!(v.get("embedded_json").is_none());
    }

    #[test]
    fn decode_base32_token_info_hint_no_bytes() {
        // Embed a base32 run; assert we report only the hint, never the bytes.
        let token = b"hello-token-info";
        let b32 = BASE32.encode(token); // uppercase A-Z2-7 with '=' padding
        let mut raw = Vec::new();
        raw.extend_from_slice(b32.as_bytes());
        let v = decode_unified_state(&b64(&raw)).unwrap();
        let hint = &v["base32_decoded_hint"];
        assert_eq!(hint["decoded_len"].as_u64().unwrap(), token.len() as u64);
        assert_eq!(hint["kind"], "text");
        // The decoded token bytes must NOT appear anywhere in the summary.
        let serialized = serde_json::to_string(&v).unwrap();
        assert!(!serialized.contains("hello-token-info"));
        assert!(!serialized.contains(&b32)); // not even the encoded form leaks
    }

    #[test]
    fn decode_empty_is_error() {
        assert!(matches!(decode_unified_state("   "), Err(StateSyncError::Empty)));
    }

    #[test]
    fn decode_invalid_base64_is_error() {
        // Characters illegal in every base64 alphabet.
        let err = decode_unified_state("!!! not base64 @@@").unwrap_err();
        assert!(matches!(err, StateSyncError::Base64(_)));
    }

    #[test]
    fn decode_never_panics_on_arbitrary_input() {
        // Fuzz-ish: a range of odd inputs must produce Ok or Err, never panic.
        for s in ["A", "AAAA", "====", "{", "}", "AB==CD", "Zm9v", "🦀🦀🦀"] {
            let _ = decode_unified_state(s);
        }
    }

    #[test]
    fn parse_item_table_flags_unified_keys() {
        let dump = serde_json::json!({
            "antigravityUnifiedStateSync.oauthToken": "CgVoZWxsbw==",
            "antigravityUnifiedStateSync.profile": "{\"state\":\"signedIn\"}",
            "workbench.colorTheme": "dark",
        });
        let entries = parse_item_table(&dump);
        assert_eq!(entries.len(), 3);

        let oauth = entries.iter().find(|e| e.key.ends_with("oauthToken")).unwrap();
        assert!(oauth.is_unified_state_sync);
        assert!(oauth.looks_like_signin_state); // key contains "oauth"

        let profile = entries.iter().find(|e| e.key.ends_with("profile")).unwrap();
        assert!(profile.is_unified_state_sync);
        assert!(profile.looks_like_signin_state); // value contains "signedIn"
        assert_eq!(profile.value_len, "{\"state\":\"signedIn\"}".len());

        let theme = entries.iter().find(|e| e.key.ends_with("colorTheme")).unwrap();
        assert!(!theme.is_unified_state_sync);
        assert!(!theme.looks_like_signin_state);
    }

    #[test]
    fn parse_item_table_is_sorted_and_non_object_safe() {
        let dump = serde_json::json!({ "b": "1", "a": "2", "c": "3" });
        let entries = parse_item_table(&dump);
        let keys: Vec<&str> = entries.iter().map(|e| e.key.as_str()).collect();
        assert_eq!(keys, ["a", "b", "c"]);

        // Non-object inputs return empty.
        assert!(parse_item_table(&serde_json::json!([1, 2, 3])).is_empty());
        assert!(parse_item_table(&serde_json::json!("scalar")).is_empty());
        assert!(parse_item_table(&serde_json::Value::Null).is_empty());
    }

    #[test]
    fn parse_item_table_emits_no_value_bytes() {
        // The entry summary must never carry the value content itself.
        let secret_ish = "SUPERSECRETVALUE123";
        let dump = serde_json::json!({
            "antigravityUnifiedStateSync.oauthToken": secret_ish,
        });
        let entries = parse_item_table(&dump);
        let serialized = serde_json::to_string(&entries).unwrap();
        assert!(!serialized.contains(secret_ish));
        assert!(serialized.contains("value_len"));
    }
}
