// SPDX-License-Identifier: Apache-2.0
//! Bracketed paste decoder (DECSET 2004 + `ESC [ 200~` / `ESC [ 201~`).
//!
//! When an application enables bracketed-paste mode the terminal wraps
//! any clipboard-pasted text in the framing sequences `ESC [ 200~ … ESC
//! [ 201~`. This lets editors and TUIs distinguish typed input from
//! pasted input — critical for security (Claude Code refuses to execute
//! pasted shell text without confirmation).

/// Enable/disable state reported by [`handle_csi_decset`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BracketedPasteState {
    Enabled,
    Disabled,
}

/// Decode `CSI ? 2004 h` / `CSI ? 2004 l` into a [`BracketedPasteState`].
///
/// Caller is responsible for verifying the `?` private intermediate and
/// passing the flattened parameter list. Returns `None` if the params
/// don't contain mode `2004` or `action` is not `h`/`l`.
#[must_use]
pub fn handle_csi_decset(params: &[u16], action: char) -> Option<BracketedPasteState> {
    let state = match action {
        'h' => BracketedPasteState::Enabled,
        'l' => BracketedPasteState::Disabled,
        _ => return None,
    };
    if params.iter().any(|&m| m == 2004) { Some(state) } else { None }
}

/// Byte-by-byte accumulator that recognises `ESC [ 200~ … ESC [ 201~`
/// boundaries in an arbitrary input stream.
///
/// Feed bytes one at a time via [`feed`](Self::feed); the accumulator
/// returns `Some(payload)` the moment it sees the closing `ESC [ 201~`
/// and is ready to accept the next paste.
#[derive(Default, Debug)]
pub struct PasteAccumulator {
    buf: Vec<u8>,
    in_paste: bool,
    /// Sliding-window tail used to detect the start/end markers without
    /// allocating per-byte regex state.
    tail: Vec<u8>,
}

const START: &[u8] = b"\x1b[200~";
const END: &[u8] = b"\x1b[201~";

impl PasteAccumulator {
    /// Construct an empty accumulator (not currently inside a paste).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// True iff we are currently between a start and end marker.
    #[must_use]
    pub fn in_paste(&self) -> bool {
        self.in_paste
    }

    /// Feed a single byte from the wire.
    ///
    /// Returns `Some(buf)` on the byte that completes the closing
    /// `ESC [ 201~` marker, with `buf` containing every byte that was
    /// fed strictly between the markers. Returns `None` otherwise.
    ///
    /// Bytes fed *outside* a paste are silently discarded — callers
    /// that need to see them should route the stream through their
    /// own splitter first and only feed the paste-active section here.
    pub fn feed(&mut self, byte: u8) -> Option<Vec<u8>> {
        if !self.in_paste {
            self.tail.push(byte);
            // Keep the tail bounded to the marker length so we don't
            // grow without bound on long non-paste prefixes.
            if self.tail.len() > START.len() {
                let excess = self.tail.len() - START.len();
                self.tail.drain(..excess);
            }
            if self.tail.ends_with(START) {
                self.in_paste = true;
                self.tail.clear();
            }
            return None;
        }

        // Inside a paste: append, then check whether the tail of `buf`
        // now matches the END marker.
        self.buf.push(byte);
        if self.buf.ends_with(END) {
            let new_len = self.buf.len() - END.len();
            let mut out = core::mem::take(&mut self.buf);
            out.truncate(new_len);
            self.in_paste = false;
            self.tail.clear();
            return Some(out);
        }
        None
    }

    /// Reset the accumulator, discarding any in-flight paste buffer.
    pub fn reset(&mut self) {
        self.buf.clear();
        self.tail.clear();
        self.in_paste = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decset_2004_toggles_state() {
        assert_eq!(handle_csi_decset(&[2004], 'h'), Some(BracketedPasteState::Enabled));
        assert_eq!(handle_csi_decset(&[2004], 'l'), Some(BracketedPasteState::Disabled));
        assert_eq!(handle_csi_decset(&[1000], 'h'), None);
        assert_eq!(handle_csi_decset(&[2004], 'x'), None);
    }

    #[test]
    fn accumulator_extracts_payload() {
        let mut acc = PasteAccumulator::new();
        // Feed start marker.
        for &b in START {
            assert!(acc.feed(b).is_none());
        }
        assert!(acc.in_paste());
        // Feed body.
        for &b in b"hello" {
            assert!(acc.feed(b).is_none());
        }
        // Feed end marker — the *last* byte returns Some(payload).
        let mut out: Option<Vec<u8>> = None;
        for &b in END {
            out = acc.feed(b);
        }
        assert_eq!(out, Some(b"hello".to_vec()));
        assert!(!acc.in_paste());
    }
}
