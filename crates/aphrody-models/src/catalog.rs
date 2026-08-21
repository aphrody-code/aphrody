// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//
// The curated catalog: short ids mapped onto pinned, digest-checked artefacts.
//
// A `ModelRef` can name anything on the Hub, which is the right primitive but
// the wrong ergonomics for a toolbox: nobody wants to type a commit sha to run
// OCR. The catalog closes that gap with stable ids (`whisper-base-en`,
// `florence2-base-ft`) that expand into one or more pinned artefacts plus the
// digest each must hash to.
//
// The data lives in `catalog.json` next to this file and is embedded at compile
// time, so a catalog lookup never touches the network or the filesystem and the
// crate stays usable on wasm32.
//
// Multi-file entries are the norm rather than the exception: an ONNX pipeline
// needs its tokenizer and preprocessor config, and a GGUF vision model needs
// its mmproj projector. Each file carries a `role` so a backend can pick the
// piece it wants without string-matching on file names.

use std::sync::OnceLock;

use crate::accel::Accelerator;
use crate::error::{ModelError, Result};
use crate::id::ModelRef;

/// The catalog document embedded at build time.
const CATALOG_JSON: &str = include_str!("../catalog.json");

/// What a model is for. This is the axis the CLI and the job scheduler route
/// on, so it is deliberately about the task, not the architecture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum ModelTask {
    /// Image or page to text.
    Ocr,
    /// Image to description: captioning, dense regions, visual question
    /// answering. The "read me this screenshot" surface.
    VisualTranscription,
    /// Audio to text.
    SpeechToText,
    /// Text to dense vector.
    TextEmbedding,
    /// Text to text.
    TextGeneration,
}

impl ModelTask {
    /// Stable machine-friendly name, matching the JSON spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ocr => "ocr",
            Self::VisualTranscription => "visual-transcription",
            Self::SpeechToText => "speech-to-text",
            Self::TextEmbedding => "text-embedding",
            Self::TextGeneration => "text-generation",
        }
    }

    /// Every task, in declaration order. Useful for CLI help and completions.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[
            Self::Ocr,
            Self::VisualTranscription,
            Self::SpeechToText,
            Self::TextEmbedding,
            Self::TextGeneration,
        ]
    }

    /// Parse the machine-friendly name.
    #[must_use]
    pub fn from_str_opt(raw: &str) -> Option<Self> {
        Self::all().iter().copied().find(|t| t.as_str() == raw)
    }
}

impl core::fmt::Display for ModelTask {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The local runtime that can execute an entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Backend {
    /// ONNX Runtime, via `ort` (what `aphrody-embed` already links).
    OnnxRuntime,
    /// A llama.cpp-family runtime consuming GGUF.
    LlamaCpp,
    /// whisper.cpp consuming the legacy GGML container.
    WhisperCpp,
}

impl Backend {
    /// Stable machine-friendly name, matching the JSON spelling.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::OnnxRuntime => "onnx-runtime",
            Self::LlamaCpp => "llama-cpp",
            Self::WhisperCpp => "whisper-cpp",
        }
    }
}

impl core::fmt::Display for Backend {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One artefact belonging to a catalog entry.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CatalogFile {
    /// What this file is to the backend: `weights`, `encoder`, `decoder`,
    /// `tokenizer`, `mmproj`, `config`, ...
    pub role: String,
    /// Pinned reference, revision included.
    #[serde(rename = "ref")]
    pub reference: ModelRef,
    /// Size in bytes at the pinned revision.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub bytes: Option<u64>,
    /// Expected SHA-256. Present for LFS objects (the weights); absent for
    /// small plain-git sidecars, which the Hub does not expose a digest for.
    #[serde(skip_serializing_if = "Option::is_none", default)]
    pub sha256: Option<String>,
}

/// Throughput tier within a task. Ranked, so a selector can order candidates
/// without hard-coding model names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum SpeedTier {
    /// Highest accuracy, lowest throughput. Per-document, not per-batch.
    Quality,
    /// The middle of the curve.
    Balanced,
    /// Highest throughput. What a mass-processing job wants.
    Fast,
}

