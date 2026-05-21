// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//! Typed image output structs and atomic disk-write helpers.
//!
//! Unlike the Python reference, which returns either `list[Path]` or
//! `list[bytes]` (a union type that collapses type safety), this module
//! provides structured result types that carry the image bytes, the inferred
//! MIME type, the chosen filename extension, and the optional disk path — all
//! in one strongly-typed value.

use crate::error::{ImageError, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// MIME-type → file extension mapping
// ---------------------------------------------------------------------------

/// Map a `Content-Type` header value or MIME string to a file extension.
///
/// The Python reference hard-codes `.png` for every output regardless of the
/// actual content type. This function derives the correct extension from the
/// actual MIME delivered by the server.
///
/// # Examples
///
/// ```
/// use aphrody_images::output::mime_to_extension;
///
/// assert_eq!(mime_to_extension("image/png"),  "png");
/// assert_eq!(mime_to_extension("image/jpeg"), "jpg");
/// assert_eq!(mime_to_extension("image/webp"), "webp");
/// assert_eq!(mime_to_extension("image/gif"),  "gif");
/// assert_eq!(mime_to_extension("image/avif"), "avif");
/// assert_eq!(mime_to_extension("image/tiff"), "tiff");
/// // Unknown / missing MIME falls back to "bin".
/// assert_eq!(mime_to_extension("application/octet-stream"), "bin");
/// assert_eq!(mime_to_extension(""), "bin");
/// ```
#[must_use]
pub fn mime_to_extension(mime: &str) -> &'static str {
    // Strip any parameters (e.g. `image/jpeg; charset=utf-8`).
    let base = mime.split(';').next().unwrap_or("").trim();
    match base {
        "image/png"                           => "png",
        "image/jpeg" | "image/jpg"            => "jpg",
        "image/webp"                          => "webp",
        "image/gif"                           => "gif",
        "image/avif"                          => "avif",
        "image/tiff"                          => "tiff",
        "image/bmp"                           => "bmp",
        "image/svg+xml"                       => "svg",
        "image/heic" | "image/heif"           => "heic",
        _                                     => "bin",
    }
}

// ---------------------------------------------------------------------------
// Data-URI decode
// ---------------------------------------------------------------------------

/// Decode a `data:<mime>;base64,<payload>` URI into `(mime, raw_bytes)`.
///
/// This goes beyond the Python reference, which has no data-URI handling.
/// Gemini's JSON responses sometimes embed images as inline data URIs
/// especially for smaller outputs or non-URL delivery paths.
///
/// # Errors
///
/// [`ImageError::InvalidDataUri`] when the string is not a valid `data:` URI
/// with a base64-encoded payload.
///
/// # Examples
///
/// ```
/// use aphrody_images::output::decode_data_uri;
/// use base64::Engine as _;
///
/// let bytes = b"hello image";
/// let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
/// let uri = format!("data:image/png;base64,{encoded}");
///
/// let (mime, decoded) = decode_data_uri(&uri).unwrap();
/// assert_eq!(mime, "image/png");
/// assert_eq!(decoded, bytes);
/// ```
pub fn decode_data_uri(uri: &str) -> Result<(String, Vec<u8>)> {
    use base64::Engine as _;

    let rest = uri
        .strip_prefix("data:")
        .ok_or_else(|| ImageError::InvalidDataUri("missing 'data:' prefix".to_string()))?;

    let (header, payload) = rest.split_once(',').ok_or_else(|| {
        ImageError::InvalidDataUri("missing ',' separator between header and payload".to_string())
    })?;

    // Header is `<mime>[;base64]`.  Only base64 encoding is supported.
    let (mime_part, is_base64) = if let Some(m) = header.strip_suffix(";base64") {
        (m, true)
    } else {
        (header, false)
    };

    if !is_base64 {
        return Err(ImageError::InvalidDataUri(
            "only base64-encoded data URIs are supported".to_string(),
        ));
    }

    let raw = base64::engine::general_purpose::STANDARD
        .decode(payload)
        .map_err(|e| ImageError::InvalidDataUri(format!("base64 decode error: {e}")))?;

    Ok((mime_part.to_string(), raw))
}

// ---------------------------------------------------------------------------
// ImageSource — where the bytes came from
// ---------------------------------------------------------------------------

