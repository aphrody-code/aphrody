// SPDX-License-Identifier: Apache-2.0
//! OSC 52 clipboard get/set decoder.
//!
//! Wire format (xterm + tmux):
//!
//! ```text
//! OSC 52 ; <selectors> ; <base64-or-?> ST
//! ```
//!
//! `<selectors>` is a concatenation of single-letter clipboard kinds:
//! `c` (system clipboard), `p` (primary X selection), `s` (secondary X
//! selection), `0`-`7` (cut buffers). When the payload is the literal
//! `?` the sender is *querying* the clipboard contents rather than
//! setting them.
//!
//! Validated against wterm's `packages/vt-decoder` reference parser (Vercel)
//! and the OSC 52 semantics documented at
//! <https://invisible-island.net/xterm/ctlseqs/ctlseqs.html>.

use base64::Engine;

/// Which clipboard the OSC 52 sequence targets.
///
/// `Clipboard` is the system clipboard (the modern default), `Primary`
/// and `Selection` are the X11 PRIMARY and SECONDARY selections, and
/// `Cut0`..`Cut7` correspond to xterm cut buffers 0-7. The first
/// selector character of a multi-target selector string wins (matching
/// the precedence used by tmux and ghostty).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClipboardKind {
    Primary,
    Selection,
    Clipboard,
    Cut0,
    Cut1,
    Cut2,
    Cut3,
    Cut4,
    Cut5,
    Cut6,
    Cut7,
}

impl ClipboardKind {
    fn from_byte(b: u8) -> Option<Self> {
        match b {
            b'p' => Some(Self::Primary),
            b's' => Some(Self::Selection),
            b'c' => Some(Self::Clipboard),
            b'0' => Some(Self::Cut0),
            b'1' => Some(Self::Cut1),
            b'2' => Some(Self::Cut2),
            b'3' => Some(Self::Cut3),
            b'4' => Some(Self::Cut4),
            b'5' => Some(Self::Cut5),
            b'6' => Some(Self::Cut6),
            b'7' => Some(Self::Cut7),
            _ => None,
        }
    }
}

/// Decoded OSC 52 operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Osc52 {
    /// `OSC 52 ; <kind> ; <base64>` — set the clipboard.
    Set {
        clipboard: ClipboardKind,
        /// Already base64-decoded payload bytes (validated round-trip).
        payload: Vec<u8>,
    },
    /// `OSC 52 ; <kind> ; ?` — query the clipboard contents.
    Query(ClipboardKind),
}

/// Parse the **body** of an OSC 52 sequence (everything between
/// `OSC 52 ;` and the terminator).
///
/// Accepts either a raw `<selectors>;<payload>` body, or the full
/// `52;<selectors>;<payload>` form (the `52;` prefix is stripped if
/// present). Returns `None` for malformed payloads, unknown clipboard
/// kinds, or invalid base64.
#[must_use]
pub fn parse(osc_payload: &[u8]) -> Option<Osc52> {
    // Allow callers to hand in the full `52;…` form for convenience.
    let body = osc_payload.strip_prefix(b"52;").unwrap_or(osc_payload);

    // Split exactly once on the first ';' — the payload itself may
    // contain `;` and is base64-safe, but in practice base64 never
    // contains `;` so a single split is correct.
    let sep = body.iter().position(|&b| b == b';')?;
    let (selectors, rest) = body.split_at(sep);
    let data = &rest[1..]; // skip the ';'

    // The first valid selector wins (tmux precedence).
    let clipboard = selectors.iter().find_map(|&b| ClipboardKind::from_byte(b))?;

    if data == b"?" {
        return Some(Osc52::Query(clipboard));
    }
    let decoded = base64::engine::general_purpose::STANDARD.decode(data).ok()?;
    Some(Osc52::Set { clipboard, payload: decoded })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_clipboard_decodes_base64() {
        // base64("Hello") = "SGVsbG8="
        let p = parse(b"c;SGVsbG8=").unwrap();
        match p {
            Osc52::Set { clipboard, payload } => {
                assert_eq!(clipboard, ClipboardKind::Clipboard);
                assert_eq!(payload, b"Hello");
            },
            other => panic!("expected Set, got {other:?}"),
        }
    }

    #[test]
    fn query_form_recognised() {
        let q = parse(b"p;?").unwrap();
        assert_eq!(q, Osc52::Query(ClipboardKind::Primary));
    }

    #[test]
    fn accepts_full_52_prefix() {
        let p = parse(b"52;c;SGVsbG8=").unwrap();
        match p {
            Osc52::Set { clipboard, payload } => {
                assert_eq!(clipboard, ClipboardKind::Clipboard);
                assert_eq!(payload, b"Hello");
            },
            _ => panic!("expected Set"),
        }
    }

    #[test]
    fn invalid_base64_rejected() {
        assert!(parse(b"c;!!!not-b64!!!").is_none());
    }

    #[test]
    fn unknown_kind_rejected() {
        assert!(parse(b"z;SGVsbG8=").is_none());
    }
}
