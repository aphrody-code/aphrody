// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors

//! Local model lifecycle for aphrody: resolve, fetch, verify, inspect, evict.
//!
//! This is the foundation layer of aphrody's local-inference toolbox. It owns
//! the weights on disk so the task crates above it (OCR, visual transcription,
//! transcription, embeddings) can ask for a model by name and receive a path
//! to bytes that are present, complete and digest-checked.
//!
//! # What it does
//!
//! * **Reference** — [`ModelRef`] names exactly one artefact, as a pinned Hub
//!   file (`hf:owner/repo/file@rev`), a direct URL, or a path already on disk.
//! * **Catalog** — [`Catalog`] maps short ids (`whisper-base-en`,
//!   `florence2-base-ft`) onto commit-pinned artefacts with expected digests,
//!   so a caller never types a sha by hand.
//! * **Store** — [`ModelStore`] owns `~/.aphrody/models`: a JSON registry, a
//!   deterministic path layout, verification, reconciliation and LRU eviction.
//! * **Fetch** — [`Downloader`] streams weights with resume support and refuses
//!   to install bytes whose digest does not match.
//! * **Inspect** — [`inspect`] parses GGUF, whisper GGML, safetensors and ONNX
//!   headers by hand, so `aphrody model info` reports what a file actually is
//!   rather than what its extension claims.
//!
//! # Layout on disk
//!
//! ```text
//! ~/.aphrody/models/
//!   registry.json
//!   hf/<owner>/<repo>/<revision>/<file>
//!   url/<url-digest>/<basename>
//! ```
//!
//! The store shares the `~/.aphrody/models` root with `aphrody-embed`, which
//! parks its fastembed downloads under `embeddings/`.
//!
//! # Example
//!
//! ```no_run
//! use aphrody_models::{Catalog, Downloader, ModelStore, PullOptions};
//!
//! # async fn run() -> aphrody_models::Result<()> {
//! let store = ModelStore::open()?;
//! let entry = Catalog::builtin().get("whisper-base-en")?;
//! let downloader = Downloader::new()?;
//!
//! for file in &entry.files {
//!     let options = PullOptions {
//!         expected_sha256: file.sha256.clone(),
//!         catalog_id: Some(entry.id.clone()),
//!         ..PullOptions::default()
//!     };
//!     let outcome = downloader
//!         .pull(&store, &file.reference, &options, |p| {
//!             if let Some(pct) = p.fraction() {
//!                 eprintln!("{:.0}%", pct * 100.0);
//!             }
//!         })
//!         .await?;
//!     println!("{} -> {}", file.role, outcome.model().path.display());
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Targets
//!
//! [`id`], [`catalog`], [`inspect`] and [`digest`]'s in-memory helpers are pure
//! and build for `wasm32-unknown-unknown`. [`store`] and [`fetch`] need a
//! filesystem and a socket, so they are host-only.

#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![warn(missing_docs)]
// The pedantic group flags `format!("{}", x)` inside error strings and a few
// `as` casts in progress arithmetic that are deliberate and range-checked.
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::module_name_repetitions)]

/// Hardware probing (GPU / VRAM / CUDA) and catalog ranking: which model this
/// machine should run for a given task.
pub mod accel;
/// Curated catalog: short ids expanded to commit-pinned, digest-checked
/// artefacts, grouped by task and backend.
pub mod catalog;
/// Report rendering: one table model, five output formats (text, JSON,
/// Markdown, HTML, CSV).
pub mod render;
/// SHA-256 helpers: one-shot, streaming, whole-file, and digest normalisation.
pub mod digest;
/// The crate's error type.
pub mod error;
/// Model reference grammar and cache-path derivation.
pub mod id;
/// Format detection and hand-rolled header parsing for GGUF, whisper GGML,
/// safetensors and ONNX artefacts.
pub mod inspect;

/// Resumable, digest-checked weight downloads. Host-only.
#[cfg(not(target_arch = "wasm32"))]
pub mod fetch;
/// The on-disk store: layout, registry, verification, eviction. Host-only.
#[cfg(not(target_arch = "wasm32"))]
pub mod store;