/// The provenance of a [`GeneratedImage`]'s bytes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ImageSource {
    /// Image bytes came from downloading a URL returned by the model.
    Url {
        /// The original URL that was downloaded.
        url: String,
        /// HTTP Content-Type header value.
        content_type: String,
    },
    /// Image bytes came from a `data:` URI embedded in the response JSON.
    DataUri {
        /// MIME type extracted from the data-URI header.
        mime: String,
    },
    /// Image bytes came from an inline binary payload in the API response
    /// (Vertex `inline_data` equivalent).
    Inline {
        /// MIME type declared in the API response.
        mime: String,
    },
}

impl ImageSource {
    /// Return the MIME type regardless of variant.
    #[must_use]
    pub fn mime(&self) -> &str {
        match self {
            Self::Url { content_type, .. } => content_type,
            Self::DataUri { mime } | Self::Inline { mime } => mime,
        }
    }

    /// Derive the appropriate file extension from the MIME type.
    #[must_use]
    pub fn extension(&self) -> &'static str {
        mime_to_extension(self.mime())
    }
}

// ---------------------------------------------------------------------------
// GeneratedImage — in-memory result
// ---------------------------------------------------------------------------

/// A single generated image held in memory.
///
/// This is the intermediate form returned by the generation layer before any
/// save decision is made. The caller can inspect the bytes, source and MIME,
/// then call [`GeneratedImage::save`] or consume the bytes directly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedImage {
    /// Raw image bytes.
    pub bytes: Vec<u8>,
    /// Provenance of the bytes.
    pub source: ImageSource,
    /// Zero-based index within a batch (always `0` for single-image calls).
    pub index: usize,
}

impl GeneratedImage {
    /// The inferred file extension for this image (e.g. `"png"`, `"jpg"`).
    #[must_use]
    pub fn extension(&self) -> &'static str {
        self.source.extension()
    }

    /// Save this image to a specific path atomically (temp file + rename).
    ///
    /// Creates parent directories as needed. The extension in `path` is NOT
    /// overridden — if the caller wants the correct extension they should use
    /// [`GeneratedImage::extension`] to build the path.
    ///
    /// # Errors
    ///
    /// [`ImageError::Io`] on any filesystem error.
    pub async fn save(&self, path: &Path) -> Result<SavedImage> {
        write_atomic(path, &self.bytes).await?;
        tracing::debug!(
            bytes = self.bytes.len(),
            path = %path.display(),
            "image saved",
        );
        Ok(SavedImage {
            path: path.to_path_buf(),
            bytes_written: self.bytes.len(),
            source: self.source.clone(),
            index: self.index,
        })
    }

    /// Auto-save to `dir` using a timestamp-prefixed filename.
    ///
    /// The filename is `{prefix}_{timestamp_us}_{index}.{ext}` where `ext`
    /// comes from the actual content MIME type — not a hard-coded `.png`.
    ///
    /// # Errors
    ///
    /// [`ImageError::Io`] on any filesystem error.
    pub async fn save_to_dir(
        &self,
        dir: &Path,
        prefix: &str,
    ) -> Result<SavedImage> {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_micros());
        let filename = format!("{prefix}_{ts}_{}.{}", self.index, self.extension());
        let path = dir.join(&filename);
        self.save(&path).await
    }
}

// ---------------------------------------------------------------------------
// SavedImage — persisted result
// ---------------------------------------------------------------------------

/// A single generated image that has been written to disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SavedImage {
    /// Absolute path of the file on disk.
    pub path: PathBuf,
    /// Number of bytes written.
    pub bytes_written: usize,
    /// Provenance of the bytes.
    pub source: ImageSource,
    /// Zero-based batch index.
    pub index: usize,
}

// ---------------------------------------------------------------------------
// ImageGenerationResult — top-level result wrapper
// ---------------------------------------------------------------------------

/// The result of a [`crate::generate::generate_single`] or
/// [`crate::generate::generate_batch`] call.
///
/// Contains all generated images plus metadata about which model ultimately
/// produced them (useful for fallback-chain observability).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenerationResult {
    /// The images produced (one per prompt in a batch, possibly multiple
    /// per single call when `n > 1`).
    pub images: Vec<GeneratedImage>,

    /// The model that produced the images (post-fallback resolution).
    pub model_used: String,

    /// Total time from first request to last image ready, in milliseconds.
    pub elapsed_ms: u64,
}

// ---------------------------------------------------------------------------
// Atomic write helper
// ---------------------------------------------------------------------------

