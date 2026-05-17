// SPDX-License-Identifier: Apache-2.0
//! Mouse tracking encoder/decoder for SGR 1006 protocol.
//!
//! In SGR 1006 mode (xterm extension), the terminal emits mouse events as
//! `CSI < button ; col ; row M` (press / motion) or `m` (release). Unlike
//! the legacy X10/X11 protocols this encoding has no 223-byte coordinate
//! limit and is therefore the only sane choice for modern TUIs (Ink, Bubble
//! Tea, ratatui).

/// Mouse button identifiers as used by the SGR 1006 protocol.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    /// Wheel scroll up (button code 64).
    WheelUp,
    /// Wheel scroll down (button code 65).
    WheelDown,
    /// Motion with no button held (button code 35).
    None,
}

/// Bitmask modifier flags layered onto the SGR 1006 button byte.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct MouseModifiers {
    pub shift: bool,
    pub alt: bool,
    pub ctrl: bool,
}

impl MouseModifiers {
    fn mask(self) -> u32 {
        let mut m = 0;
        if self.shift {
            m |= 0b0000_0100;
        }
        if self.alt {
            m |= 0b0000_1000;
        }
        if self.ctrl {
            m |= 0b0001_0000;
        }
        m
    }
}

/// What kind of mouse interaction this event represents.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MouseEventKind {
    Press,
    Release,
    Move,
}

/// One decoded mouse interaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MouseEvent {
    /// 1-based column (matching SGR 1006 wire format).
    pub x: u32,
    /// 1-based row.
    pub y: u32,
    pub button: MouseButton,
    pub kind: MouseEventKind,
    pub modifiers: MouseModifiers,
}

/// Encode an event into the SGR 1006 byte sequence sent to the pty.
///
/// Returns bytes like `ESC [ < 0 ; 300 ; 500 M` for a left-button press at
/// column 300 row 500.
#[must_use]
pub fn encode_sgr_1006(ev: MouseEvent) -> Vec<u8> {
    // Base button code (without the motion bit). For `None` we use 3 (the
    // bare "no button" code) and let the motion bit add 32 to reach 35.
    let base: u32 = match ev.button {
        MouseButton::Left => 0,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
        MouseButton::WheelUp => 64,
        MouseButton::WheelDown => 65,
        MouseButton::None => 3,
    };
    let motion_bit: u32 = if matches!(ev.kind, MouseEventKind::Move) {
        0b0010_0000
    } else {
        0
    };
    let code = base | ev.modifiers.mask() | motion_bit;
    let final_byte = match ev.kind {
        MouseEventKind::Release => b'm',
        _ => b'M',
    };
    let mut out = Vec::with_capacity(16);
    out.extend_from_slice(b"\x1b[<");
    push_u32(&mut out, code);
    out.push(b';');
    push_u32(&mut out, ev.x);
    out.push(b';');
    push_u32(&mut out, ev.y);
    out.push(final_byte);
    out
}

/// Parse an SGR 1006 sequence (as emitted by other terminals) back into an
/// [`MouseEvent`]. Returns `None` if the slice is malformed.
///
/// Accepts both `ESC [ < ...` and the already-stripped tail `< ...`.
#[must_use]
pub fn decode_sgr_1006(bytes: &[u8]) -> Option<MouseEvent> {
    let tail = strip_csi_lt(bytes)?;
    let last = *tail.last()?;
    if last != b'M' && last != b'm' {
        return None;
    }
    let body = &tail[..tail.len() - 1];
    let mut parts = body.split(|&b| b == b';');
    let code: u32 = parse_u32(parts.next()?)?;
    let x: u32 = parse_u32(parts.next()?)?;
    let y: u32 = parse_u32(parts.next()?)?;
    if parts.next().is_some() {
        return None;
    }

    let motion = (code & 0b0010_0000) != 0;
    let modifiers = MouseModifiers {
        shift: (code & 0b0000_0100) != 0,
        alt: (code & 0b0000_1000) != 0,
        ctrl: (code & 0b0001_0000) != 0,
    };
    let base = code & !0b0011_1100;
    let button = match base {
        0 => MouseButton::Left,
        1 => MouseButton::Middle,
        2 => MouseButton::Right,
        3 => MouseButton::None,
        64 => MouseButton::WheelUp,
        65 => MouseButton::WheelDown,
        _ => return None,
    };
    let kind = if last == b'm' {
        MouseEventKind::Release
    } else if motion {
        MouseEventKind::Move
    } else {
        MouseEventKind::Press
    };
    Some(MouseEvent { x, y, button, kind, modifiers })
}