pub use accel::{Accelerator, GpuInfo, HardwareProfile, Recommendation, rank_for};
pub use catalog::{Backend, Catalog, CatalogEntry, CatalogFile, ModelTask, Resolved, SpeedTier};
pub use render::{Format, Report};
pub use error::{ModelError, Result};
pub use id::{DEFAULT_REVISION, ModelRef};
pub use inspect::{ArtifactFormat, Details, Inspection, inspect_prefix};

#[cfg(not(target_arch = "wasm32"))]
pub use fetch::{Downloader, Progress, PullOptions, PullOutcome};
#[cfg(not(target_arch = "wasm32"))]
pub use store::{GcReport, InstalledModel, ModelStore, ReconcileReport, VerifyReport};

/// Fetch every artefact behind a spec, whether it is a catalog id or a raw
/// reference, and return one outcome per artefact.
///
/// This is the one-call entry point a CLI or job runner wants: it resolves the
/// spec, threads each file's expected digest through the download, and records
/// the catalog id on every resulting registry entry so provenance survives.
///
/// `on_progress` receives `(role, progress)` for each artefact, where `role` is
/// the catalog role (`weights`, `mmproj`, ...) or `"artifact"` for a direct
/// reference.
///
/// # Errors
///
/// Propagates resolution, transport, digest and filesystem failures. The first
/// failing artefact aborts the call; artefacts already fetched stay installed.
#[cfg(not(target_arch = "wasm32"))]
pub async fn pull_spec(
    store: &ModelStore,
    downloader: &Downloader,
    spec: &str,
    force: bool,
    mut on_progress: impl FnMut(&str, Progress),
) -> Result<Vec<PullOutcome>> {
    let catalog = Catalog::builtin();
    let resolved = catalog.resolve(spec)?;
    let catalog_id = resolved.catalog_id().map(ToOwned::to_owned);

    // Roles are carried alongside so progress reporting can name the piece
    // being transferred instead of echoing a URL.
    let roles: Vec<String> = match &resolved {
        Resolved::Catalog(entry) => entry.files.iter().map(|f| f.role.clone()).collect(),
        Resolved::Direct(_) => vec!["artifact".to_owned()],
    };

    let mut outcomes = Vec::new();
    for ((reference, expected_sha256), role) in resolved.artifacts().into_iter().zip(roles) {
        let options = PullOptions { expected_sha256, force, catalog_id: catalog_id.clone() };
        let outcome = downloader
            .pull(store, &reference, &options, |progress| on_progress(&role, progress))
            .await?;
        outcomes.push(outcome);
    }
    Ok(outcomes)
}

/// Human-readable byte size, binary units, two significant decimals.
///
/// Model artefacts span six orders of magnitude (a 185-byte generation config
/// next to a 574 MB checkpoint), so every surface that lists them needs this.
#[must_use]
pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 6] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    format!("{value:.2} {}", UNITS[unit])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_bytes_scales_through_the_units() {
        assert_eq!(human_bytes(0), "0 B");
        assert_eq!(human_bytes(1023), "1023 B");
        assert_eq!(human_bytes(1024), "1.00 KiB");
        assert_eq!(human_bytes(147_964_211), "141.11 MiB");
        assert_eq!(human_bytes(574_041_195), "547.45 MiB");
        assert_eq!(human_bytes(5 * 1024 * 1024 * 1024), "5.00 GiB");
        // Saturates at the largest known unit rather than overflowing it.
        assert!(human_bytes(u64::MAX).ends_with("EiB") || human_bytes(u64::MAX).ends_with("PiB"));
    }

    #[test]
    fn the_public_surface_is_reachable_through_the_root() {
        // A compile-level guard that the re-exports stay wired: every task
        // crate above this one imports through `aphrody_models::…`.
        let entry = Catalog::builtin().get("bge-small-en-v1.5").unwrap();
        assert_eq!(entry.task, ModelTask::TextEmbedding);
        assert_eq!(entry.backend, Backend::OnnxRuntime);

        let reference: ModelRef = "hf:a/b/c.gguf".parse().unwrap();
        assert_eq!(reference.basename(), "c.gguf");

        let inspection = inspect_prefix(b"PK\x03\x04", "m.bin");
        assert_eq!(inspection.format, ArtifactFormat::PyTorch);
    }
}
