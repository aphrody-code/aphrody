// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//
// The downloader: resumable, digest-checked HTTP transfer of model weights
// into the store.
//
// Design points that matter for multi-gigabyte artefacts:
//
//   * Bytes are streamed to a `<final-name>.part` sibling and only renamed
//     onto the final path once the transfer AND the digest check pass, so a
//     killed process can never leave a truncated file that looks installed.
//   * An interrupted transfer resumes with a `Range:` request. The bytes
//     already on disk are re-hashed into the running digest before the socket
//     is opened, so resumption still yields a whole-file SHA-256.
//   * A server that ignores `Range` (answers 200 instead of 206) is handled by
//     restarting the transfer rather than appending onto a stale prefix.
//
// Host-only: this module is not compiled for wasm32.

use std::io::Write as _;
use std::path::Path;
use std::sync::Once;

use futures_util::StreamExt as _;

use crate::digest::{self, Hasher};
use crate::error::{ModelError, Result};
use crate::id::ModelRef;
use crate::store::{InstalledModel, ModelStore};

/// Environment variables consulted for a Hugging Face access token, in order.
/// Both spellings are in wide use; `HF_TOKEN` is the modern one.
const HF_TOKEN_VARS: [&str; 2] = ["HF_TOKEN", "HUGGING_FACE_HUB_TOKEN"];

/// Transfer progress, reported once per received chunk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Progress {
    /// Bytes written so far, including any resumed prefix.
    pub downloaded: u64,
    /// Total size when the server advertised one.
    pub total: Option<u64>,
}

impl Progress {
    /// Completion ratio in `0.0..=1.0`, when the total size is known.
    #[must_use]
    pub fn fraction(&self) -> Option<f64> {
        self.total.filter(|t| *t > 0).map(|t| {
            // Saturate rather than exceed 1.0 if a server under-reports.
            (self.downloaded as f64 / t as f64).min(1.0)
        })
    }
}

/// Knobs for a single pull.
#[derive(Debug, Clone, Default)]
pub struct PullOptions {
    /// Digest the artefact must match. Accepts bare hex or `sha256:` form.
    /// When set and the download does not match, the transfer is discarded.
    pub expected_sha256: Option<String>,
    /// Re-download even when the artefact is already present and intact.
    pub force: bool,
    /// Catalog id to record alongside the entry, for provenance.
    pub catalog_id: Option<String>,
}

/// What a pull did.
#[derive(Debug, Clone, PartialEq)]
pub enum PullOutcome {
    /// The artefact was already installed; nothing was transferred.
    AlreadyPresent(InstalledModel),
    /// The artefact was transferred (possibly resumed) and installed.
    Downloaded(InstalledModel),
    /// The reference points at a file already on disk outside the store, so
    /// it was adopted rather than copied.
    Adopted(InstalledModel),
}

impl PullOutcome {
    /// The resulting registry entry, whichever path was taken.
    #[must_use]
    pub fn model(&self) -> &InstalledModel {
        match self {
            Self::AlreadyPresent(m) | Self::Downloaded(m) | Self::Adopted(m) => m,
        }
    }

    /// Whether bytes actually crossed the network.
    #[must_use]
    pub const fn transferred(&self) -> bool {
        matches!(self, Self::Downloaded(_))
    }
}

/// A reusable HTTP client for weight downloads.
pub struct Downloader {
    client: reqwest::Client,
}

