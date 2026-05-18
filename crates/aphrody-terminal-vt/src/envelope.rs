// SPDX-License-Identifier: Apache-2.0
//! Canonical OSC envelope strip helpers.
//!
//! Every `aphrody-*` OSC sequence is framed as:
//!
//! ```text
//! ESC ]  <payload>  BEL              (\x1b]<payload>\x07)
//! ESC ]  <payload>  ESC \\            (\x1b]<payload>\x1b\\)   ST terminator
//! ```
//!
//! Higher layers (`aphrody-terminal-llm`, `aphrody-terminal-markdown`,
//! `aphrody-terminal-browser`) all need to peel that envelope off before
//! parsing the payload body. Before T6 dedup, three crates duplicated the
//! same prefix/suffix-strip logic. This module is the single source of
//! truth — every other crate routes here.
//!
//! ## API surface
//!
//! - [`strip_osc_envelope`] — tolerant byte strip (used when the VT bus may have already removed
//!   the introducer/terminator before forwarding).
//! - [`strip_osc_envelope_str`] — string-slice variant for callers that parse OSC content as UTF-8.
//! - [`strip_osc_envelope_required`] — strict byte strip that returns `None` if the introducer or
//!   terminator is missing. Use for callers that consume the raw VT slice directly.
//!
//! All three are zero-copy: they return sub-slices of `input`.

/// OSC introducer byte sequence: `ESC ]` (`0x1B 0x5D`).
pub const OSC_INTRODUCER: &[u8] = b"\x1b]";

/// BEL terminator (`0x07`) — most common OSC string terminator in xterm.
pub const BEL: u8 = 0x07;

/// ST terminator (`ESC \`, i.e. `0x1B 0x5C`) — required by VT100/VT220 and
/// preferred over BEL in modern terminals.
pub const ST: &[u8] = b"\x1b\\";

/// Strip an OSC envelope from a byte slice, tolerantly.
///
/// Removes (if present) the leading `ESC ]` introducer and the trailing
/// `BEL` or `ST` terminator. Returns the original slice unchanged when
/// neither is present, so callers can hand in either a framed OSC sequence
/// or a bare payload body.
///
/// This mirrors the per-call semantics previously inlined in
/// `aphrody-terminal-browser::osc::strip_osc_framing` and
/// `aphrody-terminal-markdown::strip_osc_framing`.
#[must_use]
pub fn strip_osc_envelope(input: &[u8]) -> &[u8] {
    let mut out = input;
    if let Some(rest) = out.strip_prefix(OSC_INTRODUCER) {
        out = rest;
    }
    if let Some(rest) = out.strip_suffix(&[BEL]) {
        out = rest;
    } else if let Some(rest) = out.strip_suffix(ST) {
        out = rest;
    }
    out
}

/// String-slice variant of [`strip_osc_envelope`].
///
/// Identical semantics, but operates on a `&str` (zero-copy). Useful for
/// markdown / JSON-payload parsers that already hold a UTF-8 view of the
/// OSC content.
#[must_use]
pub fn strip_osc_envelope_str(input: &str) -> &str {
    let mut out = input;
    if let Some(rest) = out.strip_prefix("\x1b]") {
        out = rest;
    }
    if let Some(rest) = out.strip_suffix('\u{07}') {
        out = rest;
    } else if let Some(rest) = out.strip_suffix("\x1b\\") {
        out = rest;
    }
    out
}

