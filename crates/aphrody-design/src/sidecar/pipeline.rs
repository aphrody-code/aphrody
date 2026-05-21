// SPDX-License-Identifier: Apache-2.0
//! End-to-end pipeline: stream-in → digest manifest out.
//!
//! `Pipeline::run` is the canonical entrypoint. It owns one
//! [`ArtifactParser`] and drives the full chain:
//!
//! 1. Read bytes from the input (chunks of up to 64 KiB).
//! 2. Decode as UTF-8 (lossy at the edges to survive split codepoints).
//! 3. Feed the parser, get back `ParserEvent::Artifact(_)` entries.
//! 4. Merge same-id chunks via [`merge_into`].
//! 5. Adapter-normalize each artifact.
//! 6. Validate the artifact + accumulate per-artifact digests.
//! 7. Finalize the manifest, run cross-row validation.
//!
//! Fallback policy: if step 3 or step 5 returns an error mid-stream, the
//! offending artifact is dropped, `partial = true` is set on the final
//! manifest, and the pipeline continues. Only an IO error from the input
//! source aborts the run.

use std::collections::HashMap;

use tokio::io::{AsyncRead, AsyncReadExt};

use crate::sidecar::adapters::{ArtifactFormat, adapter_for};
use crate::sidecar::digest::{ArtifactDigest, DigestManifest, digest_artifact, digest_manifest};
use crate::sidecar::error::SidecarResult;
use crate::sidecar::merge::merge_into;
use crate::sidecar::parser::{Artifact, ArtifactParser, ParserEvent};
use crate::sidecar::validate::{validate_artifact, validate_manifest};

/// What the pipeline gives back.
#[derive(Debug, Clone)]
pub struct PipelineOutcome {
    /// The deterministic manifest.
    pub manifest: DigestManifest,
    /// Per-artifact log of what happened (useful for the UI / telemetry).
    pub log: Vec<PipelineLogEntry>,
}

/// One structured log entry produced during a pipeline run.
#[derive(Debug, Clone)]
pub struct PipelineLogEntry {
    /// Artifact id, when known.
    pub id: Option<String>,
    /// Stage that produced the entry.
    pub stage: PipelineStage,
    /// Free-form message.
    pub message: String,
}

/// Pipeline stage tags for log entries.
#[derive(Debug, Clone, Copy)]
pub enum PipelineStage {
    Parse,
    Merge,
    Normalize,
    Validate,
    Digest,
}

