// SPDX-License-Identifier: Apache-2.0
//! Output-side OSC writer for the `aphrody-md-*` LLM event sub-namespace.
//!
//! Mirrors the parser surface in `aphrody-terminal-llm::osc` — but writes
//! envelope bytes instead of consuming them. Three operations are defined:
//!
//! | Op                 | Payload (JSON)                              |
//! |--------------------|---------------------------------------------|
//! | `aphrody-md-rendered`  | `{"bytes_in":N,"bytes_out":N,"fence_count":N}` |
//! | `aphrody-md-error`     | `{"message":"…"}`                            |
//! | `aphrody-md-fence-lang`| `{"lang":"rust","line_count":N}`             |
//!
//! Envelope: `ESC ] aphrody-md-<op> ; <json> BEL` (BEL terminator chosen for
//! single-byte simplicity; consumers should accept `ESC \` ST as well per the
//! VT spec).

use std::io::{self, Write};

use serde::Serialize;

/// Markdown render-pipeline events surfaced over OSC for downstream LLM
/// consumers (host shells, hooks, JSON taps).
#[derive(Clone, Debug, Serialize)]
#[serde(untagged)]
pub enum MdEvent {
    /// A markdown document was rendered end-to-end.
    Rendered {
        /// Length of the input markdown source in bytes.
        bytes_in: u64,
        /// Length of the rendered ANSI output in bytes.
        bytes_out: u64,
        /// Number of fenced code blocks that were highlighted.
        fence_count: u32,
    },
    /// A non-fatal rendering error was caught.
    Error {
        /// Human-readable message (no secrets, no PII).
        message: String,
    },
    /// A fenced code block was recognised; the renderer is about to
    /// highlight `line_count` lines for `lang`.
    FenceLang {
        /// Language as parsed from the fence info string (`rust`, `py`, …).
        lang: String,
        /// Number of lines in the code block (counted before highlighting).
        line_count: u32,
    },
}

impl MdEvent {
    /// Return the OSC op suffix (`rendered`, `error`, `fence-lang`).
    #[must_use]
    pub const fn op(&self) -> &'static str {
        match self {
            Self::Rendered { .. } => "rendered",
            Self::Error { .. } => "error",
            Self::FenceLang { .. } => "fence-lang",
        }
    }
}

/// Write one OSC `aphrody-md-<op>;<json>` sequence terminated by BEL.
///
/// Returns the underlying [`io::Error`] on failure so callers can decide
/// whether to drop the event or surface it.
///
/// # Errors
/// Propagates [`io::Error`] from the writer; returns
/// [`io::ErrorKind::InvalidData`] if the event cannot be JSON-encoded
/// (cannot occur with the current `MdEvent` shape but kept for forward
/// compatibility).
pub fn emit_md_event<W: Write>(w: &mut W, evt: MdEvent) -> io::Result<()> {
    let op = evt.op();
    let json = serde_json::to_string(&evt)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    write!(w, "\x1b]aphrody-md-{op};{json}\x07")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rendered_envelope_round_trip() {
        let mut buf: Vec<u8> = Vec::new();
        emit_md_event(
            &mut buf,
            MdEvent::Rendered { bytes_in: 100, bytes_out: 250, fence_count: 1 },
        )
        .expect("write to Vec never fails");
        let s = String::from_utf8(buf).expect("ASCII-safe envelope");
        assert!(s.starts_with("\x1b]aphrody-md-rendered;"), "wrong prefix: {s:?}");
        assert!(s.ends_with('\x07'), "missing BEL terminator: {s:?}");
        assert!(s.contains("\"bytes_in\":100"));
        assert!(s.contains("\"bytes_out\":250"));
        assert!(s.contains("\"fence_count\":1"));
    }

    #[test]
    fn error_envelope() {
        let mut buf: Vec<u8> = Vec::new();
        emit_md_event(&mut buf, MdEvent::Error { message: "oops".to_owned() })
            .expect("write to Vec never fails");
        let s = String::from_utf8(buf).expect("ASCII-safe envelope");
        assert!(s.contains("aphrody-md-error;"));
        assert!(s.contains("\"message\":\"oops\""));
    }

    #[test]
    fn fence_lang_envelope() {
        let mut buf: Vec<u8> = Vec::new();
        emit_md_event(
            &mut buf,
            MdEvent::FenceLang { lang: "rust".to_owned(), line_count: 12 },
        )
        .expect("write to Vec never fails");
        let s = String::from_utf8(buf).expect("ASCII-safe envelope");
        assert!(s.contains("aphrody-md-fence-lang;"));
        assert!(s.contains("\"lang\":\"rust\""));
        assert!(s.contains("\"line_count\":12"));
    }
}
