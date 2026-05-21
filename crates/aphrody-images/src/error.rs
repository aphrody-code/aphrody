// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//! Typed error surface for `aphrody-images`.

use thiserror::Error;

/// All errors that can arise from image generation, download, or save
/// operations in `aphrody-images`.
#[derive(Debug, Error)]
pub enum ImageError {
    /// An unsupported aspect ratio was supplied.
    ///
    /// Valid values are the ten ratios encoded in [`crate::config::AspectRatio`].
    #[error("unsupported aspect ratio {0:?} — use one of 1:1 2:3 3:2 3:4 4:3 4:5 5:4 9:16 16:9 21:9")]
    UnsupportedAspectRatio(String),

    /// An unsupported image size was supplied.
    ///
    /// Valid values are `1K`, `2K`, `4K`.
    #[error("unsupported image size {0:?} — use one of 1K 2K 4K")]
    UnsupportedImageSize(String),

    /// The reference image list exceeds the model's per-request ceiling.
    #[error("too many reference images: got {got}, maximum is {max}")]
    TooManyReferenceImages {
        /// Number of images provided.
        got: usize,
        /// Model's maximum accepted count.
        max: usize,
    },

    /// No reference images were provided to `compose` (must be at least one).
    #[error("compose_images requires at least one reference image")]
    EmptyReferenceImages,

    /// The underlying `gemini-web` transport returned an error.
    #[error("gemini-web error: {0}")]
    GeminiWeb(#[from] gemini_web::GeminiError),

    /// The model returned a reply but contained no image URLs and no inline
    /// image data — possibly a content-policy refusal or a non-image prompt.
    #[error("model returned no image: {reason}")]
    NoImageReturned {
        /// Model that was used.
        model: String,
        /// Explanation extracted from the reply text (may be empty).
        reason: String,
    },

    /// Every model in the fallback chain failed to produce an image.
    #[error("all models in fallback chain exhausted: {0}")]
    FallbackExhausted(String),

    /// An HTTP error occurred while downloading a generated image URL.
    #[error("HTTP download error for {url}: {source}")]
    Download {
        /// The URL that failed.
        url: String,
        /// The underlying reqwest error.
        #[source]
        source: reqwest::Error,
    },

    /// The server returned a non-success HTTP status for a download.
    #[error("HTTP {status} downloading {url}")]
    DownloadStatus {
        /// URL requested.
        url: String,
        /// HTTP status code returned.
        status: u16,
    },

    /// A `data:` URI could not be decoded (malformed structure or bad base64).
    #[error("invalid data URI: {0}")]
    InvalidDataUri(String),

    /// A filesystem I/O error occurred while writing the output file.
    #[error("I/O error writing {path}: {source}")]
    Io {
        /// The path being written.
        path: std::path::PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },

    /// The source file specified for an edit or compose operation could not be
    /// read.
    #[error("failed to read source image {path}: {source}")]
    SourceRead {
        /// The path that could not be read.
        path: std::path::PathBuf,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },
}

/// Convenience alias for `aphrody-images` results.
pub type Result<T> = std::result::Result<T, ImageError>;
