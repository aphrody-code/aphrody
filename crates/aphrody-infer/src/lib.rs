// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors

//! Local ONNX inference for aphrody: runtime discovery, execution-provider
//! selection, and session loading driven by the `aphrody-models` catalog.
//!
//! This is the layer between "the weights are on disk" (`aphrody-models`) and
//! "here is the text in that image" (the task crates above). It answers two
//! questions: **which shared library** implements ONNX Runtime on this machine,
//! and **which execution provider** should run a given model.
//!
//! # The one thing to know
//!
//! [`LoadedModel::provider`] reports the accelerator that actually built the
//! session, and [`LoadedModel::fallbacks`] lists what was tried and why it was
//! rejected. Silent CPU fallback is how a "GPU pipeline" ends up ten times
//! slower than expected with nobody noticing, so that information is part of
//! the return value rather than a log line.
//!
//! # Feature gate
//!
//! The ONNX backend is behind the `onnx` feature: it links ONNX Runtime, which
//! is a 260 MB native dependency and is not wasm-compatible. Without the
//! feature the crate still compiles and its entry points return
//! [`InferError::BackendUnavailable`] — never a panic, never a stub that
//! pretends to work.
//!
//! # Example
//!
//! ```no_run
//! use aphrody_infer::{SessionConfig, load_catalog_role};
//! use aphrody_models::accel;
//!
//! # fn main() -> aphrody_infer::Result<()> {
//! let profile = accel::probe();
//! let config = SessionConfig::from_profile(&profile);
//! let model = load_catalog_role("ppocr-v5-mobile", "detector", &config)?;
//! println!("running on {} ({:?})", model.provider, model.inputs());
//! # Ok(())
//! # }
//! ```

#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![warn(missing_docs)]
#![allow(clippy::module_name_repetitions)]

/// The crate's error type.
pub mod error;
/// llama.cpp backend discovery: locating the upstream release binaries that
/// run the catalog GGUF entries.
pub mod llama;
/// Locating the ONNX Runtime shared library.
pub mod runtime;

/// Session loading with execution-provider fallback. Requires the `onnx`
/// feature.
#[cfg(all(feature = "onnx", not(target_arch = "wasm32")))]
pub mod session;

pub use error::{InferError, Result};
pub use llama::{LlamaSource, LlamaTool};
pub use runtime::{RuntimeSource, discover};

#[cfg(all(feature = "onnx", not(target_arch = "wasm32")))]
pub use session::{LoadedModel, SessionConfig, init_runtime, load};

/// Resolve a catalog entry's artefact by role and load it as a session.
///
/// This is the entry point a task pipeline uses: it never touches paths or
/// revisions, only `("ppocr-v5-mobile", "detector")`. The artefact must
/// already be installed — pull it with `aphrody model pull <id>` first, so a
/// long download never happens implicitly inside an inference call.
///
/// # Errors
///
/// [`InferError::MissingRole`] when the entry has no such role,
/// [`InferError::Model`] when the artefact is not installed, and
/// [`InferError::SessionBuild`] when no execution provider could load it.
#[cfg(all(feature = "onnx", not(target_arch = "wasm32")))]
pub fn load_catalog_role(
    entry_id: &str,
    role: &str,
    config: &SessionConfig,
) -> Result<LoadedModel> {
    use aphrody_models::{Catalog, ModelStore};

    let entry = Catalog::builtin().get(entry_id)?;
    let file = entry.file(role).ok_or_else(|| InferError::MissingRole {
        entry: entry_id.to_owned(),
        role: role.to_owned(),
    })?;

    let store = ModelStore::open()?;
    let installed = store
        .get(&file.reference)?
        .ok_or_else(|| aphrody_models::ModelError::NotInstalled(file.reference.to_string()))?;

    // Stamp the artefact as used so the LRU eviction in `aphrody model gc`
    // reflects what the inference paths actually depend on.
    let _ = store.touch(&file.reference);

    session::load(&installed.path, config)
}

/// Stub for builds without the `onnx` feature.
///
/// # Errors
///
/// Always [`InferError::BackendUnavailable`].
#[cfg(not(all(feature = "onnx", not(target_arch = "wasm32"))))]
pub fn load_catalog_role(_entry_id: &str, _role: &str) -> Result<()> {
    Err(InferError::BackendUnavailable("load_catalog_role"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_discovery_is_reachable_without_the_backend() {
        // `runtime` is deliberately outside the feature gate: `aphrody model
        // accel` reports where the runtime would come from even in a build
        // that cannot load it.
        let source = discover();
        assert!(!source.label().is_empty());
    }

    #[cfg(not(all(feature = "onnx", not(target_arch = "wasm32"))))]
    #[test]
    fn without_the_feature_the_entry_point_says_so_instead_of_panicking() {
        let err = load_catalog_role("ppocr-v5-mobile", "detector").unwrap_err();
        assert!(matches!(err, InferError::BackendUnavailable(_)));
        assert!(err.to_string().contains("--features infer"), "{err}");
    }
}
