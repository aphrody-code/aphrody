// SPDX-License-Identifier: Apache-2.0
//! Integration smoke tests for the 5 Ink/React-critical VT extensions
//! exposed under `aphrody_terminal_vt::ext`. These tests exercise the
//! stateless decoder API (not the full `TerminalState` Perform path),
//! since the spec for `docs/PLAN.md` §0.5 item #1 requires standalone
//! extension helpers that external integrators (Claude Code TUI, the
//! LLM bridge, the JSON-out filter) can use without spinning up the
//! whole screen buffer.

use aphrody_terminal_vt::ext::{
    AltScreenEvent, BracketedPasteState, ClipboardKind, Osc52, PasteAccumulator, ScrollMargins,
    SgrMouseEvent, SgrMouseKind, alt_screen, bracketed_paste, decstbm, mouse_sgr_1006, osc_52,
};

/// Minimal driver: pretend a CSI parser handed us the same `(intermediates,
/// params, action)` triple `vte` would produce, so we can feed raw byte
/// sequences and assert the decoded event.
fn drive_decset(seq: &[u8]) -> Option<AltScreenEvent> {
    // Expected shape: ESC '[' '?' <digits>(;<digits>)* ('h'|'l')
    let rest = seq.strip_prefix(b"\x1b[")?;
    let rest = rest.strip_prefix(b"?")?;
    let last = *rest.last()?;
    let body = &rest[..rest.len() - 1];
    let params: Vec<u16> = body
        .split(|&b| b == b';')
        .filter(|p| !p.is_empty())
        .filter_map(|p| std::str::from_utf8(p).ok()?.parse::<u16>().ok())
        .collect();
    alt_screen::handle_csi_decset(b"?", &params, last as char)
}

#[test]
fn alt_screen() {
    // Modern 1049 enter / leave.
    assert_eq!(drive_decset(b"\x1b[?1049h"), Some(AltScreenEvent::Enter));
    assert_eq!(drive_decset(b"\x1b[?1049l"), Some(AltScreenEvent::Leave));
    // Legacy ?47 alias used by old ncurses/vim must also map to Enter.
    assert_eq!(drive_decset(b"\x1b[?47h"), Some(AltScreenEvent::Enter));
    assert_eq!(drive_decset(b"\x1b[?47l"), Some(AltScreenEvent::Leave));
    // Unrelated private modes (e.g. mouse 1000) must NOT match.
    assert_eq!(drive_decset(b"\x1b[?1000h"), None);
}

#[test]
fn mouse_sgr_1006() {
    // CSI <0;10;20 M  →  left-button press at (10, 20).
    let press = mouse_sgr_1006::parse_csi_m(&[0, 10, 20], 'M').unwrap();
    assert_eq!(press, SgrMouseEvent { button: 0, x: 10, y: 20, kind: SgrMouseKind::Press });
    assert!(!press.motion());

    // CSI <0;10;20 m  →  release at same coords.
    let release = mouse_sgr_1006::parse_csi_m(&[0, 10, 20], 'm').unwrap();
    assert_eq!(release.kind, SgrMouseKind::Release);
    assert_eq!((release.x, release.y), (10, 20));

    // Bit 32 (motion) must be preserved through the decoder.
    let motion = mouse_sgr_1006::parse_csi_m(&[0b0010_0000, 1, 1], 'M').unwrap();
    assert_eq!(motion.kind, SgrMouseKind::Move);
    assert!(motion.motion());

    // Enable/disable byte sequences are stable.
    assert_eq!(mouse_sgr_1006::enable_seq(), "\x1b[?1006h\x1b[?1003h");
    assert_eq!(mouse_sgr_1006::disable_seq(), "\x1b[?1003l\x1b[?1006l");
}

#[test]
fn osc_52() {
    // Set form: base64("Hello") = "SGVsbG8=".
    let set = osc_52::parse(b"52;c;SGVsbG8=").unwrap();
    match set {
        Osc52::Set { clipboard, payload } => {
            assert_eq!(clipboard, ClipboardKind::Clipboard);
            assert_eq!(payload, b"Hello");
        },
        other => panic!("expected Set, got {other:?}"),
    }

    // Query form: payload is the literal '?'.
    let query = osc_52::parse(b"52;c;?").unwrap();
    assert_eq!(query, Osc52::Query(ClipboardKind::Clipboard));
}

#[test]
fn bracketed_paste() {
    // DECSET 2004 h → Enabled.
    assert_eq!(
        bracketed_paste::handle_csi_decset(&[2004], 'h'),
        Some(BracketedPasteState::Enabled)
    );

    // Accumulator end-to-end: start markers, body, end markers.
    let mut acc = PasteAccumulator::new();
    for &b in b"\x1b[200~" {
        assert!(acc.feed(b).is_none());
    }
    assert!(acc.in_paste());
    for &b in b"hello" {
        assert!(acc.feed(b).is_none());
    }
    let mut out: Option<Vec<u8>> = None;
    for &b in b"\x1b[201~" {
        out = acc.feed(b);
    }
    assert_eq!(out, Some(b"hello".to_vec()));
    assert!(!acc.in_paste());
}

#[test]
fn decstbm() {
    // Explicit margins.
    assert_eq!(decstbm::parse_csi_r(&[5, 20], 30), ScrollMargins { top: 5, bottom: 20 });
    // Missing params → default to full screen (1, rows).
    assert_eq!(decstbm::parse_csi_r(&[], 30), ScrollMargins { top: 1, bottom: 30 });
    // Invalid (top >= bottom) → Microsoft-style fallback to full screen.
    assert_eq!(decstbm::parse_csi_r(&[10, 5], 30), ScrollMargins { top: 1, bottom: 30 });
}
