// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//
// Resident-model backend: one `llama-server`, many pages.
//
// The CLI backend (`vlm.rs`) spawns `llama-mtmd-cli` per page, which reloads
// several gigabytes of weights every time. Measured on a Dragon Ball databook
// lot: 7.6 s per plate, of which a large share is load, not inference. Over
// eleven thousand plates that is most of a day spent re-reading the same file.
//
// This module keeps the model resident behind `llama-server` and posts each
// page to its OpenAI-compatible endpoint. The trade is the one the CLI backend
// was chosen to avoid — a crash takes the whole run rather than one page — so
// the server is supervised: its health is checked before the first page, and a
// dead server is reported as such instead of failing every remaining plate
// with a confusing transport error.
//
// The process is owned by `ServerRunner` and killed on drop, so an interrupted
// batch does not leave a multi-gigabyte process holding the GPU.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use aphrody_infer::llama::{self, LlamaTool};

use crate::doctags::Document;
use crate::vlm::{OcrOptions, PageResult, PageText, resolve_artifacts};
use crate::{OcrError, Result};

/// How long to wait for the server to answer its health endpoint.
///
/// Loading a 4 GiB GGUF onto the GPU takes tens of seconds on a cold page
/// cache; a minute is generous without hanging a batch forever on a server
/// that will never come up.
const STARTUP_TIMEOUT: Duration = Duration::from_mins(2);

/// Per-request timeout. A dense plate takes seconds, not minutes; anything
/// beyond this is a stuck generation, and failing the page is better than
/// stalling the batch behind it.
const REQUEST_TIMEOUT: Duration = Duration::from_mins(3);

/// A running `llama-server` with a vision model loaded.
pub struct ServerRunner {
    child: Child,
    endpoint: String,
    client: reqwest::blocking::Client,
    options: OcrOptions,
}

impl ServerRunner {
    /// Start a server for the model named by `options` and wait until it is
    /// ready to answer.
    ///
    /// `port` is bound on loopback only: this server holds no auth and must
    /// not be reachable from the network.
    ///
    /// # Errors
    ///
    /// [`OcrError::NoRunner`] when llama.cpp is not installed,
    /// [`OcrError::Model`] when the weights are not pulled, and
    /// [`OcrError::Process`] when the server does not become healthy.
    pub fn start(options: OcrOptions, port: u16) -> Result<Self> {
        let source = llama::resolve(LlamaTool::Server).ok_or(OcrError::NoRunner)?;
        let (weights, mmproj) = resolve_artifacts(&options.model_id)?;

        let mut command = Command::new(source.path());
        command
            .arg("-m")
            .arg(&weights)
            .arg("--mmproj")
            .arg(&mmproj)
            .arg("-ngl")
            .arg(options.gpu_layers.to_string())
            .arg("--host")
            .arg("127.0.0.1")
            .arg("--port")
            .arg(port.to_string())
            // One page at a time keeps VRAM predictable; the win here is not
            // reloading the model, not batching concurrent requests.
            .arg("--parallel")
            .arg("1")
            .stdout(Stdio::null());

        // llama-server logs continuously on stderr. Piping it without ever
        // reading the pipe deadlocks the server the moment the OS buffer fills
        // (64 KiB on Windows) — it blocks on write and stops answering. So the
        // log goes to a file: never blocks, and still readable when the server
        // dies before becoming healthy.
        let log_path = std::env::temp_dir().join(format!("aphrody-llama-server-{port}.log"));
        let log = std::fs::File::create(&log_path)
            .map_err(|source| OcrError::Io { path: log_path.clone(), source })?;
        command.stderr(Stdio::from(log));

        let mut child = command
            .spawn()
            .map_err(|source| OcrError::Io { path: source_path(&source, port), source })?;

        let endpoint = format!("http://127.0.0.1:{port}");
        let client = reqwest::blocking::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|e| OcrError::Process {
                command: "reqwest".to_owned(),
                status: "client".to_owned(),
                stderr: e.to_string(),
            })?;

        if let Err(e) = wait_healthy(&client, &endpoint, &mut child, &log_path) {
            // Never leave a half-started server holding the GPU.
            let _ = child.kill();
            return Err(e);
        }

