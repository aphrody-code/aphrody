// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//
// `aphrody-images` — production-grade Nano Banana image generation, editing
// and composition for the aphrody CLI.
//
// # What the Python reference (`apps/aphrody/aphrody/images.py`) does
//
// The Python module implements three operations via the `google.genai` Vertex
// AI SDK (`generate_content` with `response_modalities=["TEXT","IMAGE"]`):
//   * `generate_image(prompt, n, aspect_ratio, image_size, …)` — text-to-image
//     with sequential per-image calls and a synchronous fallback chain
//     (Pro → 2-Flash → Flash).
//   * `edit_image(image, prompt)` — image-in / image-out editing.
//   * `compose_images(images, prompt)` — multi-reference (up to 14) blending.
//
// All three extract `inline_data.data` bytes from the API response and write
// them atomically to disk, or return raw bytes when no output path is given.
// Generation is **synchronous** (blocking SDK calls), **sequential** (no
// concurrency even for n>1), and output is always PNG-named regardless of
// actual content type.  URL-format responses (Nano Banana web path via
// `generated_image_urls`) are NOT handled — the Python only touches the Vertex
// inline-data path.
//
// # How `aphrody-images` surpasses it
//
//   * **Async-native**: every I/O operation is `async`; no blocking calls on the
//     async executor.
//   * **Concurrent batch generation**: `generate_batch` drives N prompts in
//     parallel, bounded by a configurable `Semaphore` to cap inflight requests.
//   * **URL download path**: handles `ChatReply::generated_image_urls` returned
//     by the gemini-web `GeminiWebClient` — downloads each URL concurrently with
//     `reqwest`, maps `Content-Type` → extension (png/jpg/webp/gif/…).
//   * **Data-URI / base64 inline payload decode**: a real decoder for
//     `data:<mime>;base64,<payload>` strings as sometimes delivered in JSON blobs.
//   * **Typed result structs** (`GeneratedImage`, `ImageGenerationResult`): the
//     caller gets structured data, not an untyped `list[Path] | list[bytes]`.
//   * **Correct extension mapping**: the extension derives from the actual
//     `Content-Type` response header (or the data-URI MIME), not a hard-coded
//     `.png` suffix.
//   * **Atomic disk writes**: temporary-file + rename, identical to the Python's
//     `_write_png` but properly async (no blocking `std::fs` on a tokio thread).
//   * **`#![forbid(unsafe_code)]`**, clippy::pedantic, Edition 2024 async closures.
//   * **Zero-allocation model registry** via `const` arrays and `&'static str`.
//   * **`thiserror`-powered typed errors** with fine-grained variants, not bare
//     `RuntimeError` strings.

#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod config;
pub mod download;
pub mod error;
pub mod generate;
pub mod models;
pub mod output;

pub use config::{AspectRatio, GenerateOptions, ImageSize};
pub use download::{download_image_urls, DownloadedImage};
pub use error::{ImageError, Result};
pub use generate::{generate_batch, generate_single, ImageClient};
pub use models::{ImageModel, MODEL_FALLBACK_CHAIN};
pub use output::{GeneratedImage, ImageSource, SavedImage};