impl Downloader {
    /// Build a downloader with aphrody's default HTTP posture.
    ///
    /// # Errors
    ///
    /// [`ModelError::Download`] when the TLS/HTTP client cannot be built.
    pub fn new() -> Result<Self> {
        ensure_crypto_provider();
        let client = reqwest::Client::builder()
            .user_agent(concat!("aphrody-models/", env!("CARGO_PKG_VERSION")))
            // Weight files are large; the read timeout guards against a stalled
            // socket without capping total transfer time.
            .read_timeout(std::time::Duration::from_mins(2))
            .connect_timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| ModelError::Download {
                url: String::from("<client>"),
                reason: e.to_string(),
            })?;
        Ok(Self { client })
    }

    /// Wrap an already-configured client (proxy, custom roots, test server).
    #[must_use]
    pub const fn with_client(client: reqwest::Client) -> Self {
        Self { client }
    }

    /// Fetch `reference` into `store`, resuming an interrupted transfer when
    /// one is present, and record it in the registry.
    ///
    /// `on_progress` is invoked for every received chunk; pass `|_| {}` to
    /// ignore it.
    ///
    /// # Errors
    ///
    /// [`ModelError::Download`] on transport or HTTP-status failure,
    /// [`ModelError::ChecksumMismatch`] when `expected_sha256` does not match
    /// the received bytes, [`ModelError::Io`] on filesystem failure.
    pub async fn pull(
        &self,
        store: &ModelStore,
        reference: &ModelRef,
        options: &PullOptions,
        mut on_progress: impl FnMut(Progress),
    ) -> Result<PullOutcome> {
        // A `file:` reference is already on disk: adopt it in place.
        if let ModelRef::Local(path) = reference {
            let entry = store.adopt_local(path)?;
            return Ok(PullOutcome::Adopted(entry));
        }

        let final_path = store.path_for(reference);
        if !options.force && final_path.is_file() {
            if let Some(existing) = store.get(reference)? {
                // Trust the index only if the bytes still match it, otherwise
                // fall through and re-download.
                if store.verify(reference)?.is_intact() {
                    return Ok(PullOutcome::AlreadyPresent(existing));
                }
            }
        }

        let url = reference
            .download_url()
            .ok_or_else(|| ModelError::BadRef {
                input: reference.to_string(),
                reason: "reference has no download URL",
            })?;

        let part_path = store.part_path_for(reference);
        if options.force {
            // A forced pull must not append onto a previous attempt.
            let _ = std::fs::remove_file(&part_path);
        }
        if let Some(parent) = part_path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| ModelError::io(parent.to_path_buf(), e))?;
        }

        let received = self.stream_to_part(&url, &part_path, &mut on_progress).await?;

        if let Some(expected) = &options.expected_sha256 {
            let expected = digest::normalize_digest(expected);
            if expected != received {
                // Never promote bytes that failed the check, and never leave
                // them around to be resumed onto.
                let _ = std::fs::remove_file(&part_path);
                return Err(ModelError::ChecksumMismatch {
                    model: reference.to_string(),
                    expected,
                    actual: received,
                });
            }
        }

        std::fs::rename(&part_path, &final_path)
            .map_err(|e| ModelError::io(final_path.clone(), e))?;

        let entry = store.describe_file(reference, &final_path, options.catalog_id.clone())?;
        store.record(entry.clone())?;
        Ok(PullOutcome::Downloaded(entry))
    }

    /// Stream `url` into `part_path`, resuming if a partial file exists.
    /// Returns the whole-file SHA-256 of what now sits in `part_path`.
    async fn stream_to_part(
        &self,
        url: &str,
        part_path: &Path,
        on_progress: &mut impl FnMut(Progress),
    ) -> Result<String> {
        let resume_from = std::fs::metadata(part_path).map_or(0, |m| m.len());

        let mut request = self.client.get(url);
        if let Some(token) = hf_token(url) {
            request = request.bearer_auth(token);
        }
        if resume_from > 0 {
            request = request.header(reqwest::header::RANGE, format!("bytes={resume_from}-"));
        }

        let response = request.send().await.map_err(|e| ModelError::Download {
            url: url.to_owned(),
            reason: e.to_string(),
        })?;

        let status = response.status();
        if !status.is_success() {
            return Err(ModelError::Download {
                url: url.to_owned(),
                reason: format!(
                    "HTTP {}{}",
                    status.as_u16(),
                    status.canonical_reason().map(|r| format!(" {r}")).unwrap_or_default()
                ),
            });
        }

        // 206 means the server honoured the range and we append; anything else
        // (including a 200 answering a range request) restarts from scratch.
        let resuming = resume_from > 0 && status == reqwest::StatusCode::PARTIAL_CONTENT;

        let mut hasher = Hasher::new();
        let mut written = 0_u64;
        if resuming {
            // Fold the prefix already on disk into the digest so the final
            // hash covers the whole artefact, not just this leg.
            hash_existing(part_path, &mut hasher)?;
            written = resume_from;
        }

        let total = response
            .content_length()
            .map(|remaining| if resuming { remaining + resume_from } else { remaining });

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .append(resuming)
            .truncate(!resuming)
            .open(part_path)
            .map_err(|e| ModelError::io(part_path.to_path_buf(), e))?;

        let mut stream = response.bytes_stream();
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| ModelError::Download {
                url: url.to_owned(),
                reason: e.to_string(),
            })?;
            file.write_all(&chunk).map_err(|e| ModelError::io(part_path.to_path_buf(), e))?;
            hasher.update(&chunk);
            written = written.saturating_add(chunk.len() as u64);
            on_progress(Progress { downloaded: written, total });
        }
        file.flush().map_err(|e| ModelError::io(part_path.to_path_buf(), e))?;
        // Durability: the rename that follows is atomic, but the data it
        // publishes must already be on the platter.
        file.sync_all().map_err(|e| ModelError::io(part_path.to_path_buf(), e))?;

        Ok(hasher.finish_hex())
    }
}

/// Feed an existing partial file into a running digest.
fn hash_existing(path: &Path, hasher: &mut Hasher) -> Result<()> {
    use std::io::Read as _;

    let mut file =
        std::fs::File::open(path).map_err(|e| ModelError::io(path.to_path_buf(), e))?;
    let mut buf = vec![0_u8; 1 << 20];
    loop {
        let n = file.read(&mut buf).map_err(|e| ModelError::io(path.to_path_buf(), e))?;
        if n == 0 {
            return Ok(());
        }
        hasher.update(&buf[..n]);
    }
}

/// Whether a bearer token may be attached to this URL.
///
/// Host-gated so a Hub token is never leaked to an unrelated download server
/// named in a `url:` reference.
fn is_hub_url(url: &str) -> bool {
    url.starts_with("https://huggingface.co/")
}

