// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//! Generation configuration — validated aspect ratios, image sizes, and the
//! full option set accepted by [`crate::generate::generate_single`] and
//! [`crate::generate::generate_batch`].

use crate::error::{ImageError, Result};
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// AspectRatio
// ---------------------------------------------------------------------------

/// The ten aspect ratios accepted by Nano Banana Pro's `ImageConfig`.
///
/// Matches the Python `ASPECT_RATIOS` tuple exactly.
///
/// # Examples
///
/// ```
/// use aphrody_images::config::AspectRatio;
///
/// let ar = AspectRatio::parse("16:9").unwrap();
/// assert_eq!(ar.as_str(), "16:9");
///
/// assert!(AspectRatio::parse("7:3").is_err());
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AspectRatio(String);

impl AspectRatio {
    /// All valid aspect ratio tokens (matches the Python constant).
    pub const VALID: &'static [&'static str] =
        &["1:1", "2:3", "3:2", "3:4", "4:3", "4:5", "5:4", "9:16", "16:9", "21:9"];

    /// Parse and validate an aspect ratio string.
    ///
    /// # Errors
    ///
    /// [`ImageError::UnsupportedAspectRatio`] when the string is not one of
    /// the ten accepted ratios.
    pub fn parse(s: &str) -> Result<Self> {
        if Self::VALID.contains(&s) {
            Ok(Self(s.to_string()))
        } else {
            Err(ImageError::UnsupportedAspectRatio(s.to_string()))
        }
    }

    /// The raw aspect-ratio string (`"16:9"`, etc.).
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for AspectRatio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

// ---------------------------------------------------------------------------
// ImageSize
// ---------------------------------------------------------------------------

/// Output resolution tier for Nano Banana Pro (Pro-family models only).
///
/// Matches the Python `IMAGE_SIZES` constant. Flash-family models ignore this
/// field; the client drops it automatically when the resolved model is not in
/// the Gemini-3 Pro family.
///
/// # Examples
///
/// ```
/// use aphrody_images::config::ImageSize;
///
/// let sz = ImageSize::parse("4K").unwrap();
/// assert_eq!(sz.as_str(), "4K");
///
/// assert!(ImageSize::parse("8K").is_err());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageSize {
    /// Native 1K output.
    K1,
    /// Native 2K output.
    K2,
    /// Native 4K output.
    K4,
}

impl ImageSize {
    /// Parse from a `"1K"` / `"2K"` / `"4K"` string.
    ///
    /// # Errors
    ///
    /// [`ImageError::UnsupportedImageSize`] for any other value.
    pub fn parse(s: &str) -> Result<Self> {
        match s {
            "1K" | "1k" => Ok(Self::K1),
            "2K" | "2k" => Ok(Self::K2),
            "4K" | "4k" => Ok(Self::K4),
            _ => Err(ImageError::UnsupportedImageSize(s.to_string())),
        }
    }

    /// The canonical string representation used in the API payload.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::K1 => "1K",
            Self::K2 => "2K",
            Self::K4 => "4K",
        }
    }
}

impl std::fmt::Display for ImageSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

// ---------------------------------------------------------------------------
// GenerateOptions
// ---------------------------------------------------------------------------

/// Full option set for a Nano Banana image generation request.
///
/// Build via [`GenerateOptions::builder`] or construct directly; all fields
/// are optional with sane defaults.
///
/// # Examples
///
/// ```
/// use aphrody_images::config::{AspectRatio, GenerateOptions, ImageSize};
///
/// let opts = GenerateOptions::builder()
///     .aspect_ratio(AspectRatio::parse("16:9").unwrap())
///     .image_size(ImageSize::K4)
///     .negative_prompt("blurry, watermark")
///     .grounding(true)
///     .build();
///
/// assert_eq!(opts.aspect_ratio.as_ref().unwrap().as_str(), "16:9");
/// assert!(opts.grounding);
/// ```
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GenerateOptions {
    /// Explicit model id override. `None` → env var → default.
    pub model: Option<String>,

    /// Output aspect ratio (validated at parse time).
    pub aspect_ratio: Option<AspectRatio>,

    /// Output resolution tier (Pro-family models only).
    pub image_size: Option<ImageSize>,

    /// Negative-prompt constraints appended to the main prompt (matches Python
    /// `negative_prompt` → `"{prompt}\n\nConstraints (avoid): {negative_prompt}"`).
    pub negative_prompt: Option<String>,

    /// Vertex AI people-policy token (e.g. `"allow_adult"`). Passed through
    /// to the API unchanged.
    pub person_generation: Option<String>,

    /// Enable Google Search grounding for factual image generation.
    pub grounding: bool,

    /// Maximum concurrent inflight requests for batch generation.
    ///
    /// Defaults to `4` when `None`.
    pub batch_concurrency: Option<usize>,

    /// Directory to save generated images into. When `None` the images are
    /// returned as in-memory bytes via [`crate::output::ImageSource::Bytes`].
    pub output_dir: Option<std::path::PathBuf>,

    /// Filename prefix for auto-generated output file names. Defaults to `"gen"`.
    pub filename_prefix: Option<String>,
}

impl GenerateOptions {
    /// Start building options with a fluent builder.
    #[must_use]
    pub fn builder() -> GenerateOptionsBuilder {
        GenerateOptionsBuilder::default()
    }

    /// Effective batch concurrency: caller value or `4`.
    #[must_use]
    pub fn effective_concurrency(&self) -> usize {
        self.batch_concurrency.unwrap_or(4)
    }

