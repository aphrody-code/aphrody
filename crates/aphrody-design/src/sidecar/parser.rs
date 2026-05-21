// SPDX-License-Identifier: Apache-2.0
//! Streaming `<artifact>` block extractor.
//!
//! Agents (Claude Code, Gemini CLI, generic LLM stdout streams) emit
//! artifacts wrapped in a custom XML-ish envelope:
//!
//! ```xml
//! <artifact type="text/html" id="dash-1" title="Dashboard">
//!   <!doctype html><html>…</html>
//! </artifact>
//! ```
//!
//! `ArtifactParser` accepts incoming text chunk-by-chunk (the same way the
//! upstream sidecar's JSON-IPC framer splits stdout on newlines, cf.
//! `packages/sidecar/src/index.ts::createJsonIpcServer`) and yields
//! [`ParserEvent::Artifact`] each time a complete `<artifact>…</artifact>`
//! block has been seen.

use std::collections::HashMap;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::sidecar::error::{SidecarError, SidecarResult};

/// One complete artifact extracted from the stream.
///
/// Field naming follows the upstream `SidecarStampShape` contract: snake_case
/// in the wire JSON. `type_` carries the Rust trailing underscore convention
/// since `type` is a reserved keyword, but it serializes as `"type_"` (the
/// brief's spec — matches the Rust struct exactly).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Artifact {
    /// Caller-supplied stable id (must be unique within a stream).
    pub id: String,
    /// MIME type as declared on the `<artifact type="...">` attribute.
    #[serde(rename = "type_")]
    pub type_: String,
    /// Human-readable title for the preview UI.
    pub title: String,
    /// Raw payload (UTF-8). Adapter normalization happens downstream.
    pub content: String,
    /// Resolved MIME from the format adapter (set by the pipeline, defaults
    /// to `type_` until the artifact is normalized).
    pub mime: String,
}

/// Events the streaming parser emits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParserEvent {
    /// A complete artifact was finalized.
    Artifact(Artifact),
    /// Non-fatal parse problem; the offending block was skipped. The
    /// pipeline lifts this to `partial = true` on the manifest.
    Warning(String),
    /// End of input was reached, all pending state was flushed.
    Eof,
}

/// Streaming parser state.
///
/// The parser keeps a single rolling text buffer. Each call to
/// [`ArtifactParser::feed`] appends `chunk`, then drains every fully
/// terminated `<artifact …>…</artifact>` block it can find.
///
/// We intentionally do **not** use a full XML reader on the rolling buffer
/// (artifact payloads may be raw HTML / Markdown / binary-as-base64 that is
/// not well-formed XML); instead, we scan with a precompiled regex pair
/// matching the open / close tags. quick-xml is still used to parse the
/// opening tag's attributes (escaped values, single/double quotes, etc.).
pub struct ArtifactParser {
    buffer: String,
    open_re: Regex,
    close_tag: &'static str,
}

impl Default for ArtifactParser {
    fn default() -> Self {
        Self::new()
    }
}

impl ArtifactParser {
    /// Construct a fresh parser.
    #[must_use]
    pub fn new() -> Self {
        // Open tag: `<artifact ...>` (attributes optional but our schema
        // requires `type` + `id`; that's validated below, not in the regex).
        // The `(?s)` flag is irrelevant for the open tag (no newlines in it)
        // — newlines may appear in attribute values but we forbid them.
        let open_re = Regex::new(r"<artifact\b[^>]*>").expect("static regex compiles");
        Self { buffer: String::new(), open_re, close_tag: "</artifact>" }
    }

    /// Append a new chunk of streamed text and drain every complete artifact
    /// that is now fully terminated.
    ///
    /// Returns the events in stream order. The buffer keeps any trailing
    /// partial artifact for the next call.
    ///
    /// # Errors
    ///
    /// Returns [`SidecarError::Parse`] if a closing tag is found without a
    /// matching opening tag, or [`SidecarError::Attribute`] if an opening
    /// tag is missing the required `type` / `id` attributes.
    pub fn feed(&mut self, chunk: &str) -> SidecarResult<Vec<ParserEvent>> {
        self.buffer.push_str(chunk);
        self.drain()
    }

    /// Drain any artifacts still buffered, then emit [`ParserEvent::Eof`].
    ///
    /// # Errors
    ///
    /// Same as [`ArtifactParser::feed`]. Additionally, if the buffer ends
    /// with an unterminated `<artifact …>` opening tag (no closing tag seen
    /// before EOF), the partial fragment is **discarded** and a parse error
    /// is logged via `tracing::warn!` — but EOF is still emitted so the
    /// caller can finalize whatever it has.
    pub fn finish(&mut self) -> SidecarResult<Vec<ParserEvent>> {
        let mut events = self.drain()?;
        if !self.buffer.trim().is_empty() && self.open_re.is_match(&self.buffer) {
            tracing::warn!(
                buffered = self.buffer.len(),
                "discarding unterminated artifact fragment at EOF"
            );
        }
        self.buffer.clear();
        events.push(ParserEvent::Eof);
        Ok(events)
    }