/// First candidate that carries an actual value; blank vars are ignored so an
/// exported-but-empty `HF_TOKEN` falls through to the other spelling.
fn first_usable_token(candidates: impl IntoIterator<Item = String>) -> Option<String> {
    candidates.into_iter().find(|value| !value.trim().is_empty())
}

/// Hugging Face access token, when the URL targets the Hub and one is set.
fn hf_token(url: &str) -> Option<String> {
    if !is_hub_url(url) {
        return None;
    }
    first_usable_token(HF_TOKEN_VARS.iter().filter_map(|var| std::env::var(var).ok()))
}

/// Install the process-wide rustls crypto provider exactly once.
///
/// reqwest is built with `rustls-no-provider` workspace-wide, so the first
/// client construction panics with `No provider set` unless a provider was
/// installed first (CLAUDE.md §7). Installing twice is not an error we care
/// about — another crate winning the race is a perfectly good outcome.
fn ensure_crypto_provider() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        let _ = rustls::crypto::ring::default_provider().install_default();
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_fraction_is_clamped_and_optional() {
        assert_eq!(Progress { downloaded: 50, total: Some(200) }.fraction(), Some(0.25));
        assert_eq!(Progress { downloaded: 300, total: Some(200) }.fraction(), Some(1.0));
        assert_eq!(Progress { downloaded: 10, total: None }.fraction(), None);
        // A zero total would divide by zero; it must read as "unknown".
        assert_eq!(Progress { downloaded: 0, total: Some(0) }.fraction(), None);
    }

    #[test]
    fn tokens_are_only_ever_sent_to_the_hub() {
        assert!(is_hub_url("https://huggingface.co/a/b/resolve/main/f"));
        // A look-alike host must not receive the bearer header.
        assert!(!is_hub_url("https://huggingface.co.evil.example/a"));
        assert!(!is_hub_url("http://huggingface.co/a"));
        assert!(!is_hub_url("https://example.com/weights.gguf"));
        // The env-reading wrapper inherits the same gate.
        assert_eq!(hf_token("https://example.com/weights.gguf"), None);
    }

    #[test]
    fn blank_tokens_fall_through_to_the_next_candidate() {
        let candidates = ["   ".to_owned(), "hf_real".to_owned()];
        assert_eq!(first_usable_token(candidates), Some("hf_real".to_owned()));
        assert_eq!(first_usable_token([String::new()]), None);
        assert_eq!(first_usable_token([]), None);
    }

    #[test]
    fn both_token_spellings_are_consulted() {
        assert_eq!(HF_TOKEN_VARS, ["HF_TOKEN", "HUGGING_FACE_HUB_TOKEN"]);
    }

    #[test]
    fn resumed_prefix_is_folded_into_the_digest() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("blob.part");
        let head = vec![0x11_u8; (1 << 20) + 7];
        std::fs::write(&path, &head).unwrap();

        let mut hasher = Hasher::new();
        hash_existing(&path, &mut hasher).unwrap();
        let tail = b"tail-bytes";
        hasher.update(tail);

        let mut whole = head.clone();
        whole.extend_from_slice(tail);
        assert_eq!(hasher.finish_hex(), digest::sha256_hex(&whole));
    }

    #[test]
    fn outcome_reports_whether_bytes_moved() {
        let dir = tempfile::tempdir().unwrap();
        let store = ModelStore::with_root(dir.path()).unwrap();
        let external = dir.path().join("x.gguf");
        std::fs::write(&external, b"weights").unwrap();
        let entry = store.adopt_local(&external).unwrap();

        let adopted = PullOutcome::Adopted(entry.clone());
        assert!(!adopted.transferred());
        assert_eq!(adopted.model().bytes, 7);
        assert!(PullOutcome::Downloaded(entry).transferred());
    }

    #[tokio::test]
    async fn local_reference_is_adopted_without_network() {
        let dir = tempfile::tempdir().unwrap();
        let store = ModelStore::with_root(dir.path().join("models")).unwrap();
        let external = dir.path().join("outside.gguf");
        std::fs::write(&external, b"already here").unwrap();

        let downloader = Downloader::new().unwrap();
        let reference = ModelRef::parse(&format!("file:{}", external.display())).unwrap();
        let outcome = downloader
            .pull(&store, &reference, &PullOptions::default(), |_| {})
            .await
            .unwrap();

        assert!(matches!(outcome, PullOutcome::Adopted(_)));
        assert!(!outcome.transferred());
        assert_eq!(outcome.model().sha256, digest::sha256_hex(b"already here"));
        assert!(external.is_file(), "adoption must not move the file");
    }

    #[tokio::test]
    async fn client_construction_installs_the_crypto_provider() {
        // Building a rustls-no-provider reqwest client panics if no provider
        // is installed; reaching the assert proves `ensure_crypto_provider`
        // ran. Two constructions also prove the install is idempotent.
        assert!(Downloader::new().is_ok());
        assert!(Downloader::new().is_ok());
    }
}
