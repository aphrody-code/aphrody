// SPDX-License-Identifier: Apache-2.0
//! NDJSON frame primitive — `StdoutFrame` envelope + sync `emit_frame` writer.
//!
//! Companion to the older [`crate::Envelope`] surface (kept for the
//! `spawn_framed_child` helper). The new frame layout is monotonic-ts-friendly
//! (`u64` milliseconds since boot) and uses a discriminated `kind`/`value`
//! payload shape so consumers don't have to introspect `serde_json::Value`.
//!
//! # Wire format
//!
//! Each frame serialises as a single NDJSON record (terminated by `\n`):
//!
//! ```json
//! {"ts":1747569731123,"source":"app","payload":{"kind":"text","value":"hello"}}
//! {"ts":1747569731124,"source":"app","payload":{"kind":"json","value":{"event":"build"}}}
//! {"ts":1747569731125,"source":"err","payload":{"kind":"bytes","value":"AQID"}}
//! ```
//!
//! # Pattern reuse
//!
//! - Mirrors the `serde_json::to_writer + writeln!` NDJSON emission style
//!   used by `n2b_report::render_ndjson`.
//! - Frame source enum mirrors the OSC dispatch source-tagging convention in
//!   `aphrody_terminal_llm::osc` (typed origin per event).
//! - Sink trait bound (`std::io::Write`) matches the streaming pattern used
//!   by `aphrody_terminal_backend` for line-oriented IO.

use std::io::Write;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::Engine as _;
use serde::{Deserialize, Serialize};

use crate::error::JsonOutError;

// ── Source tag ────────────────────────────────────────────────────────────────

/// Origin of a framed line. Maps to the `source` field on the wire.
///
/// `App` covers regular child stdout (the dominant case — application output).
/// `Err` is child stderr. `Meta` is reserved for control events emitted by the
/// framer itself (e.g. spawn / exit notices).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FrameSource {
    /// Default child stdout — application payload.
    App,
    /// Child stderr — diagnostics.
    Err,
    /// Out-of-band framer event (spawn / exit / heartbeat).
    Meta,
}

impl FrameSource {
    /// Stable lowercase form (matches the JSON serialisation).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::App => "app",
            Self::Err => "err",
            Self::Meta => "meta",
        }
    }
}

// ── Payload variant ───────────────────────────────────────────────────────────

/// Typed line payload.
///
/// The wire form uses an internally-tagged enum with a `kind` discriminator
/// and a `value` field, so downstream consumers can switch on `kind` without
/// pattern-matching against the shape of the JSON value itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "lowercase")]
pub enum FramePayload {
    /// Plain UTF-8 line that did not parse as JSON.
    Text(String),
    /// Line that parsed as valid JSON (object, array, number, bool, or null).
    /// Embedded verbatim — no double-encoding.
    Json(serde_json::Value),
    /// Binary blob (non-UTF-8) — base64 (STANDARD, padded) at the wire level.
    /// In-memory we hold the raw bytes; serialisation/deserialisation goes
    /// through a base64 string for human-readable NDJSON.
    Bytes(#[serde(with = "base64_bytes")] Vec<u8>),
}

impl FramePayload {
    /// The discriminator string (`"text"`, `"json"`, or `"bytes"`).
    #[must_use]
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Text(_) => "text",
            Self::Json(_) => "json",
            Self::Bytes(_) => "bytes",
        }
    }
}

mod base64_bytes {
    //! Serde helper to round-trip `Vec<u8>` as a base64 string on the wire.
    use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
    use serde::{Deserialize as _, Deserializer, Serializer};

    pub(super) fn serialize<S: Serializer>(bytes: &[u8], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&B64.encode(bytes))
    }

    pub(super) fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(d)?;
        B64.decode(&s).map_err(serde::de::Error::custom)
    }
}

// ── Frame envelope ────────────────────────────────────────────────────────────

/// One NDJSON record on the framer's output stream.
///
/// `ts` is **milliseconds since the Unix epoch**, sourced via a monotonic
/// process-wide counter so two frames emitted back-to-back in the same
/// millisecond still satisfy `ts(prev) <= ts(next)` (cf. the `monotonic_ts`
/// tests). This makes downstream ordering / dedup logic trivial without
/// needing tie-breaking on a sequence number.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StdoutFrame {
    /// Monotonic millisecond timestamp.
    pub ts: u64,
    /// Origin tag.
    pub source: FrameSource,
    /// Typed payload.
    pub payload: FramePayload,
}

impl StdoutFrame {
    /// Build a frame with the next monotonic timestamp.
    #[must_use]
    pub fn new(source: FrameSource, payload: FramePayload) -> Self {
        Self { ts: monotonic_ts_millis(), source, payload }
    }

    /// Build a frame with an explicit timestamp (escape hatch for tests +
    /// replay tooling).
    #[must_use]
    pub fn with_ts(ts: u64, source: FrameSource, payload: FramePayload) -> Self {
        Self { ts, source, payload }
    }

