// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//
// Public model selector, decoupled from the optional `fastembed` dependency.
//
// This enum is always compiled (no feature gate, no target gate) so that
// callers can name a model, persist a choice in config, and pattern-match on
// it even in builds where the `embeddings` feature is off or the target is
// wasm32. The mapping to `fastembed::EmbeddingModel` lives behind the feature
// gate in `engine.rs`.

use core::fmt;

/// A local embedding model supported by aphrody.
///
/// The whitelist is deliberately small: two compact 384-dimension English
/// sentence-transformer models that download quickly and run on CPU. Both are
/// well-suited to seeding semantic memory/search without a GPU or an external
/// API. More variants can be added as needed, but every CLI surface that
/// stores vectors (`LanceDB` fixed-size-list columns) benefits from a stable,
/// small dimension default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum EmbeddingModelKind {
    /// `BAAI/bge-small-en-v1.5` — 384 dims. fastembed's own default; strong
    /// retrieval quality for its size. This is aphrody's default too.
    #[default]
    BgeSmallEnV15,
    /// `sentence-transformers/all-MiniLM-L6-v2` — 384 dims. The classic,
    /// ubiquitous compact sentence-transformer; slightly smaller download.
    AllMiniLmL6V2,
}

impl EmbeddingModelKind {
    /// Stable, machine-friendly identifier for config / CLI parsing.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::BgeSmallEnV15 => "bge-small-en-v1.5",
            Self::AllMiniLmL6V2 => "all-minilm-l6-v2",
        }
    }

    /// The embedding dimension this model produces.
    ///
    /// Hard-coded from the model cards so the dimension is known without
    /// loading the model (useful for provisioning a fixed-size vector column
    /// before the first download). The engine cross-checks this against the
    /// model's own metadata at load time.
    #[must_use]
    pub const fn dimension(self) -> usize {
        match self {
            // Both whitelisted models are 384-dimensional.
            Self::BgeSmallEnV15 | Self::AllMiniLmL6V2 => 384,
        }
    }

    /// All supported models, for enumeration in help text / CLIs.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[Self::BgeSmallEnV15, Self::AllMiniLmL6V2]
    }
}

impl fmt::Display for EmbeddingModelKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl core::str::FromStr for EmbeddingModelKind {
    type Err = UnknownModel;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Accept the canonical id plus a couple of common aliases so config /
        // CLI input is forgiving without being ambiguous.
        match s.trim().to_ascii_lowercase().as_str() {
            "bge-small-en-v1.5" | "bge-small" | "bge-small-en" | "default" => Ok(Self::BgeSmallEnV15),
            "all-minilm-l6-v2" | "minilm" | "all-minilm" | "minilm-l6" => Ok(Self::AllMiniLmL6V2),
            _ => Err(UnknownModel(s.to_owned())),
        }
    }
}

/// Error returned when a model identifier is not recognised.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("unknown embedding model {0:?}: expected one of bge-small-en-v1.5, all-minilm-l6-v2")]
pub struct UnknownModel(pub String);