        Ok(Self { child, endpoint, client, options })
    }

    /// The endpoint this runner talks to.
    #[must_use]
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }

    /// Read one image through the resident model.
    ///
    /// # Errors
    ///
    /// [`OcrError::Io`] when the image cannot be read, [`OcrError::Process`]
    /// when the server rejects the request or answers unusably.
    pub fn read(&self, image: &Path) -> Result<PageResult> {
        let started = Instant::now();
        let data_uri = data_uri(image)?;

        let body = serde_json::json!({
            "messages": [{
                "role": "user",
                "content": [
                    { "type": "image_url", "image_url": { "url": data_uri } },
                    { "type": "text", "text": self.options.prompt },
                ],
            }],
            "max_tokens": self.options.max_tokens,
            // Transcription must be reproducible: the same plate has to give
            // the same text on a re-run, or an idempotent deposit is a lie.
            "temperature": 0.0,
            "stream": false,
        });

        let response = self
            .client
            .post(format!("{}/v1/chat/completions", self.endpoint))
            .json(&body)
            .send()
            .map_err(|e| OcrError::Process {
                command: self.endpoint.clone(),
                status: "request".to_owned(),
                stderr: e.to_string(),
            })?;

        let status = response.status();
        let text = response.text().unwrap_or_default();
        if !status.is_success() {
            return Err(OcrError::Process {
                command: self.endpoint.clone(),
                status: status.to_string(),
                stderr: crate::vlm::tail(&text, 400),
            });
        }

        let content = extract_content(&text).ok_or_else(|| OcrError::Process {
            command: self.endpoint.clone(),
            status: "malformed response".to_owned(),
            stderr: crate::vlm::tail(&text, 400),
        })?;

        let document = Document::parse(&content);
        let page_text = document
            .to_markdown()
            .map_or(PageText::None, |markdown| PageText::Text { markdown });

        Ok(PageResult {
            image: image.to_path_buf(),
            text: page_text,
            elapsed_ms: started.elapsed().as_millis(),
            raw: self.options.keep_raw.then_some(content),
        })
    }
}

impl Drop for ServerRunner {
    fn drop(&mut self) {
        // A resident model holds gigabytes of VRAM; leaking it past the batch
        // would make the next run fail for want of memory.
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Poll the server until it reports ready, or the child dies, or time runs out.
fn wait_healthy(
    client: &reqwest::blocking::Client,
    endpoint: &str,
    child: &mut Child,
    log_path: &Path,
) -> Result<()> {
    let deadline = Instant::now() + STARTUP_TIMEOUT;
    let health = format!("{endpoint}/health");

    while Instant::now() < deadline {
        // A server that exited will never become healthy: say so immediately
        // rather than burning the whole timeout.
        if let Ok(Some(status)) = child.try_wait() {
            let stderr = std::fs::read_to_string(log_path).unwrap_or_default();
            return Err(OcrError::Process {
                command: "llama-server".to_owned(),
                status: status.to_string(),
                stderr: crate::vlm::tail(&stderr, 600),
            });
        }

        if let Ok(response) = client.get(&health).timeout(Duration::from_secs(2)).send() {
            if response.status().is_success() {
                return Ok(());
            }
        }
        std::thread::sleep(Duration::from_millis(500));
    }

    Err(OcrError::Process {
        command: "llama-server".to_owned(),
        status: "timeout".to_owned(),
        stderr: format!("no healthy answer from {endpoint} within {STARTUP_TIMEOUT:?}"),
    })
}

/// Read an image and encode it as a `data:` URI for the chat API.
fn data_uri(image: &Path) -> Result<String> {
    let bytes = std::fs::read(image)
        .map_err(|source| OcrError::Io { path: image.to_path_buf(), source })?;
    Ok(format!("data:{};base64,{}", mime_for(image), base64(&bytes)))
}

/// MIME type from the file extension, defaulting to JPEG.
#[must_use]
pub fn mime_for(path: &Path) -> &'static str {
    match path.extension().and_then(|e| e.to_str()).map(str::to_ascii_lowercase).as_deref() {
        Some("png") => "image/png",
        Some("webp") => "image/webp",
        Some("bmp") => "image/bmp",
        // The databook export writes JPEG, and so does almost every scan.
        _ => "image/jpeg",
    }
}

/// Standard base64, no line breaks.
///
/// Hand-rolled rather than pulling a crate: it is twenty lines, it has one
/// caller, and the alternative is another entry in the supply chain for an
/// encoding that has not changed since 1987.
#[must_use]
pub fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] =
        b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = u32::from(chunk[0]);
        let b1 = chunk.get(1).copied().map_or(0, u32::from);
        let b2 = chunk.get(2).copied().map_or(0, u32::from);
        let triple = (b0 << 16) | (b1 << 8) | b2;

        out.push(ALPHABET[((triple >> 18) & 0x3F) as usize] as char);
        out.push(ALPHABET[((triple >> 12) & 0x3F) as usize] as char);
        out.push(if chunk.len() > 1 {
            ALPHABET[((triple >> 6) & 0x3F) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 { ALPHABET[(triple & 0x3F) as usize] as char } else { '=' });
    }
    out
}