    /// Convenience: build a `Text` frame.
    #[must_use]
    pub fn text(source: FrameSource, line: impl Into<String>) -> Self {
        Self::new(source, FramePayload::Text(line.into()))
    }

    /// Convenience: build a `Json` frame.
    #[must_use]
    pub fn json(source: FrameSource, value: serde_json::Value) -> Self {
        Self::new(source, FramePayload::Json(value))
    }

    /// Convenience: build a `Bytes` frame.
    #[must_use]
    pub fn bytes(source: FrameSource, raw: impl Into<Vec<u8>>) -> Self {
        Self::new(source, FramePayload::Bytes(raw.into()))
    }
}

// ── Sync NDJSON emitter ───────────────────────────────────────────────────────

/// Serialise `frame` as one NDJSON record (object + trailing `\n`) and write
/// the bytes to `w`.
///
/// Uses `serde_json::to_writer` (zero intermediate allocation aside from the
/// serde codec's internal buffer) followed by a single one-byte newline.
/// Mirrors the `n2b_report::render_ndjson` emission style.
///
/// # Errors
///
/// Returns [`JsonOutError::Serialize`] if serialisation fails (in practice
/// impossible — every field is a string, byte vector, or `serde_json::Value`),
/// or [`JsonOutError::Io`] if writing to `w` fails.
pub fn emit_frame<W: Write>(w: &mut W, frame: &StdoutFrame) -> Result<(), JsonOutError> {
    serde_json::to_writer(&mut *w, frame).map_err(JsonOutError::Serialize)?;
    w.write_all(b"\n")?;
    Ok(())
}

/// Convenience wrapper that returns the full NDJSON record as a `String`.
///
/// # Errors
///
/// Returns [`JsonOutError::Serialize`] on serde failure.
pub fn frame_to_jsonl(frame: &StdoutFrame) -> Result<String, JsonOutError> {
    let mut buf = Vec::with_capacity(128);
    emit_frame(&mut buf, frame)?;
    String::from_utf8(buf).map_err(|e| JsonOutError::Io(std::io::Error::other(e.to_string())))
}

// ── Monotonic ts ──────────────────────────────────────────────────────────────

/// Returns the current Unix timestamp in milliseconds, **clamped to never go
/// backwards** within a single process. The clamp uses a process-wide
/// [`AtomicU64`] so tight loops that emit multiple frames inside one wall-clock
/// millisecond still produce non-decreasing `ts` values.
///
/// Wall-clock skew (NTP step-back) and SystemTime errors degrade gracefully:
/// the function returns the previous high-water mark in either case rather
/// than panicking or returning `0`.
#[must_use]
pub fn monotonic_ts_millis() -> u64 {
    static LAST: AtomicU64 = AtomicU64::new(0);

    let wall = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| u64::try_from(d.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0);

    // Read last, propose max(wall, last + tiebreak), CAS-loop until installed.
    loop {
        let prev = LAST.load(Ordering::Relaxed);
        let next = if wall > prev { wall } else { prev.saturating_add(0).max(prev) };
        // Tight-loop tiebreak: if wall == prev, bump by 1 ms so frames in the
        // same millisecond still have strictly non-decreasing (in fact
        // strictly increasing) timestamps relative to LAST.
        let installed = if next == prev { prev.saturating_add(1) } else { next };
        if LAST
            .compare_exchange(prev, installed, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            return installed;
        }
    }
}

// Re-export base64 engine for symmetry with the legacy `frame_binary` helper.
#[doc(hidden)]
pub fn encode_base64_padded(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_source_lowercase_serde() {
        let s = serde_json::to_string(&FrameSource::App).unwrap();
        assert_eq!(s, "\"app\"");
        let back: FrameSource = serde_json::from_str("\"err\"").unwrap();
        assert_eq!(back, FrameSource::Err);
    }

    #[test]
    fn payload_kind_discriminator() {
        let p = FramePayload::Text("x".into());
        let v = serde_json::to_value(&p).unwrap();
        assert_eq!(v["kind"], "text");
        assert_eq!(v["value"], "x");
    }

    #[test]
    fn bytes_payload_roundtrip_base64() {
        let p = FramePayload::Bytes(vec![0x01, 0x02, 0x03]);
        let s = serde_json::to_string(&p).unwrap();
        assert!(s.contains("AQID"), "got {s}");
        let back: FramePayload = serde_json::from_str(&s).unwrap();
        assert_eq!(back, p);
    }

    #[test]
    fn emit_frame_writes_trailing_newline() {
        let f = StdoutFrame::text(FrameSource::App, "hello");
        let mut buf = Vec::new();
        emit_frame(&mut buf, &f).unwrap();
        assert_eq!(buf.last().copied(), Some(b'\n'));
        // Exactly one newline byte (the terminator).
        assert_eq!(buf.iter().filter(|&&b| b == b'\n').count(), 1);
    }

    #[test]
    fn monotonic_ts_strictly_increases() {
        let a = monotonic_ts_millis();
        let b = monotonic_ts_millis();
        let c = monotonic_ts_millis();
        assert!(a < b && b < c, "expected strict monotonic order, got {a}/{b}/{c}");
    }
}
