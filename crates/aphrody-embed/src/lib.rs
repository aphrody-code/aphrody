// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//
// `aphrody-embed` — local, offline text embeddings for the aphrody CLI.
//
// # What this gives aphrody
//
// The agent can turn text into dense float vectors **without any external
// API** (no OpenAI, no Vertex, no network round-trip per call once the model
// is cached). This is the missing producer side of semantic memory/search:
// `aphrody-memory` already stores and queries vectors (HNSW brute-force +
// LanceDB fixed-size-list columns) but has no embedder of its own. Pipe an
// `Embedder` output straight into those stores.
//
// # Backend
//
// Inference runs on `fastembed` (Apache-2.0 OR MIT), which wraps the ONNX
// Runtime via `ort` and a HuggingFace tokenizer. The default model is
// `BAAI/bge-small-en-v1.5` (384 dims) — small, CPU-friendly, strong retrieval
// quality. `sentence-transformers/all-MiniLM-L6-v2` (also 384 dims) is the
// second whitelisted option. Model weights download from HuggingFace into
// `~/.aphrody/models/embeddings` on first use, then are reused offline.
//
// # Feature & target gating
//
//   * The real engine is behind the **opt-in `embeddings` feature** (off by
//     default). A default build of this crate — and of the `aphrody` CLI that
//     may depend on it — pulls **none** of `fastembed`/`ort`/`tokenizers`.
//   * The engine is additionally **host-only**: gated `cfg(not(target_arch =
//     "wasm32"))`. The ONNX Runtime backend cannot link on wasm32, so on that
//     target every entry point returns a typed [`EmbedError::Unsupported`]
//     rather than failing to compile or shipping an empty body. This lets the
//     crate be a dependency of wasm builds that simply never call it (or call
//     it and degrade to a remote provider).
//
// # Example (requires `--features embeddings`, host target, model download)
//
// ```no_run
// # #[cfg(all(feature = "embeddings", not(target_arch = "wasm32")))]
// # fn demo() -> aphrody_embed::Result<()> {
// use aphrody_embed::{Embedder, EmbeddingModelKind};
//
// let mut embedder = Embedder::new(EmbeddingModelKind::default())?;
// let vectors = embedder.embed_texts(&["passage: hello world", "query: hi"])?;
// assert_eq!(vectors.len(), 2);
// assert_eq!(vectors[0].len(), embedder.dimension()); // 384
// // Feed `vectors` into aphrody-memory's LanceDB / HNSW vector store.
// # Ok(())
// # }
// ```

#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

mod cache;
mod error;
mod model;

pub use cache::model_cache_dir;
pub use error::{EmbedError, Result};
pub use model::{EmbeddingModelKind, UnknownModel};

/// Whether this build can actually compute embeddings.
///
/// `true` only when compiled with the `embeddings` feature on a non-wasm32
/// target. Callers can check this before constructing an [`Embedder`] to pick
/// a remote fallback up front instead of catching [`EmbedError::Unsupported`].
#[must_use]
pub const fn is_available() -> bool {
    cfg!(all(feature = "embeddings", not(target_arch = "wasm32")))
}

// ---------------------------------------------------------------------------
// Real engine (host + feature on) vs. typed-unavailable fallback.
//
// `#![forbid(unsafe_code)]` above is relaxed for the engine's test module on
// the host (env mutation) via a localised allow there, not crate-wide.
// ---------------------------------------------------------------------------
#[cfg(all(feature = "embeddings", not(target_arch = "wasm32")))]
mod engine;

#[cfg(all(feature = "embeddings", not(target_arch = "wasm32")))]
pub use engine::Embedder;

// Fallback surface for builds without the engine (feature off, or wasm32).
//
// We expose an `Embedder` type with the SAME constructor signatures so callers
// compile identically across configurations; every constructor returns
// `EmbedError::Unsupported` with a precise reason. This is a real, typed
// behaviour — not an empty body or a panic.
#[cfg(not(all(feature = "embeddings", not(target_arch = "wasm32"))))]
mod unavailable {
    use std::path::PathBuf;

    use crate::error::{EmbedError, Result};
    use crate::model::EmbeddingModelKind;

    /// Placeholder embedder for builds where local embeddings are unavailable.
    ///
    /// Mirrors the real [`crate::Embedder`] API so downstream code is
    /// configuration-agnostic, but every constructor fails with a typed
    /// [`EmbedError::Unsupported`] naming the reason.
    #[derive(Debug)]
    #[non_exhaustive]
    pub struct Embedder;

    impl Embedder {
        #[cfg(target_arch = "wasm32")]
        const WHY: &'static str =
            "the ONNX Runtime backend (fastembed/ort) does not link on wasm32";
        #[cfg(not(target_arch = "wasm32"))]
        const WHY: &'static str =
            "this build was compiled without the `embeddings` feature (build aphrody-embed with --features embeddings)";

        /// Always returns [`EmbedError::Unsupported`] on this build/target.
        ///
        /// # Errors
        /// Always.
        pub fn new(_model: EmbeddingModelKind) -> Result<Self> {
            Err(EmbedError::Unsupported(Self::WHY))
        }

        /// Always returns [`EmbedError::Unsupported`] on this build/target.
        ///
        /// # Errors
        /// Always.
        pub fn with_cache_dir(_model: EmbeddingModelKind, _cache_dir: PathBuf) -> Result<Self> {
            Err(EmbedError::Unsupported(Self::WHY))
        }

        /// Always returns [`EmbedError::Unsupported`] on this build/target.
        ///
        /// # Errors
        /// Always.
        pub fn new_with_progress(_model: EmbeddingModelKind) -> Result<Self> {
            Err(EmbedError::Unsupported(Self::WHY))
        }
    }
}

#[cfg(not(all(feature = "embeddings", not(target_arch = "wasm32"))))]
pub use unavailable::Embedder;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_kinds_are_384_dimensional() {
        for kind in EmbeddingModelKind::all() {
            assert_eq!(kind.dimension(), 384, "{kind} should be 384-d");
        }
    }

    #[test]
    fn model_kind_round_trips_through_str() {
        for kind in EmbeddingModelKind::all() {
            let parsed: EmbeddingModelKind = kind.as_str().parse().expect("parse canonical id");
            assert_eq!(parsed, *kind);
        }
        // Aliases resolve.
        assert_eq!(
            "default".parse::<EmbeddingModelKind>().unwrap(),
            EmbeddingModelKind::BgeSmallEnV15
        );
        assert_eq!(
            "minilm".parse::<EmbeddingModelKind>().unwrap(),
            EmbeddingModelKind::AllMiniLmL6V2
        );
        assert!("does-not-exist".parse::<EmbeddingModelKind>().is_err());
    }

    #[test]
    fn default_model_is_bge_small() {
        assert_eq!(EmbeddingModelKind::default(), EmbeddingModelKind::BgeSmallEnV15);
    }

    #[test]
    fn availability_matches_build_config() {
        let expected = cfg!(all(feature = "embeddings", not(target_arch = "wasm32")));
        assert_eq!(is_available(), expected);
    }

    // When the engine is NOT compiled in (feature off, or wasm32), the
    // fallback Embedder must fail with a typed Unsupported error — never panic,
    // never succeed.
    #[cfg(not(all(feature = "embeddings", not(target_arch = "wasm32"))))]
    #[test]
    fn unavailable_embedder_returns_typed_error() {
        let err = Embedder::new(EmbeddingModelKind::default()).unwrap_err();
        assert!(matches!(err, EmbedError::Unsupported(_)), "got {err:?}");
    }
}