/// Streaming pipeline driver.
pub struct Pipeline {
    parser: ArtifactParser,
    by_id: HashMap<String, Artifact>,
    order: Vec<String>,
    log: Vec<PipelineLogEntry>,
    degraded: bool,
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl Pipeline {
    /// Fresh pipeline with no buffered state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            parser: ArtifactParser::new(),
            by_id: HashMap::new(),
            order: Vec::new(),
            log: Vec::new(),
            degraded: false,
        }
    }

    /// Drain everything from `stream` and return the manifest.
    ///
    /// # Errors
    ///
    /// [`SidecarError::Io`] if reading from `stream` fails. All other
    /// errors degrade the pipeline to partial mode.
    pub async fn run<R: AsyncRead + Unpin>(mut self, mut stream: R) -> SidecarResult<PipelineOutcome> {
        let mut buf = vec![0u8; 64 * 1024];
        let mut text_carry = String::new();
        loop {
            let n = stream.read(&mut buf).await?;
            if n == 0 {
                break;
            }
            text_carry.push_str(&String::from_utf8_lossy(&buf[..n]));
            // We feed all of carry, then reset — quick-xml's attribute
            // decoder will see the boundary intact since the regex finds
            // full open/close tags only.
            self.feed_text(&text_carry);
            text_carry.clear();
        }
        // Finalize the parser (drops unterminated fragments, emits Eof).
        match self.parser.finish() {
            Ok(events) => self.absorb_events(events),
            Err(e) => self.note_parse(None, e.to_string()),
        }
        Ok(self.finalize())
    }

    /// Synchronous fast path for callers that already have the whole stream
    /// in memory (used by the `parse` CLI subcommand on small inputs).
    ///
    /// # Errors
    ///
    /// Returns [`SidecarError::Validation`] if the cross-row manifest checks
    /// fail; never returns [`SidecarError::Io`].
    pub fn run_sync(mut self, text: &str) -> SidecarResult<PipelineOutcome> {
        self.feed_text(text);
        match self.parser.finish() {
            Ok(events) => self.absorb_events(events),
            Err(e) => self.note_parse(None, e.to_string()),
        }
        Ok(self.finalize())
    }

    fn feed_text(&mut self, text: &str) {
        match self.parser.feed(text) {
            Ok(events) => self.absorb_events(events),
            Err(e) => {
                self.note_parse(None, e.to_string());
            },
        }
    }

    fn absorb_events(&mut self, events: Vec<ParserEvent>) {
        for ev in events {
            match ev {
                ParserEvent::Artifact(art) => {
                    let id = art.id.clone();
                    if let Some(existing) = self.by_id.get_mut(&id) {
                        if let Err(e) = merge_into(existing, &art) {
                            self.note(Some(id.clone()), PipelineStage::Merge, e.to_string());
                        }
                    } else {
                        self.order.push(id.clone());
                        self.by_id.insert(id, art);
                    }
                },
                ParserEvent::Warning(msg) => {
                    self.degraded = true;
                    self.note(None, PipelineStage::Parse, msg);
                },
                ParserEvent::Eof => {},
            }
        }
    }

    fn finalize(mut self) -> PipelineOutcome {
        let mut rows: Vec<ArtifactDigest> = Vec::new();
        // Preserve insertion order while we walk; the digest layer will
        // sort by id for deterministic hashing.
        for id in std::mem::take(&mut self.order) {
            let Some(artifact) = self.by_id.remove(&id) else { continue };
            let Some(fmt) = ArtifactFormat::from_mime(&artifact.type_) else {
                self.degraded = true;
                self.note(
                    Some(id.clone()),
                    PipelineStage::Validate,
                    format!("dropping artifact: unsupported type {}", artifact.type_),
                );
                continue;
            };
            let adapter = adapter_for(fmt);
            let normalized = match adapter.normalize(artifact.content.as_bytes()) {
                Ok(n) => n,
                Err(e) => {
                    self.degraded = true;
                    self.note(Some(id.clone()), PipelineStage::Normalize, e.to_string());
                    continue;
                },
            };
            if let Err(e) = validate_artifact(&artifact, &normalized) {
                self.degraded = true;
                self.note(Some(id.clone()), PipelineStage::Validate, e.to_string());
                continue;
            }
            rows.push(digest_artifact(&artifact, &normalized));
            self.note(Some(id), PipelineStage::Digest, "ok".into());
        }
        let manifest = digest_manifest(rows, self.degraded);
        if let Err(e) = validate_manifest(&manifest) {
            self.degraded = true;
            self.note(None, PipelineStage::Validate, e.to_string());
        }
        let mut final_manifest = manifest;
        final_manifest.partial = self.degraded;
        PipelineOutcome { manifest: final_manifest, log: self.log }
    }

    fn note(&mut self, id: Option<String>, stage: PipelineStage, message: String) {
        self.log.push(PipelineLogEntry { id, stage, message });
    }

    fn note_parse(&mut self, id: Option<String>, message: String) {
        self.degraded = true;
        self.note(id, PipelineStage::Parse, message);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_happy_path_three_artifacts() {
        let stream = concat!(
            r#"<artifact type="text/markdown" id="m" title="MD">## Hi</artifact>"#,
            r#"<artifact type="text/html" id="h" title="HTML"><p>x</p></artifact>"#,
            r#"<artifact type="text/markdown" id="m2" title="MD2">**bold**</artifact>"#,
        );
        let out = Pipeline::new().run_sync(stream).unwrap();
        assert_eq!(out.manifest.artifacts.len(), 3);
        assert!(!out.manifest.partial);
        let ids: Vec<&str> = out.manifest.artifacts.iter().map(|a| a.id.as_str()).collect();
        // Sorted by id ⇒ ["h", "m", "m2"].
        assert_eq!(ids, ["h", "m", "m2"]);
    }

    #[test]
    fn pipeline_falls_back_on_unknown_type() {
        let stream = concat!(
            r#"<artifact type="text/markdown" id="ok" title="OK">good</artifact>"#,
            r#"<artifact type="application/x-bogus" id="bad" title="Bad">junk</artifact>"#,
        );
        let out = Pipeline::new().run_sync(stream).unwrap();
        assert_eq!(out.manifest.artifacts.len(), 1);
        assert_eq!(out.manifest.artifacts[0].id, "ok");
        assert!(out.manifest.partial, "should be marked degraded");
    }

    #[test]
    fn pipeline_falls_back_on_parse_error_but_keeps_prior_artifacts() {
        // Inject a syntactically valid open tag missing required `id`.
        let stream = concat!(
            r#"<artifact type="text/markdown" id="good" title="G">ok</artifact>"#,
            r#"<artifact type="text/markdown" title="no-id">junk</artifact>"#,
        );
        let out = Pipeline::new().run_sync(stream).unwrap();
        assert_eq!(out.manifest.artifacts.len(), 1);
        assert_eq!(out.manifest.artifacts[0].id, "good");
        assert!(out.manifest.partial);
    }
}