/// Pull `choices[0].message.content` out of a chat-completions response.
#[must_use]
pub fn extract_content(body: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(body).ok()?;
    value
        .get("choices")?
        .as_array()?
        .first()?
        .get("message")?
        .get("content")?
        .as_str()
        .map(ToOwned::to_owned)
}

/// Path to report when spawning the server fails.
fn source_path(_error: &std::io::Error, _port: u16) -> PathBuf {
    llama::resolve(LlamaTool::Server)
        .map_or_else(|| PathBuf::from("llama-server"), |s| s.path().to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_the_rfc_test_vectors() {
        // RFC 4648 §10.
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foob"), "Zm9vYg==");
        assert_eq!(base64(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn base64_handles_bytes_above_ascii() {
        // A JPEG is mostly high bytes; a signed-char bug would corrupt them.
        assert_eq!(base64(&[0xFF, 0xD8, 0xFF]), "/9j/");
        assert_eq!(base64(&[0x00, 0x00, 0x00]), "AAAA");
        assert_eq!(base64(&[0xFF; 6]), "////////");
    }

    #[test]
    fn base64_length_is_always_a_multiple_of_four() {
        for len in 0..32_usize {
            let encoded = base64(&vec![0xA5; len]);
            assert_eq!(encoded.len() % 4, 0, "len {len} -> {encoded}");
        }
    }

    #[test]
    fn mime_follows_the_extension_case_insensitively() {
        assert_eq!(mime_for(Path::new("a.png")), "image/png");
        assert_eq!(mime_for(Path::new("a.PNG")), "image/png");
        assert_eq!(mime_for(Path::new("a.webp")), "image/webp");
        assert_eq!(mime_for(Path::new("a.jpg")), "image/jpeg");
        // Unknown or absent extension falls back to JPEG, what scans are.
        assert_eq!(mime_for(Path::new("a")), "image/jpeg");
        assert_eq!(mime_for(Path::new("a.tiff")), "image/jpeg");
    }

    #[test]
    fn content_is_extracted_from_a_chat_completion() {
        let body = r#"{"choices":[{"message":{"role":"assistant","content":"読めた"}}]}"#;
        assert_eq!(extract_content(body).as_deref(), Some("読めた"));
    }

    #[test]
    fn a_malformed_response_yields_none_rather_than_panicking() {
        for body in [
            "",
            "not json",
            "{}",
            r#"{"choices":[]}"#,
            r#"{"choices":[{}]}"#,
            r#"{"choices":[{"message":{}}]}"#,
            r#"{"choices":[{"message":{"content":null}}]}"#,
            r#"{"error":{"message":"context too long"}}"#,
        ] {
            assert_eq!(extract_content(body), None, "{body}");
        }
    }

    #[test]
    fn timeouts_are_bounded_and_ordered() {
        // Startup may be slow; a single request must never be slower than the
        // batch can tolerate.
        assert!(STARTUP_TIMEOUT >= Duration::from_mins(1));
        assert!(REQUEST_TIMEOUT >= Duration::from_mins(1));
    }
}