/// Write `data` to `dest` atomically via a sibling `.tmp` file + rename.
///
/// Creates all parent directories before writing. On failure the `.tmp` file
/// is cleaned up. Uses `tokio::fs` throughout — no blocking on the executor.
///
/// This is the async equivalent of the Python `_write_png` helper, with the
/// correct extension derived from content type rather than hard-coded `.png`.
///
/// # Errors
///
/// [`ImageError::Io`] wrapping the underlying `std::io::Error`.
pub async fn write_atomic(dest: &Path, data: &[u8]) -> Result<()> {
    // Ensure parent directory exists.
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| ImageError::Io {
            path: parent.to_path_buf(),
            source: e,
        })?;
    }

    // Build a sibling temp path.
    let tmp = dest.with_extension(format!(
        "{}.tmp",
        dest.extension().and_then(|e| e.to_str()).unwrap_or("bin")
    ));

    // Write to temp, then rename.
    tokio::fs::write(&tmp, data).await.map_err(|e| ImageError::Io {
        path: tmp.clone(),
        source: e,
    })?;

    tokio::fs::rename(&tmp, dest).await.map_err(|e| {
        // Best-effort cleanup of the temp file — ignore errors here.
        let tmp2 = tmp.clone();
        tokio::spawn(async move {
            let _ = tokio::fs::remove_file(&tmp2).await;
        });
        ImageError::Io { path: dest.to_path_buf(), source: e }
    })?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mime_to_extension_known_types() {
        assert_eq!(mime_to_extension("image/png"), "png");
        assert_eq!(mime_to_extension("image/jpeg"), "jpg");
        assert_eq!(mime_to_extension("image/jpg"), "jpg");
        assert_eq!(mime_to_extension("image/webp"), "webp");
        assert_eq!(mime_to_extension("image/gif"), "gif");
        assert_eq!(mime_to_extension("image/avif"), "avif");
        assert_eq!(mime_to_extension("image/tiff"), "tiff");
        assert_eq!(mime_to_extension("image/bmp"), "bmp");
        assert_eq!(mime_to_extension("image/svg+xml"), "svg");
        assert_eq!(mime_to_extension("image/heic"), "heic");
        assert_eq!(mime_to_extension("image/heif"), "heic");
    }

    #[test]
    fn mime_to_extension_with_params() {
        // Content-Type headers often include charset or other params.
        assert_eq!(mime_to_extension("image/png; charset=utf-8"), "png");
        assert_eq!(mime_to_extension("image/jpeg; quality=high"), "jpg");
    }

    #[test]
    fn mime_to_extension_unknown_falls_back() {
        assert_eq!(mime_to_extension("application/octet-stream"), "bin");
        assert_eq!(mime_to_extension(""), "bin");
        assert_eq!(mime_to_extension("video/mp4"), "bin");
    }

    #[test]
    fn decode_data_uri_valid_png() {
        use base64::Engine as _;
        let bytes = b"\x89PNG\r\n\x1a\n fake png body";
        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
        let uri = format!("data:image/png;base64,{encoded}");
        let (mime, decoded) = decode_data_uri(&uri).unwrap();
        assert_eq!(mime, "image/png");
        assert_eq!(decoded, bytes);
    }

    #[test]
    fn decode_data_uri_invalid_prefix() {
        assert!(matches!(
            decode_data_uri("https://example.com/image.png"),
            Err(ImageError::InvalidDataUri(_))
        ));
    }

    #[test]
    fn decode_data_uri_non_base64() {
        assert!(matches!(
            decode_data_uri("data:image/png,raw-text-payload"),
            Err(ImageError::InvalidDataUri(_))
        ));
    }

    #[test]
    fn decode_data_uri_bad_base64_payload() {
        // Valid structure but invalid base64 content.
        let uri = "data:image/png;base64,!!!not-valid-base64!!!";
        assert!(matches!(
            decode_data_uri(uri),
            Err(ImageError::InvalidDataUri(_))
        ));
    }

    #[test]
    fn image_source_extension_derives_from_mime() {
        let src = ImageSource::Url {
            url: "https://example.com/img".into(),
            content_type: "image/webp".into(),
        };
        assert_eq!(src.extension(), "webp");

        let src2 = ImageSource::DataUri { mime: "image/jpeg".into() };
        assert_eq!(src2.extension(), "jpg");

        let src3 = ImageSource::Inline { mime: "image/png".into() };
        assert_eq!(src3.extension(), "png");
    }

    #[tokio::test]
    async fn write_atomic_creates_file_and_dirs() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub").join("image.png");
        let data = b"fake png data";

        write_atomic(&path, data).await.unwrap();

        let written = tokio::fs::read(&path).await.unwrap();
        assert_eq!(written, data);
    }
}