impl SpeedTier {
    /// Stable machine-friendly name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Quality => "quality",
            Self::Balanced => "balanced",
            Self::Fast => "fast",
        }
    }
}

impl core::fmt::Display for SpeedTier {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A catalogued model: one id, one task, one or more artefacts.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CatalogEntry {
    /// Short stable id, e.g. `whisper-base-en`.
    pub id: String,
    /// Human title for listings.
    pub title: String,
    /// What the model is for.
    pub task: ModelTask,
    /// Runtime that can execute it.
    pub backend: Backend,
    /// One-paragraph description, including the practical trade-off.
    pub summary: String,
    /// Artefacts to pull, in the order a backend generally wants them.
    pub files: Vec<CatalogFile>,
    /// Execution providers this entry can run on.
    #[serde(default = "default_accel")]
    pub accel: Vec<Accelerator>,
    /// VRAM the weights need resident. `0` means it is comfortable on CPU.
    #[serde(default)]
    pub vram_min_bytes: u64,
    /// Throughput tier within the task.
    #[serde(default = "default_speed")]
    pub speed: SpeedTier,
}

fn default_accel() -> Vec<Accelerator> {
    vec![Accelerator::Cpu]
}

const fn default_speed() -> SpeedTier {
    SpeedTier::Balanced
}

impl CatalogEntry {
    /// Total download size, when every file declares one.
    #[must_use]
    pub fn total_bytes(&self) -> Option<u64> {
        self.files.iter().map(|f| f.bytes).try_fold(0_u64, |acc, b| Some(acc + b?))
    }

    /// The artefact filling a given role.
    #[must_use]
    pub fn file(&self, role: &str) -> Option<&CatalogFile> {
        self.files.iter().find(|f| f.role == role)
    }

    /// The primary artefact: the explicit `weights` role when present, else
    /// the first file listed.
    #[must_use]
    pub fn primary(&self) -> Option<&CatalogFile> {
        self.file("weights").or_else(|| self.files.first())
    }
}

/// The parsed catalog document.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Catalog {
    /// Schema version of `catalog.json`.
    pub version: u32,
    /// Provenance note carried in the JSON; not used programmatically.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub comment: Vec<String>,
    /// Every catalogued model.
    pub entries: Vec<CatalogEntry>,
}

impl Catalog {
    /// The built-in catalog, parsed once per process.
    ///
    /// # Panics
    ///
    /// Panics if `catalog.json` is malformed. That file is embedded at compile
    /// time and covered by a test in this module, so a panic here means the
    /// build shipped a broken artefact — failing loudly is correct.
    #[must_use]
    pub fn builtin() -> &'static Self {
        static BUILTIN: OnceLock<Catalog> = OnceLock::new();
        BUILTIN.get_or_init(|| {
            serde_json::from_str(CATALOG_JSON).expect("embedded catalog.json is malformed")
        })
    }

    /// Look up an entry by id.
    ///
    /// # Errors
    ///
    /// [`ModelError::UnknownCatalogId`] when no entry carries that id.
    pub fn get(&self, id: &str) -> Result<&CatalogEntry> {
        self.entries
            .iter()
            .find(|e| e.id == id)
            .ok_or_else(|| ModelError::UnknownCatalogId(id.to_owned()))
    }

    /// Every entry serving a given task.
    #[must_use]
    pub fn by_task(&self, task: ModelTask) -> Vec<&CatalogEntry> {
        self.entries.iter().filter(|e| e.task == task).collect()
    }

    /// Every entry executable by a given backend.
    #[must_use]
    pub fn by_backend(&self, backend: Backend) -> Vec<&CatalogEntry> {
        self.entries.iter().filter(|e| e.backend == backend).collect()
    }

    /// All ids, in catalog order.
    #[must_use]
    pub fn ids(&self) -> Vec<&str> {
        self.entries.iter().map(|e| e.id.as_str()).collect()
    }

    /// Resolve a user-supplied string to a set of artefacts to pull.
    ///
    /// A catalog id expands to that entry's files; anything else is parsed as
    /// a raw [`ModelRef`] and yields a single unpinned artefact. This is the
    /// one function a CLI needs: it makes `aphrody model pull whisper-base-en`
    /// and `aphrody model pull hf:owner/repo/file.gguf` the same code path.
    ///
    /// # Errors
    ///
    /// [`ModelError::BadRef`] when the string is neither a catalog id nor a
    /// parseable reference.
    pub fn resolve(&self, spec: &str) -> Result<Resolved<'_>> {
        if let Ok(entry) = self.get(spec) {
            return Ok(Resolved::Catalog(entry));
        }
        let reference = ModelRef::parse(spec)?;
        Ok(Resolved::Direct(reference))
    }
}

