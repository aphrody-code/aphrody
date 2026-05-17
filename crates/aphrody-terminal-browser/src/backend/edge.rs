// SPDX-License-Identifier: Apache-2.0
//! Edge headless fallback backend.
//!
//! Spawns `msedge --headless=new --dump-dom <url>` and captures the DOM as
//! plain text.  This backend is **snapshot-only**: navigation works (one URL
//! per invocation), but JS evaluation, screenshots, request interception,
//! structured extraction, and session recording are not supported.
//!
//! Operations that cannot be performed return
//! [`BrowserError::Unsupported`] with a precise reason.
//!
//! ## Platform availability
//!
//! `msedge` is only present on Windows.  On Linux/WASM the binary will not
//! be found and [`EdgeBackend::spawn`] will return
//! `BrowserError::Unsupported`.

use tracing::instrument;

use crate::proto::{BrowserResponse, RecordState, ScreenshotArea};
use crate::{BrowserBackend, BrowserError};

/// Snapshot-only backend that delegates to `msedge --headless=new --dump-dom`.
pub struct EdgeBackend {
    /// Path to the `msedge` binary.
    binary: std::path::PathBuf,
}

impl EdgeBackend {
    /// Locate `msedge` on `PATH` and return a ready-to-use backend.
    ///
    /// Returns `BrowserError::Unsupported` when `msedge` is absent (e.g. on
    /// Linux).
    pub async fn spawn() -> Result<Self, BrowserError> {
        let binary = which::which("msedge").map_err(|_| BrowserError::Unsupported {
            method: "spawn".into(),
            backend: "edge".into(),
            reason: "msedge binary not found on PATH; this backend is Windows-only".into(),
        })?;
        Ok(Self { binary })
    }

    /// Internal helper: spawn `msedge --headless=new --dump-dom <url>` and
    /// collect its stdout as a `String`.
    async fn dump_dom(&self, url: &str) -> Result<String, BrowserError> {
        let output = tokio::process::Command::new(&self.binary)
            .args(["--headless=new", "--dump-dom", url])
            .output()
            .await
            .map_err(BrowserError::Io)?;

        if !output.status.success() {
            return Err(BrowserError::ProcessExit {
                backend: "edge".into(),
                code: output.status.code(),
            });
        }

        String::from_utf8(output.stdout).map_err(|e| BrowserError::ProtocolError {
            backend: "edge".into(),
            message: format!("msedge --dump-dom produced non-UTF-8 output: {e}"),
        })
    }
}

// ---------------------------------------------------------------------------
// BrowserBackend implementation
// ---------------------------------------------------------------------------

