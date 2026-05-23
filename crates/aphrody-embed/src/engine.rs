// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//
// The real embeddings engine, backed by `fastembed` (ONNX Runtime).
//
// This module is compiled ONLY when the `embeddings` feature is enabled AND
// the target is not wasm32 (see the cfg gate at the `mod engine` declaration
// in lib.rs). It contains the genuine business logic: loading an ONNX model
// (downloaded from HuggingFace into the aphrody model cache on first use) and
// running CPU inference to turn text into dense float vectors.

use std::path::PathBuf;

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

use crate::cache::model_cache_dir;
use crate::error::{EmbedError, Result};
use crate::model::EmbeddingModelKind;

/// Map aphrody's public model selector onto fastembed's enum.
const fn to_fastembed(kind: EmbeddingModelKind) -> EmbeddingModel {
    match kind {
        EmbeddingModelKind::BgeSmallEnV15 => EmbeddingModel::BGESmallENV15,
        EmbeddingModelKind::AllMiniLmL6V2 => EmbeddingModel::AllMiniLML6V2,
    }
}

/// A loaded, ready-to-use local text embedder.
///
/// Wraps a `fastembed::TextEmbedding` (an ONNX session + tokenizer). Construct
/// it with [`Embedder::new`] (default cache dir) or [`Embedder::with_cache_dir`].
/// The first construction for a given model downloads the weights from
/// `HuggingFace` into the cache directory; subsequent constructions reuse them.
///
/// Inference requires `&mut self` because the underlying ONNX session is
/// mutated during a run (fastembed's `embed` takes `&mut self`).
pub struct Embedder {
    model: TextEmbedding,
    kind: EmbeddingModelKind,
    dimension: usize,
}

impl core::fmt::Debug for Embedder {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        // `TextEmbedding` is not `Debug`; expose the useful metadata only.
        f.debug_struct("Embedder")
            .field("model", &self.kind)
            .field("dimension", &self.dimension)
            .finish()
    }
}

impl Embedder {
    /// Load `model` using the default aphrody cache (`~/.aphrody/models/embeddings`).
    ///
    /// # Errors
    /// Returns [`EmbedError::CacheDir`] / [`EmbedError::NoHome`] if the cache
    /// path cannot be resolved or created, or [`EmbedError::ModelInit`] if the
    /// model download / ONNX session build / tokenizer load fails (e.g. no
    /// network on first run).
    pub fn new(model: EmbeddingModelKind) -> Result<Self> {
        Self::build(model, None, false)
    }

    /// Load `model`, caching weights under `cache_dir` instead of the default.
    ///
    /// # Errors
    /// See [`Embedder::new`].
    pub fn with_cache_dir(model: EmbeddingModelKind, cache_dir: PathBuf) -> Result<Self> {
        Self::build(model, Some(cache_dir), false)
    }

    /// Load `model`, showing a download progress bar on stderr (first run only).
    ///
    /// # Errors
    /// See [`Embedder::new`].
    pub fn new_with_progress(model: EmbeddingModelKind) -> Result<Self> {
        Self::build(model, None, true)
    }

    fn build(
        kind: EmbeddingModelKind,
        cache_override: Option<PathBuf>,
        show_progress: bool,
    ) -> Result<Self> {
        let cache_dir = model_cache_dir(cache_override)?;
        tracing::debug!(
            model = %kind,
            cache_dir = %cache_dir.display(),
            "loading local embedding model"
        );

        let fe_model = to_fastembed(kind);

        // Cross-check the dimension the model advertises against our static
        // table so a future fastembed change to a model's dim surfaces loudly.
        // Query metadata BEFORE constructing InitOptions, which consumes the
        // (non-Copy) EmbeddingModel by value.
        let info_dim = TextEmbedding::get_model_info(&fe_model)
            .map(|info| info.dim)
            .map_err(|e| EmbedError::ModelInit(e.to_string()))?;
        let expected = kind.dimension();
        if info_dim != expected {
            return Err(EmbedError::DimensionMismatch {
                expected,
                actual: info_dim,
            });
        }

        let options = InitOptions::new(fe_model)
            .with_cache_dir(cache_dir)
            .with_show_download_progress(show_progress);
        let model =
            TextEmbedding::try_new(options).map_err(|e| EmbedError::ModelInit(e.to_string()))?;

        Ok(Self {
            model,
            kind,
            dimension: info_dim,
        })
    }

    /// The embedding dimension produced by this model (e.g. `384`).
    #[must_use]
    pub const fn dimension(&self) -> usize {
        self.dimension
    }

    /// The model backing this embedder.
    #[must_use]
    pub const fn model(&self) -> EmbeddingModelKind {
        self.kind
    }

    /// Embed a batch of texts into dense float vectors.
    ///
    /// Returns one `Vec<f32>` per input, in input order, each of length
    /// [`Embedder::dimension`]. `batch_size` controls the ONNX micro-batch
    /// (None => fastembed's default of 256); it does not change the number of
    /// returned vectors.
    ///
    /// # Errors
    /// Returns [`EmbedError::Inference`] if the ONNX run fails, or
    /// [`EmbedError::DimensionMismatch`] if any returned vector's length does
    /// not match the model dimension (defensive; protects fixed-size vector
    /// columns downstream).
    pub fn embed_batch<S>(&mut self, texts: &[S], batch_size: Option<usize>) -> Result<Vec<Vec<f32>>>
    where
        S: AsRef<str> + Send + Sync,
    {
        if texts.is_empty() {
            return Ok(Vec::new());
        }
        let vectors = self
            .model
            .embed(texts, batch_size)
            .map_err(|e| EmbedError::Inference(e.to_string()))?;

        // Guard the fixed dimension invariant for every vector in the batch.
        if let Some(bad) = vectors.iter().find(|v| v.len() != self.dimension) {
            return Err(EmbedError::DimensionMismatch {
                expected: self.dimension,
                actual: bad.len(),
            });
        }
        Ok(vectors)
    }

    /// Embed string slices, the common case. Convenience over [`Embedder::embed_batch`].
    ///
    /// # Errors
    /// See [`Embedder::embed_batch`].
    pub fn embed_texts(&mut self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        self.embed_batch(texts, None)
    }

    /// Embed a single text into one dense float vector.
    ///
    /// # Errors
    /// See [`Embedder::embed_batch`].
    pub fn embed_one(&mut self, text: &str) -> Result<Vec<f32>> {
        let mut out = self.embed_batch(&[text], None)?;
        out.pop().ok_or(EmbedError::Inference(
            "model returned no vector for a single input".into(),
        ))
    }
}
