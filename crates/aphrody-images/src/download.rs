// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//! Concurrent image URL download.
//!
//! The Python reference (`images.py`) only handles inline `data` bytes from
//! the Vertex AI `generate_content` response; it does not download the
//! `generated_image_urls` that the Gemini web surface returns.
//!
//! This module fills that gap: given a list of URLs (e.g. from
//! [`gemini_web::types::ChatReply::generated_image_urls`]), it downloads them
//! concurrently using `reqwest`, maps the `Content-Type` response header to
//! the correct file extension, and returns structured [`DownloadedImage`]
//! values.
//!
//! Concurrency is bounded by a `tokio::sync::Semaphore` — the caller controls
//! the limit via [`download_image_urls`]'s `max_concurrent` parameter.

use crate::error::{ImageError, Result};
use crate::output::{mime_to_extension, GeneratedImage, ImageSource};
use tokio::sync::Semaphore;

/// A downloaded image with its raw bytes, MIME type and origin URL.
///
/// Equivalent to [`GeneratedImage`] but specifically for the URL-download path.
/// You can convert this into a [`GeneratedImage`] for unified handling.
#[derive(Debug, Clone)]
pub struct DownloadedImage {
    /// The URL that was downloaded.
    pub url: String,
    /// Raw bytes of the image.
    pub bytes: Vec<u8>,
    /// `Content-Type` header value returned by the server.
    pub content_type: String,
    /// Zero-based index in the original URL list.
    pub index: usize,
}

impl DownloadedImage {
    /// Inferred file extension from `Content-Type`.
    #[must_use]
    pub fn extension(&self) -> &'static str {
        mime_to_extension(&self.content_type)
    }

    /// Convert into a [`GeneratedImage`] for use with the unified output path.
    #[must_use]
    pub fn into_generated(self) -> GeneratedImage {
        GeneratedImage {
            bytes: self.bytes,
            source: ImageSource::Url {
                url: self.url,
                content_type: self.content_type,
            },
            index: self.index,
        }
    }
}

/// Download a list of image URLs concurrently, bounded to `max_concurrent`
/// inflight requests.
///
/// Each URL is fetched with `reqwest`; the `Content-Type` response header is
/// used to derive the correct file extension. HTTP errors and non-2xx statuses
/// are returned as [`ImageError`] variants.
///
/// The returned vector preserves the original URL order (index is stable).
///
/// # Arguments
///
/// * `client` — a configured `reqwest::Client` (caller supplies — avoids
///   re-creating the client per batch, which would bypass connection pooling).
/// * `urls` — list of image URLs to download.
/// * `max_concurrent` — maximum number of simultaneous HTTP requests.
///
/// # Errors
///
/// [`ImageError::Download`] for `reqwest`-level errors, or
/// [`ImageError::DownloadStatus`] for non-2xx HTTP responses.
///
/// # Panics
///
/// Panics only if an internal `tokio::spawn` task panics (which would indicate
/// a bug in this crate). The semaphore-closed panic is also unreachable in
/// practice because the semaphore lives for the duration of the call.
///
/// # Examples
///
/// ```no_run
/// # async fn run() -> aphrody_images::Result<()> {
/// use aphrody_images::download::download_image_urls;
///
/// let client = reqwest::Client::new();
/// let urls = vec!["https://lh3.googleusercontent.com/example".to_string()];
/// let images = download_image_urls(&client, &urls, 4).await?;
/// for img in images {
///     println!("Downloaded {} bytes ({})", img.bytes.len(), img.extension());
/// }
/// # Ok(())
/// # }
/// ```
pub async fn download_image_urls(
    client: &reqwest::Client,
    urls: &[String],
    max_concurrent: usize,
) -> Result<Vec<DownloadedImage>> {
    if urls.is_empty() {
        return Ok(Vec::new());
    }

    let sem = std::sync::Arc::new(Semaphore::new(max_concurrent));
    let mut handles = Vec::with_capacity(urls.len());

    for (index, url) in urls.iter().enumerate() {
        let sem = sem.clone();
        let client = client.clone();
        let url = url.clone();

        let handle = tokio::spawn(async move {
            // Acquire a permit before issuing the request.
            let _permit = sem.acquire().await.expect("semaphore closed");
            download_single(&client, url, index).await
        });
        handles.push(handle);
    }

    // Collect results in order, propagating the first error.
    let mut results = Vec::with_capacity(handles.len());
    for handle in handles {
        // Panics in subtasks are propagated as errors here.
        let img = handle.await.expect("download task panicked")?;
        results.push(img);
    }

    // Sort by index to restore original order (tokio::spawn scheduling may
    // complete out-of-order).
    results.sort_unstable_by_key(|i| i.index);
    Ok(results)
}

/// Download a single image URL, returning a [`DownloadedImage`].
async fn download_single(
    client: &reqwest::Client,
    url: String,
    index: usize,
) -> Result<DownloadedImage> {
    tracing::debug!(url = %url, index, "downloading generated image");

    let response = client.get(&url).send().await.map_err(|e| ImageError::Download {
        url: url.clone(),
        source: e,
    })?;

    let status = response.status();
    if !status.is_success() {
        return Err(ImageError::DownloadStatus {
            url,
            status: status.as_u16(),
        });
    }

    // Extract content type before consuming the response body.
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/png")
        .to_string();

    let bytes = response.bytes().await.map_err(|e| ImageError::Download {
        url: url.clone(),
        source: e,
    })?;

    tracing::debug!(
        url = %url,
        bytes = bytes.len(),
        content_type = %content_type,
        "image downloaded",
    );

    Ok(DownloadedImage {
        url,
        bytes: bytes.to_vec(),
        content_type,
        index,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downloaded_image_extension_maps_content_type() {
        let img = DownloadedImage {
            url: "https://example.com/img".into(),
            bytes: vec![],
            content_type: "image/webp".into(),
            index: 0,
        };
        assert_eq!(img.extension(), "webp");
    }

    #[test]
    fn downloaded_image_extension_with_params() {
        let img = DownloadedImage {
            url: "https://example.com/img".into(),
            bytes: vec![1, 2, 3],
            content_type: "image/jpeg; charset=utf-8".into(),
            index: 0,
        };
        assert_eq!(img.extension(), "jpg");
    }

    #[test]
    fn into_generated_preserves_data() {
        let img = DownloadedImage {
            url: "https://lh3.googleusercontent.com/abc".into(),
            bytes: vec![0xDE, 0xAD],
            content_type: "image/png".into(),
            index: 2,
        };
        let gimg = img.into_generated();
        assert_eq!(gimg.bytes, vec![0xDE, 0xAD]);
        assert_eq!(gimg.index, 2);
        assert_eq!(gimg.extension(), "png");
        assert!(matches!(
            gimg.source,
            ImageSource::Url { url, .. } if url == "https://lh3.googleusercontent.com/abc"
        ));
    }

    #[tokio::test]
    async fn download_empty_url_list_returns_empty() {
        // Install a rustls CryptoProvider before creating any reqwest::Client.
        // The empty-list path returns before any network I/O.
        let _ = rustls::crypto::ring::default_provider().install_default();
        let client = reqwest::Client::new();
        let result = download_image_urls(&client, &[], 4).await.unwrap();
        assert!(result.is_empty());
    }
}
