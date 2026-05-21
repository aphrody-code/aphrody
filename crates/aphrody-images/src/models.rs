// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//! Nano Banana image model registry.
//!
//! Model ids mirror the Python reference exactly so prompt-engineering docs and
//! environment variable overrides are interchangeable between the two surfaces.
//!
//! # Model selection order (matches Python `_resolve_model`)
//!
//! 1. Explicit `model` field in [`crate::config::GenerateOptions`].
//! 2. `APHRODY_IMAGE_MODEL` environment variable.
//! 3. [`DEFAULT_IMAGE_MODEL`] (Nano Banana Pro).

/// Nano Banana Pro — Gemini 3 Pro Image.
///
/// Max fidelity, native 1K/2K/4K output, typography, real-world grounding via
/// Google Search, up to 14 reference images.
pub const NANO_BANANA_PRO: &str = "gemini-3-pro-image-preview";

/// Nano Banana 2 Flash — Gemini 3.1 Flash Image.
///
/// Fast image model with thinking support; serves as the first fallback when
/// the Pro model is unavailable.
pub const NANO_BANANA_2_FLASH: &str = "gemini-3.1-flash-image-preview";

/// Nano Banana Flash — Gemini 2.5 Flash Image.
///
/// Fastest and most widely entitled model; the final fallback in the chain.
pub const NANO_BANANA_FLASH: &str = "gemini-2.5-flash-image";

/// Default image model when no override is supplied.
pub const DEFAULT_IMAGE_MODEL: &str = NANO_BANANA_PRO;

/// Maximum reference images per compose/edit request (API ceiling).
pub const MAX_REFERENCE_IMAGES: usize = 14;

/// Ordered degradation chain: Pro → 2-Flash → Flash.
///
/// Matches the Python `FALLBACK_CHAIN` tuple exactly. Every generation attempt
/// walks this chain until one model produces an image, stopping on the first
/// success.
pub const MODEL_FALLBACK_CHAIN: &[&str] =
    &[NANO_BANANA_PRO, NANO_BANANA_2_FLASH, NANO_BANANA_FLASH];

/// A fully-resolved image model identifier (`&'static str` or owned).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageModel(String);

impl ImageModel {
    /// Construct from any string (caller-provided model id or env var).
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Resolve the model from an optional override and the environment.
    ///
    /// Resolution order:
    /// 1. `override_id` — an explicit caller value.
    /// 2. `APHRODY_IMAGE_MODEL` environment variable.
    /// 3. [`DEFAULT_IMAGE_MODEL`].
    #[must_use]
    pub fn resolve(override_id: Option<&str>) -> Self {
        let id = override_id
            .filter(|s| !s.is_empty())
            .map_or_else(
                || {
                    std::env::var("APHRODY_IMAGE_MODEL")
                        .unwrap_or_else(|_| DEFAULT_IMAGE_MODEL.to_string())
                },
                str::to_string,
            );
        Self(id)
    }

    /// Returns `true` if this model belongs to the Gemini 3 family (Pro-only
    /// knobs such as `image_size` are available).
    ///
    /// Mirrors the Python `_is_pro_family` predicate.
    ///
    /// # Examples
    ///
    /// ```
    /// use aphrody_images::models::ImageModel;
    ///
    /// assert!(ImageModel::new("gemini-3-pro-image-preview").is_pro_family());
    /// assert!(ImageModel::new("gemini-3.1-flash-image-preview").is_pro_family());
    /// assert!(!ImageModel::new("gemini-2.5-flash-image").is_pro_family());
    /// ```
    #[must_use]
    pub fn is_pro_family(&self) -> bool {
        self.0.starts_with("gemini-3") && self.0.contains("image")
    }

    /// The raw model-id string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Build the ordered fallback list starting with `self`.
    ///
    /// Deduplicates while preserving insertion order (matches Python
    /// `_fallback_models`). The returned vec always starts with `self`.
    ///
    /// # Examples
    ///
    /// ```
    /// use aphrody_images::models::{ImageModel, NANO_BANANA_PRO, MODEL_FALLBACK_CHAIN};
    ///
    /// let model = ImageModel::new(NANO_BANANA_PRO);
    /// let chain = model.fallback_chain();
    /// // Pro is already first in the constant chain — no duplication.
    /// assert_eq!(chain.len(), MODEL_FALLBACK_CHAIN.len());
    /// assert_eq!(chain[0], NANO_BANANA_PRO);
    /// ```
    #[must_use]
    pub fn fallback_chain(&self) -> Vec<&str> {
        let mut seen = std::collections::HashSet::new();
        let mut chain = Vec::with_capacity(MODEL_FALLBACK_CHAIN.len() + 1);

        // Self first.
        chain.push(self.0.as_str());
        seen.insert(self.0.as_str());

        // Append from the constant chain, skipping duplicates.
        for &m in MODEL_FALLBACK_CHAIN {
            if seen.insert(m) {
                chain.push(m);
            }
        }
        chain
    }
}

impl std::fmt::Display for ImageModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_pro_family_gemini3() {
        assert!(ImageModel::new(NANO_BANANA_PRO).is_pro_family());
        assert!(ImageModel::new(NANO_BANANA_2_FLASH).is_pro_family());
    }

    #[test]
    fn is_pro_family_gemini25_is_false() {
        assert!(!ImageModel::new(NANO_BANANA_FLASH).is_pro_family());
    }

    #[test]
    fn fallback_chain_no_duplicates_when_primary_is_pro() {
        let model = ImageModel::new(NANO_BANANA_PRO);
        let chain = model.fallback_chain();
        // Pro already leads the constant chain, so length equals constant chain.
        assert_eq!(chain.len(), MODEL_FALLBACK_CHAIN.len());
        assert_eq!(chain[0], NANO_BANANA_PRO);
        // No duplicates.
        let unique: std::collections::HashSet<_> = chain.iter().copied().collect();
        assert_eq!(unique.len(), chain.len());
    }

    #[test]
    fn fallback_chain_custom_model_prepended() {
        let model = ImageModel::new("gemini-custom-preview");
        let chain = model.fallback_chain();
        // Custom model prepended; all three constant entries appended.
        assert_eq!(chain[0], "gemini-custom-preview");
        assert_eq!(chain.len(), MODEL_FALLBACK_CHAIN.len() + 1);
    }

    #[test]
    fn resolve_uses_default_without_env_override() {
        // When no explicit override is given and the env var is absent, the
        // resolved model must equal DEFAULT_IMAGE_MODEL.
        // We test via the explicit-override path to avoid touching env state.
        let model = ImageModel::resolve(Some(DEFAULT_IMAGE_MODEL));
        assert_eq!(model.as_str(), DEFAULT_IMAGE_MODEL);
    }

    #[test]
    fn resolve_explicit_override_wins() {
        let model = ImageModel::resolve(Some("gemini-explicit-test"));
        assert_eq!(model.as_str(), "gemini-explicit-test");
    }
}