fn strip_csi_lt(bytes: &[u8]) -> Option<&[u8]> {
    if let Some(rest) = bytes.strip_prefix(b"\x1b[<") {
        Some(rest)
    } else {
        bytes.strip_prefix(b"<")
    }
}

fn parse_u32(b: &[u8]) -> Option<u32> {
    if b.is_empty() {
        return None;
    }
    let mut n: u32 = 0;
    for &c in b {
        if !c.is_ascii_digit() {
            return None;
        }
        n = n.checked_mul(10)?.checked_add(u32::from(c - b'0'))?;
    }
    Some(n)
}

fn push_u32(out: &mut Vec<u8>, mut v: u32) {
    if v == 0 {
        out.push(b'0');
        return;
    }
    let mut buf = [0u8; 10];
    let mut i = buf.len();
    while v > 0 {
        i -= 1;
        buf[i] = b'0' + (v % 10) as u8;
        v /= 10;
    }
    out.extend_from_slice(&buf[i..]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mouse_sgr_1006_parses_extended_coords() {
        // Encode a press well outside the 223-byte X10 limit and round-trip it.
        let ev = MouseEvent {
            x: 300,
            y: 500,
            button: MouseButton::Left,
            kind: MouseEventKind::Press,
            modifiers: MouseModifiers::default(),
        };
        let bytes = encode_sgr_1006(ev);
        assert_eq!(bytes, b"\x1b[<0;300;500M");
        let parsed = decode_sgr_1006(&bytes).expect("decode");
        assert_eq!(parsed, ev);

        // Release uses lowercase 'm' as the final byte.
        let release = MouseEvent {
            kind: MouseEventKind::Release,
            ..ev
        };
        assert_eq!(encode_sgr_1006(release), b"\x1b[<0;300;500m");

        // Right-click with ctrl modifier.
        let rc = MouseEvent {
            x: 10,
            y: 20,
            button: MouseButton::Right,
            kind: MouseEventKind::Press,
            modifiers: MouseModifiers {
                ctrl: true,
                ..Default::default()
            },
        };
        let rc_bytes = encode_sgr_1006(rc);
        // Code = button(2) | ctrl(16) = 18.
        assert_eq!(rc_bytes, b"\x1b[<18;10;20M");
        assert_eq!(decode_sgr_1006(&rc_bytes).unwrap(), rc);

        // Motion-only event (no button held) — base 3 + motion bit 32 = 35,
        // which is the canonical xterm encoding for "drag with no button".
        let mv = MouseEvent {
            x: 1,
            y: 1,
            button: MouseButton::None,
            kind: MouseEventKind::Move,
            modifiers: MouseModifiers::default(),
        };
        assert_eq!(encode_sgr_1006(mv), b"\x1b[<35;1;1M");
        assert_eq!(decode_sgr_1006(b"\x1b[<35;1;1M").unwrap(), mv);

        // Wheel up is button code 64 (no extra motion bit).
        let wu = MouseEvent {
            x: 4,
            y: 5,
            button: MouseButton::WheelUp,
            kind: MouseEventKind::Press,
            modifiers: MouseModifiers::default(),
        };
        assert_eq!(encode_sgr_1006(wu), b"\x1b[<64;4;5M");
    }
}