    /// Effective filename prefix: caller value or `"gen"`.
    #[must_use]
    pub fn effective_prefix(&self) -> &str {
        self.filename_prefix.as_deref().unwrap_or("gen")
    }

    /// Build the final prompt string by appending any negative constraint.
    ///
    /// Mirrors the Python `full_prompt` construction.
    ///
    /// # Examples
    ///
    /// ```
    /// use aphrody_images::config::GenerateOptions;
    ///
    /// let opts = GenerateOptions { negative_prompt: Some("blur".into()), ..Default::default() };
    /// let full = opts.full_prompt("a banana");
    /// assert!(full.contains("Constraints (avoid): blur"));
    /// ```
    #[must_use]
    pub fn full_prompt<'a>(&'a self, prompt: &'a str) -> std::borrow::Cow<'a, str> {
        match &self.negative_prompt {
            Some(neg) => {
                std::borrow::Cow::Owned(format!("{prompt}\n\nConstraints (avoid): {neg}"))
            }
            None => std::borrow::Cow::Borrowed(prompt),
        }
    }
}

// ---------------------------------------------------------------------------
// GenerateOptionsBuilder
// ---------------------------------------------------------------------------

/// Fluent builder for [`GenerateOptions`].
#[derive(Debug, Default)]
pub struct GenerateOptionsBuilder {
    opts: GenerateOptions,
}

impl GenerateOptionsBuilder {
    /// Set the explicit model override.
    #[must_use]
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.opts.model = Some(model.into());
        self
    }

    /// Set the aspect ratio.
    #[must_use]
    pub fn aspect_ratio(mut self, ar: AspectRatio) -> Self {
        self.opts.aspect_ratio = Some(ar);
        self
    }

    /// Set the image size tier (Pro only).
    #[must_use]
    pub fn image_size(mut self, sz: ImageSize) -> Self {
        self.opts.image_size = Some(sz);
        self
    }

    /// Set the negative prompt string.
    #[must_use]
    pub fn negative_prompt(mut self, s: impl Into<String>) -> Self {
        self.opts.negative_prompt = Some(s.into());
        self
    }

    /// Set the person-generation policy.
    #[must_use]
    pub fn person_generation(mut self, s: impl Into<String>) -> Self {
        self.opts.person_generation = Some(s.into());
        self
    }

    /// Enable or disable Google Search grounding.
    #[must_use]
    pub fn grounding(mut self, enabled: bool) -> Self {
        self.opts.grounding = enabled;
        self
    }

    /// Set the batch concurrency cap.
    #[must_use]
    pub fn batch_concurrency(mut self, n: usize) -> Self {
        self.opts.batch_concurrency = Some(n);
        self
    }

    /// Set the output directory.
    #[must_use]
    pub fn output_dir(mut self, dir: impl Into<std::path::PathBuf>) -> Self {
        self.opts.output_dir = Some(dir.into());
        self
    }

    /// Set the filename prefix for auto-named outputs.
    #[must_use]
    pub fn filename_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.opts.filename_prefix = Some(prefix.into());
        self
    }

    /// Consume the builder, returning the completed [`GenerateOptions`].
    #[must_use]
    pub fn build(self) -> GenerateOptions {
        self.opts
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aspect_ratio_valid_values() {
        for &s in AspectRatio::VALID {
            let ar = AspectRatio::parse(s).unwrap();
            assert_eq!(ar.as_str(), s);
        }
    }

    #[test]
    fn aspect_ratio_invalid_rejected() {
        assert!(matches!(
            AspectRatio::parse("7:3"),
            Err(ImageError::UnsupportedAspectRatio(_))
        ));
    }

    #[test]
    fn image_size_round_trip() {
        assert_eq!(ImageSize::parse("1K").unwrap(), ImageSize::K1);
        assert_eq!(ImageSize::parse("2K").unwrap(), ImageSize::K2);
        assert_eq!(ImageSize::parse("4k").unwrap(), ImageSize::K4);
        assert_eq!(ImageSize::K4.as_str(), "4K");
    }

    #[test]
    fn image_size_invalid_rejected() {
        assert!(matches!(
            ImageSize::parse("8K"),
            Err(ImageError::UnsupportedImageSize(_))
        ));
    }

    #[test]
    fn full_prompt_no_negative() {
        let opts = GenerateOptions::default();
        assert_eq!(opts.full_prompt("hello").as_ref(), "hello");
    }

    #[test]
    fn full_prompt_with_negative() {
        let opts = GenerateOptions {
            negative_prompt: Some("blurry".into()),
            ..Default::default()
        };
        let full = opts.full_prompt("a cat");
        assert_eq!(full.as_ref(), "a cat\n\nConstraints (avoid): blurry");
    }

    #[test]
    fn builder_produces_expected_options() {
        let opts = GenerateOptions::builder()
            .model("gemini-custom")
            .aspect_ratio(AspectRatio::parse("16:9").unwrap())
            .image_size(ImageSize::K4)
            .negative_prompt("noise")
            .grounding(true)
            .batch_concurrency(8)
            .build();

        assert_eq!(opts.model.as_deref(), Some("gemini-custom"));
        assert_eq!(opts.aspect_ratio.as_ref().unwrap().as_str(), "16:9");
        assert_eq!(opts.image_size, Some(ImageSize::K4));
        assert_eq!(opts.negative_prompt.as_deref(), Some("noise"));
        assert!(opts.grounding);
        assert_eq!(opts.effective_concurrency(), 8);
    }
}