/// What [`Catalog::resolve`] produced.
#[derive(Debug, Clone, PartialEq)]
pub enum Resolved<'a> {
    /// A catalog id, expanded to its entry.
    Catalog(&'a CatalogEntry),
    /// A raw reference typed by the user.
    Direct(ModelRef),
}

impl Resolved<'_> {
    /// The artefacts to pull, each with its expected digest when known.
    #[must_use]
    pub fn artifacts(&self) -> Vec<(ModelRef, Option<String>)> {
        match self {
            Self::Catalog(entry) => {
                entry.files.iter().map(|f| (f.reference.clone(), f.sha256.clone())).collect()
            }
            Self::Direct(reference) => vec![(reference.clone(), None)],
        }
    }

    /// The catalog id, when this came from the catalog.
    #[must_use]
    pub fn catalog_id(&self) -> Option<&str> {
        match self {
            Self::Catalog(entry) => Some(entry.id.as_str()),
            Self::Direct(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_catalog_parses() {
        let catalog = Catalog::builtin();
        assert_eq!(catalog.version, 1);
        assert!(!catalog.entries.is_empty());
    }

    #[test]
    fn ids_are_unique() {
        let ids = Catalog::builtin().ids();
        let unique: std::collections::BTreeSet<_> = ids.iter().collect();
        assert_eq!(unique.len(), ids.len(), "duplicate catalog id in {ids:?}");
    }

    #[test]
    fn every_reference_is_pinned_to_an_immutable_revision() {
        for entry in &Catalog::builtin().entries {
            for file in &entry.files {
                let ModelRef::Hf { revision, .. } = &file.reference else {
                    panic!("{}: catalog entries must be Hub references", entry.id);
                };
                // A 40-char hex string is a git commit; a branch name would
                // let the bytes move out from under the recorded digest.
                assert_eq!(
                    revision.len(),
                    40,
                    "{}/{} is pinned to `{revision}`, not a commit sha",
                    entry.id,
                    file.role
                );
                assert!(
                    revision.chars().all(|c| c.is_ascii_hexdigit()),
                    "{}/{} revision `{revision}` is not hex",
                    entry.id,
                    file.role
                );
            }
        }
    }

    #[test]
    fn declared_digests_are_well_formed() {
        for entry in &Catalog::builtin().entries {
            for file in &entry.files {
                let Some(sha) = &file.sha256 else { continue };
                assert_eq!(sha.len(), 64, "{}/{}: {sha}", entry.id, file.role);
                assert!(
                    sha.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
                    "{}/{}: digest must be lower-case hex, got {sha}",
                    entry.id,
                    file.role
                );
            }
        }
    }

    #[test]
    fn every_weight_bearing_file_declares_a_digest() {
        for entry in &Catalog::builtin().entries {
            for file in &entry.files {
                // Sidecars are plain git objects with no Hub-exposed digest;
                // anything big enough to be LFS must be verifiable.
                let is_sidecar = matches!(
                    file.role.as_str(),
                    "tokenizer"
                        | "config"
                        | "preprocessor"
                        | "generation-config"
                        | "detector-config"
                        | "recognizer-config"
                );
                if !is_sidecar {
                    assert!(
                        file.sha256.is_some(),
                        "{}/{} carries weights but no digest",
                        entry.id,
                        file.role
                    );
                }
            }
        }
    }

    #[test]
    fn every_entry_exposes_a_primary_artifact_and_a_size() {
        for entry in &Catalog::builtin().entries {
            assert!(entry.primary().is_some(), "{} has no files", entry.id);
            assert!(entry.total_bytes().is_some(), "{} has a file without a size", entry.id);
            assert!(!entry.summary.is_empty(), "{} has no summary", entry.id);
        }
    }

    #[test]
    fn the_three_target_workloads_are_covered() {
        let catalog = Catalog::builtin();
        for task in [ModelTask::Ocr, ModelTask::VisualTranscription, ModelTask::SpeechToText] {
            assert!(!catalog.by_task(task).is_empty(), "no model catalogued for {task}");
        }
    }

    #[test]
    fn multi_file_entries_expose_their_roles() {
        let florence = Catalog::builtin().get("florence2-base-ft").unwrap();
        assert_eq!(florence.task, ModelTask::VisualTranscription);
        assert_eq!(florence.backend, Backend::OnnxRuntime);
        assert!(florence.file("vision-encoder").is_some());
        assert!(florence.file("tokenizer").is_some());
        assert!(florence.file("nonexistent-role").is_none());

        let smolvlm = Catalog::builtin().get("smolvlm-500m-q8").unwrap();
        assert_eq!(smolvlm.backend, Backend::LlamaCpp);
        // A vision GGUF is useless without its projector.
        assert!(smolvlm.file("mmproj").is_some());
        assert_eq!(smolvlm.primary().unwrap().role, "weights");
    }

    #[test]
    fn whisper_entries_target_the_whisper_backend() {
        for entry in Catalog::builtin().by_task(ModelTask::SpeechToText) {
            assert_eq!(entry.backend, Backend::WhisperCpp, "{}", entry.id);
        }
        assert_eq!(Catalog::builtin().by_backend(Backend::WhisperCpp).len(), 3);
    }

    #[test]
    fn unknown_id_is_reported() {
        let err = Catalog::builtin().get("no-such-model").unwrap_err();
        assert!(matches!(err, ModelError::UnknownCatalogId(_)));
        assert!(err.to_string().contains("no-such-model"));
    }

    #[test]
    fn resolve_prefers_catalog_ids_then_falls_back_to_references() {
        let catalog = Catalog::builtin();

        let resolved = catalog.resolve("whisper-base-en").unwrap();
        assert_eq!(resolved.catalog_id(), Some("whisper-base-en"));
        let artifacts = resolved.artifacts();
        assert_eq!(artifacts.len(), 1);
        assert!(artifacts[0].1.is_some(), "catalog pulls carry an expected digest");

        let direct = catalog.resolve("hf:owner/repo/weights.gguf").unwrap();
        assert_eq!(direct.catalog_id(), None);
        assert_eq!(direct.artifacts().len(), 1);
        assert_eq!(direct.artifacts()[0].1, None);
    }

    #[test]
    fn resolve_rejects_garbage() {
        assert!(Catalog::builtin().resolve("not a ref").is_err());
    }

    #[test]
    fn task_names_round_trip() {
        for task in ModelTask::all() {
            assert_eq!(ModelTask::from_str_opt(task.as_str()), Some(*task));
        }
        assert_eq!(ModelTask::from_str_opt("nope"), None);
    }

    #[test]
    fn catalog_urls_target_the_pinned_commit() {
        let entry = Catalog::builtin().get("whisper-base-en").unwrap();
        let url = entry.primary().unwrap().reference.download_url().unwrap();
        assert!(url.starts_with("https://huggingface.co/ggerganov/whisper.cpp/resolve/"), "{url}");
        assert!(url.ends_with("/ggml-base.en.bin?download=true"), "{url}");
    }
}
