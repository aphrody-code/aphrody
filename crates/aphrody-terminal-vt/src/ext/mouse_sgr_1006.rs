// SPDX-License-Identifier: Apache-2.0
//! Stateless SGR 1006 mouse decoder + enable/disable sequences.
//!
//! Wire format (xterm extension):
//!
//! ```text
//! CSI < <button> ; <col> ; <row> M    (press / motion)
//! CSI < <button> ; <col> ; <row> m    (release)
//! ```
//!
//! `<button>` is a bitfield: low bits encode the physical button
//! (0=left, 1=middle, 2=right, 64+=wheel), with bits 4/8/16 carrying
//! shift/alt/ctrl modifiers and bit 32 indicating a motion-while-pressed.
//!
//! Unlike the legacy X10 protocol the coordinates are decimal text, so
//! columns and rows above 223 round-trip cleanly — required for any
//! modern wide-terminal TUI.

/// Whether this SGR mouse event is a press, release or drag/motion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SgrMouseKind {
    Press,
    Release,
    /// "Motion while a button is held down" — bit 32 set in the button
    /// code with `'M'` as the action byte.
    Move,
}

/// Decoded SGR 1006 mouse event.
///
/// `button` preserves the **raw** button code (including modifier and
/// motion bits) so downstream consumers can re-emit it or inspect
/// shift/alt/ctrl without losing information.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SgrMouseEvent {
    /// Raw button code as it appeared on the wire (with modifier + motion
    /// bits OR'd in).
    pub button: u16,
    /// 1-based column.
    pub x: u16,
    /// 1-based row.
    pub y: u16,
    pub kind: SgrMouseKind,
}

impl SgrMouseEvent {
    /// True if the shift modifier bit (4) is set.
    #[must_use]
    pub const fn shift(self) -> bool {
        (self.button & 0b0000_0100) != 0
    }
    /// True if the alt/meta modifier bit (8) is set.
    #[must_use]
    pub const fn alt(self) -> bool {
        (self.button & 0b0000_1000) != 0
    }
    /// True if the ctrl modifier bit (16) is set.
    #[must_use]
    pub const fn ctrl(self) -> bool {
        (self.button & 0b0001_0000) != 0
    }
    /// True if the motion bit (32) is set.
    #[must_use]
    pub const fn motion(self) -> bool {
        (self.button & 0b0010_0000) != 0
    }
}

/// Parse the three numeric fields of a `CSI < Pb ; Px ; Py {M|m}`
/// sequence into an [`SgrMouseEvent`].
///
/// `params` must contain the three flattened parameters from `vte` (or
/// any equivalent CSI splitter). `action_char` is the final byte: `'M'`
/// for press / motion, `'m'` for release. Anything else returns `None`.
#[must_use]
pub fn parse_csi_m(params: &[u16], action_char: char) -> Option<SgrMouseEvent> {
    if params.len() != 3 {
        return None;
    }
    let button = params[0];
    let x = params[1];
    let y = params[2];
    let kind = match action_char {
        'M' => {
            if (button & 0b0010_0000) != 0 {
                SgrMouseKind::Move
            } else {
                SgrMouseKind::Press
            }
        },
        'm' => SgrMouseKind::Release,
        _ => return None,
    };
    Some(SgrMouseEvent { button, x, y, kind })
}

/// Byte sequence to ENABLE SGR 1006 mouse mode + X11 button + motion
/// reporting (`?1006h` + `?1003h`).
#[must_use]
pub const fn enable_seq() -> &'static str {
    "\x1b[?1006h\x1b[?1003h"
}

/// Byte sequence to DISABLE SGR 1006 mouse mode + motion reporting
/// (`?1003l` + `?1006l`). Issued in reverse order so the wire format
/// downgrades before the encoding flips off.
#[must_use]
pub const fn disable_seq() -> &'static str {
    "\x1b[?1003l\x1b[?1006l"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn press_release_basic() {
        let p = parse_csi_m(&[0, 10, 20], 'M').unwrap();
        assert_eq!(p, SgrMouseEvent { button: 0, x: 10, y: 20, kind: SgrMouseKind::Press });

        let r = parse_csi_m(&[0, 10, 20], 'm').unwrap();
        assert_eq!(r, SgrMouseEvent { button: 0, x: 10, y: 20, kind: SgrMouseKind::Release });
    }

    #[test]
    fn motion_bit_preserved() {
        // button=32 → motion-only drag with no button held.
        let m = parse_csi_m(&[32, 5, 7], 'M').unwrap();
        assert_eq!(m.kind, SgrMouseKind::Move);
        assert!(m.motion());
        assert!(!m.shift() && !m.alt() && !m.ctrl());
    }

    #[test]
    fn modifier_flags_decoded() {
        // ctrl + shift + left press: 0 | 4 | 16 = 20.
        let p = parse_csi_m(&[20, 1, 1], 'M').unwrap();
        assert!(p.shift());
        assert!(p.ctrl());
        assert!(!p.alt());
        assert!(!p.motion());
    }

    #[test]
    fn enable_disable_seqs_are_stable() {
        assert_eq!(enable_seq(), "\x1b[?1006h\x1b[?1003h");
        assert_eq!(disable_seq(), "\x1b[?1003l\x1b[?1006l");
    }

    #[test]
    fn bad_action_or_arity_rejected() {
        assert!(parse_csi_m(&[0, 1], 'M').is_none());
        assert!(parse_csi_m(&[0, 1, 2, 3], 'M').is_none());
        assert!(parse_csi_m(&[0, 1, 2], 'X').is_none());
    }
}