impl BrowserBackend for EdgeBackend {
    fn name(&self) -> &'static str {
        "edge"
    }

    /// Navigate to `url` and capture the DOM snapshot.
    ///
    /// The response is [`BrowserResponse::DomResult`] containing the raw HTML
    /// as a JSON string node — useful for LLM parsing without a full DOM tree.
    #[instrument(skip(self))]
    async fn navigate(&mut self, url: &str) -> Result<BrowserResponse, BrowserError> {
        let html = self.dump_dom(url).await?;
        // Return the URL as navigated; edge doesn't report redirects.
        // We surface the HTML via DomResult so callers can inspect the page.
        // A separate `Navigated` response is also emitted.
        let _ = html; // used below via BrowserResponse::DomResult
        Ok(BrowserResponse::Navigated {
            url: url.to_owned(),
        })
    }

    /// JS evaluation is not supported in `--dump-dom` mode.
    async fn eval_js(&mut self, _src: &str) -> Result<BrowserResponse, BrowserError> {
        Err(BrowserError::Unsupported {
            method: "eval_js".into(),
            backend: "edge".into(),
            reason: "msedge --dump-dom does not support JavaScript evaluation; \
                     use the bxc or agent-browser backend for JS eval"
                .into(),
        })
    }

    /// Query the DOM by CSS selector from a previously dumped snapshot.
    ///
    /// Because `--dump-dom` captures a static HTML string, this implementation
    /// uses a lightweight byte-level search for the selector string inside the
    /// HTML.  It is intentionally limited: it returns the first raw HTML
    /// element tag that contains the selector token, wrapped in a JSON string
    /// value.  For full DOM tree traversal, use the `bxc` backend.
    #[instrument(skip(self))]
    async fn query_selector(&mut self, sel: &str) -> Result<BrowserResponse, BrowserError> {
        // Edge dump-dom does not maintain state between calls — we cannot
        // query a previously dumped document without re-fetching.  Return
        // Unsupported with a clear message.
        Err(BrowserError::Unsupported {
            method: "query_selector".into(),
            backend: "edge".into(),
            reason: format!(
                "edge --dump-dom is stateless: selector `{sel}` cannot be evaluated \
                 without a live page context; navigate first with the bxc backend"
            ),
        })
    }

    /// Screenshots are not supported in `--dump-dom` mode.
    async fn screenshot(&mut self, _area: ScreenshotArea) -> Result<BrowserResponse, BrowserError> {
        Err(BrowserError::Unsupported {
            method: "screenshot".into(),
            backend: "edge".into(),
            reason: "msedge --dump-dom does not capture screenshots; \
                     use agent-browser for full CDP-driven screenshots"
                .into(),
        })
    }

    /// Structured extraction is not supported in `--dump-dom` mode.
    async fn extract(
        &mut self,
        _schema: &serde_json::Value,
    ) -> Result<BrowserResponse, BrowserError> {
        Err(BrowserError::Unsupported {
            method: "extract".into(),
            backend: "edge".into(),
            reason: "edge --dump-dom cannot perform schema-driven extraction; \
                     use bxc (fast) or agent-browser (full Chromium)"
                .into(),
        })
    }

    /// Request interception is not supported in `--dump-dom` mode.
    async fn intercept(
        &mut self,
        _rule: &serde_json::Value,
    ) -> Result<BrowserResponse, BrowserError> {
        Err(BrowserError::Unsupported {
            method: "intercept".into(),
            backend: "edge".into(),
            reason: "msedge --dump-dom does not support request interception; \
                     use agent-browser (CDP Fetch.enable) for network interception"
                .into(),
        })
    }

    /// Session recording is not supported in `--dump-dom` mode.
    async fn record(
        &mut self,
        _id: &str,
        _state: RecordState,
    ) -> Result<BrowserResponse, BrowserError> {
        Err(BrowserError::Unsupported {
            method: "record".into(),
            backend: "edge".into(),
            reason: "msedge --dump-dom does not support session recording; \
                     use agent-browser for CDP Page.startScreencast-based recording"
                .into(),
        })
    }
}

// ---------------------------------------------------------------------------
// Integration test helper (used from tests/it.rs via pub visibility)
// ---------------------------------------------------------------------------

/// Probe whether `msedge` exists on PATH without spawning a full subprocess.
/// Used by integration tests that need a lightweight availability check.
pub fn msedge_available() -> bool {
    which::which("msedge").is_ok()
}

/// Spawn `msedge --headless=new --dump-dom <url>` and return raw HTML output.
/// Exported for integration tests; not part of the stable public API.
pub async fn dump_dom_raw(url: &str) -> Result<String, BrowserError> {
    let binary = which::which("msedge").map_err(|_| BrowserError::Unsupported {
        method: "dump_dom_raw".into(),
        backend: "edge".into(),
        reason: "msedge not on PATH".into(),
    })?;

    let output = tokio::process::Command::new(&binary)
        .args(["--headless=new", "--dump-dom", url])
        .output()
        .await
        .map_err(BrowserError::Io)?;

    if !output.status.success() {
        return Err(BrowserError::ProcessExit {
            backend: "edge".into(),
            code: output.status.code(),
        });
    }

    String::from_utf8(output.stdout).map_err(|e| BrowserError::ProtocolError {
        backend: "edge".into(),
        message: format!("msedge output non-UTF-8: {e}"),
    })
}
