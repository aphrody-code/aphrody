// SPDX-License-Identifier: Apache-2.0
//! OSC payload helpers (window title, clipboard).
//!
//! The VT parser routes OSC sequences to `osc_dispatch`; this module
//! decodes the well-known payloads (`OSC 0`, `OSC 52`) into ergonomic Rust
//! values for the host to consume.

use base64::Engine;

/// Decode an `OSC 0` (or `OSC 2`) title payload.
///
/// The wire format is `OSC 0 ; <title> ST`; vte gives us the parameter
/// vector with the leading `0` already split off. We just stringify the
/// remaining bytes.
pub(crate) fn decode_title(parts: &[&[u8]]) -> Option<String> {
    // parts[0] is the OSC selector; the title is everything after.
    if parts.len() < 2 {
        return None;
    }
    // Most senders use a single field; rejoin with ';' if the title itself
    // contained a literal semicolon.
    let joined = join_with_semicolon(&parts[1..]);
    String::from_utf8(joined).ok()
}

/// Decode an `OSC 52 ; <selectors> ; <base64-or-?> ST` clipboard payload.
///
/// Returns the decoded UTF-8 text, or `None` if the payload is malformed,
/// is a query (`?`), or the base64 fails to decode.
pub(crate) fn decode_clipboard(parts: &[&[u8]]) -> Option<String> {
    // Expect at least: ["52", "<selectors>", "<data>"].
    if parts.len() < 3 {
        return None;
    }
    let data = parts[2];
    if data == b"?" {
        // Query, not a set.
        return None;
    }
    let raw = base64::engine::general_purpose::STANDARD.decode(data).ok()?;
    String::from_utf8(raw).ok()
}

fn join_with_semicolon(slices: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();
    for (i, s) in slices.iter().enumerate() {
        if i > 0 {
            out.push(b';');
        }
        out.extend_from_slice(s);
    }
    out
}