    fn drain(&mut self) -> SidecarResult<Vec<ParserEvent>> {
        let mut events = Vec::new();
        let mut deferred_err: Option<SidecarError> = None;
        loop {
            let Some(open_m) = self.open_re.find(&self.buffer) else {
                // No opening tag in the buffer — keep everything for the
                // next chunk (it may be the start of a future tag).
                break;
            };
            let open_start = open_m.start();
            let open_end = open_m.end();

            // Look for the matching close tag *after* the opening one.
            let Some(close_rel) = self.buffer[open_end..].find(self.close_tag) else {
                // Open seen but no close yet — retain from the open tag
                // onward and wait for more input.
                self.buffer.drain(..open_start);
                break;
            };
            let close_start = open_end + close_rel;
            let close_end = close_start + self.close_tag.len();

            // Slice out the full open tag + payload + close tag.
            let open_tag = self.buffer[open_start..open_end].to_string();
            let payload = self.buffer[open_end..close_start].to_string();

            // Consume this artifact from the buffer regardless of whether
            // it parses — otherwise a malformed tag would jam the stream
            // and starve every artifact that follows it.
            self.buffer.drain(..close_end);

            // Parse attributes off the opening tag using quick-xml so that
            // escaped values (&quot;, &amp;) are correctly decoded.
            let attrs = match parse_open_attrs(&open_tag) {
                Ok(a) => a,
                Err(e) => {
                    if deferred_err.is_none() {
                        deferred_err = Some(e);
                    }
                    continue;
                },
            };
            let id = match attrs.get("id") {
                Some(v) => v.clone(),
                None => {
                    if deferred_err.is_none() {
                        deferred_err = Some(SidecarError::Attribute("missing required `id`".into()));
                    }
                    continue;
                },
            };
            let type_ = match attrs.get("type") {
                Some(v) => v.clone(),
                None => {
                    if deferred_err.is_none() {
                        deferred_err =
                            Some(SidecarError::Attribute("missing required `type`".into()));
                    }
                    continue;
                },
            };
            let title = attrs.get("title").cloned().unwrap_or_default();

            // Strip a single leading newline (cosmetic — most LLM agents
            // emit `<artifact ...>\n…\n</artifact>` for readability).
            let content = payload.strip_prefix('\n').unwrap_or(&payload).to_string();

            let mime = type_.clone();
            events.push(ParserEvent::Artifact(Artifact {
                id,
                type_,
                title,
                content,
                mime,
            }));
        }
        if let Some(err) = deferred_err {
            // We surface the *first* malformed-artifact error after emitting
            // every well-formed artifact that came earlier in the buffer.
            // If no good artifact preceded it, the error becomes hard so the
            // caller's `?` can surface it; otherwise it is folded into a
            // `Warning` event the pipeline lifts to `partial = true`.
            if events.is_empty() {
                return Err(err);
            }
            tracing::warn!(error = %err, "dropped malformed artifact, continuing with {} good event(s)", events.len());
            events.push(ParserEvent::Warning(err.to_string()));
        }
        Ok(events)
    }
}

/// Parse the attributes off a single `<artifact ...>` opening tag.
fn parse_open_attrs(open_tag: &str) -> SidecarResult<HashMap<String, String>> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    // quick-xml expects a balanced doc; wrap as empty-element so it accepts
    // the bare opening tag.
    let normalized = open_tag.trim_end_matches('>').to_string() + "/>";
    let mut reader = Reader::from_str(&normalized);
    reader.config_mut().trim_text(false);

    let mut buf = Vec::new();
    let mut out = HashMap::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Empty(e)) | Ok(Event::Start(e)) => {
                for attr in e.attributes().flatten() {
                    let key = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
                    let val = attr
                        .decode_and_unescape_value(reader.decoder())
                        .map_err(|e| SidecarError::Attribute(e.to_string()))?
                        .into_owned();
                    out.insert(key, val);
                }
                return Ok(out);
            },
            Ok(Event::Eof) => {
                return Err(SidecarError::Parse(
                    "no opening tag found in attribute scan".into(),
                ));
            },
            Ok(_) => continue,
            Err(e) => return Err(SidecarError::Parse(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn arts(events: Vec<ParserEvent>) -> Vec<Artifact> {
        events
            .into_iter()
            .filter_map(|e| if let ParserEvent::Artifact(a) = e { Some(a) } else { None })
            .collect()
    }

    #[test]
    fn parses_single_artifact_one_chunk() {
        let mut p = ArtifactParser::new();
        let evs = p
            .feed(r#"<artifact type="text/markdown" id="m1" title="Hello">## Hi</artifact>"#)
            .unwrap();
        let got = arts(evs);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].id, "m1");
        assert_eq!(got[0].type_, "text/markdown");
        assert_eq!(got[0].title, "Hello");
        assert_eq!(got[0].content, "## Hi");
    }

    #[test]
    fn parses_three_artifacts_streamed_across_chunks() {
        let mut p = ArtifactParser::new();
        let mut acc = Vec::new();
        for chunk in [
            r#"<artifact type="text/markdown" id="a" title="A">aa</art"#,
            "ifact><artifact type=\"text/html\" id=\"b\" title=\"B\"><p>b</p></artifact><arti",
            r#"fact type="text/markdown" id="c" title="C">cc</artifact>"#,
        ] {
            acc.extend(p.feed(chunk).unwrap());
        }
        acc.extend(p.finish().unwrap());
        let got = arts(acc);
        assert_eq!(got.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(), ["a", "b", "c"]);
        assert_eq!(got[0].content, "aa");
        assert_eq!(got[1].content, "<p>b</p>");
        assert_eq!(got[2].content, "cc");
    }

    #[test]
    fn missing_id_attribute_is_rejected() {
        let mut p = ArtifactParser::new();
        let err = p
            .feed(r#"<artifact type="text/markdown" title="oops">x</artifact>"#)
            .unwrap_err();
        assert!(matches!(err, SidecarError::Attribute(_)));
    }

    #[test]
    fn finish_emits_eof_even_with_empty_buffer() {
        let mut p = ArtifactParser::new();
        let evs = p.finish().unwrap();
        assert_eq!(evs, vec![ParserEvent::Eof]);
    }
}