/// Strict variant: require both the `ESC ]` introducer and one of the
/// recognised terminators (`BEL` or `ST`).
///
/// Returns `None` if either is missing. Use this when the caller is
/// reading directly from the VT byte stream and needs to reject
/// half-formed or bare-payload input.
///
/// This mirrors the semantics previously inlined in
/// `aphrody-terminal-llm::osc::parse_aphrody_llm_osc`.
#[must_use]
pub fn strip_osc_envelope_required(input: &[u8]) -> Option<&[u8]> {
    let body = input.strip_prefix(OSC_INTRODUCER)?;
    if let Some(body) = body.strip_suffix(&[BEL]) { Some(body) } else { body.strip_suffix(ST) }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // ---- strip_osc_envelope (tolerant, bytes) ------------------------------

    #[test]
    fn tolerant_bytes_framed_bel() {
        assert_eq!(strip_osc_envelope(b"\x1b]aphrody-md;SGk=\x07"), b"aphrody-md;SGk=");
    }

    #[test]
    fn tolerant_bytes_framed_st() {
        assert_eq!(strip_osc_envelope(b"\x1b]aphrody-md;SGk=\x1b\\"), b"aphrody-md;SGk=");
    }

    #[test]
    fn tolerant_bytes_bare_payload_passthrough() {
        assert_eq!(strip_osc_envelope(b"aphrody-md;SGk="), b"aphrody-md;SGk=");
    }

    #[test]
    fn tolerant_bytes_prefix_only() {
        assert_eq!(strip_osc_envelope(b"\x1b]aphrody-md;SGk="), b"aphrody-md;SGk=");
    }

    #[test]
    fn tolerant_bytes_terminator_only() {
        assert_eq!(strip_osc_envelope(b"aphrody-md;SGk=\x07"), b"aphrody-md;SGk=");
    }

    #[test]
    fn tolerant_bytes_empty() {
        assert_eq!(strip_osc_envelope(b""), b"");
    }

    // ---- strip_osc_envelope_str (tolerant, str) ----------------------------

    #[test]
    fn tolerant_str_framed_bel() {
        assert_eq!(strip_osc_envelope_str("\x1b]aphrody-md;SGk=\x07"), "aphrody-md;SGk=");
    }

    #[test]
    fn tolerant_str_framed_st() {
        assert_eq!(strip_osc_envelope_str("\x1b]aphrody-md;SGk=\x1b\\"), "aphrody-md;SGk=");
    }

    #[test]
    fn tolerant_str_bare_payload_passthrough() {
        assert_eq!(strip_osc_envelope_str("aphrody-md;SGk="), "aphrody-md;SGk=");
    }

    // ---- strip_osc_envelope_required (strict, bytes) ----------------------

    #[test]
    fn strict_bel_ok() {
        assert_eq!(
            strip_osc_envelope_required(b"\x1b]aphrody-md;SGk=\x07"),
            Some(b"aphrody-md;SGk=".as_slice())
        );
    }

    #[test]
    fn strict_st_ok() {
        assert_eq!(
            strip_osc_envelope_required(b"\x1b]aphrody-md;SGk=\x1b\\"),
            Some(b"aphrody-md;SGk=".as_slice())
        );
    }

    #[test]
    fn strict_missing_prefix_rejected() {
        assert_eq!(strip_osc_envelope_required(b"aphrody-md;SGk=\x07"), None);
    }

    #[test]
    fn strict_missing_terminator_rejected() {
        assert_eq!(strip_osc_envelope_required(b"\x1b]aphrody-md;SGk="), None);
    }

    #[test]
    fn strict_bare_rejected() {
        assert_eq!(strip_osc_envelope_required(b"aphrody-md;SGk="), None);
    }

    // ---- Cross-impl consistency: tolerant + strict agree on framed input -

    #[test]
    fn osc_envelope_strip_dedup_consistent() {
        // Three payloads that historically lived in two duplicated sites.
        let payloads: [&[u8]; 3] = [
            b"\x1b]aphrody-md;SGVsbG8=\x07",
            b"\x1b]aphrody-json;eyJrIjoidiJ9\x1b\\",
            b"\x1b]aphrody-task;t1;done;subj\x07",
        ];

        for raw in payloads {
            let tolerant_bytes = strip_osc_envelope(raw);
            let strict_bytes = strip_osc_envelope_required(raw).unwrap();
            let tolerant_str = strip_osc_envelope_str(std::str::from_utf8(raw).unwrap());

            // Both byte variants must produce the same envelope-stripped body.
            assert_eq!(
                tolerant_bytes, strict_bytes,
                "tolerant vs strict byte impl diverged for {raw:?}"
            );
            // The str variant must match the byte variant when input is UTF-8.
            assert_eq!(
                tolerant_str.as_bytes(),
                tolerant_bytes,
                "str vs byte impl diverged for {raw:?}"
            );
        }
    }
}
