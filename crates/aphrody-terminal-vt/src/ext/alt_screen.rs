// SPDX-License-Identifier: Apache-2.0
//! Alternate-screen DECSET/DECRST decoder.
//!
//! Modern TUIs (Claude Code, Gemini CLI, Ink/React, ratatui) request the
//! alternate screen with `CSI ? 1049 h` and restore with `CSI ? 1049 l`.
//! The legacy aliases `?47` and `?1047` are still emitted by older
//! ncurses-based programs and must be accepted to avoid corrupting the
//! primary buffer when interop traffic flows through the parser.

/// Alternate-screen transition reported by [`handle_csi_decset`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AltScreenEvent {
    /// `CSI ? 1049 h` (or `?47h` / `?1047h`) — switch to alt buffer.
    Enter,
    /// `CSI ? 1049 l` (or `?47l` / `?1047l`) — restore primary buffer.
    Leave,
}

/// Decode a private DECSET / DECRST sequence into an [`AltScreenEvent`].
///
/// `intermediates` must contain `b'?'` as the first byte (i.e. the
/// sequence is a *private* mode and not a standard ANSI mode). `params`
/// is the list of mode numbers (`vte::Params` flattened by the caller). `action` is the final byte: `'h'` for set, `'l'` for reset; anything
/// else returns `None`.
///
/// Returns `None` when the sequence does not target an alt-screen mode.
#[must_use]
pub fn handle_csi_decset(
    intermediates: &[u8],
    params: &[u16],
    action: char,
) -> Option<AltScreenEvent> {
    if intermediates.first() != Some(&b'?') {
        return None;
    }
    let enable = match action {
        'h' => true,
        'l' => false,
        _ => return None,
    };
    for &mode in params {
        // 1049 = save cursor + alt screen + clear; 1047 = alt screen only;
        // 47 = legacy alias used by older ncurses / vim.
        if matches!(mode, 1049 | 1047 | 47) {
            return Some(if enable { AltScreenEvent::Enter } else { AltScreenEvent::Leave });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modern_1049_enter_leave() {
        assert_eq!(handle_csi_decset(b"?", &[1049], 'h'), Some(AltScreenEvent::Enter));
        assert_eq!(handle_csi_decset(b"?", &[1049], 'l'), Some(AltScreenEvent::Leave));
    }

    #[test]
    fn legacy_aliases_accepted() {
        assert_eq!(handle_csi_decset(b"?", &[47], 'h'), Some(AltScreenEvent::Enter));
        assert_eq!(handle_csi_decset(b"?", &[1047], 'l'), Some(AltScreenEvent::Leave));
    }

    #[test]
    fn non_private_or_unrelated_mode_returns_none() {
        // Missing private '?' intermediate.
        assert_eq!(handle_csi_decset(b"", &[1049], 'h'), None);
        // Unrelated private mode (1000 = X11 mouse).
        assert_eq!(handle_csi_decset(b"?", &[1000], 'h'), None);
        // Wrong action byte.
        assert_eq!(handle_csi_decset(b"?", &[1049], 'm'), None);
    }
}
