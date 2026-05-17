// SPDX-License-Identifier: Apache-2.0
//! # aphrody-terminal-browser
//!
//! LLM-to-DOM bridge layer for `aphrody-terminal`.
//!
//! This crate exposes:
//!
//! - [`BrowserBackend`] — the trait every backend implements.
//! - [`BrowserError`] — typed error enum covering all failure modes.
//! - [`Active`] — enum over the three concrete backends with auto-probe.
//! - [`osc::parse_aphrody_browser_osc`] — parse raw OSC byte slices into
//!   [`proto::BrowserRequest`] values, ready to dispatch to the active backend.
//!
//! ## Backends
//!
//! | Variant | Binary | Capability |
//! |---|---|---|
//! | `Active::Bxc` | `bxc` | Fast scrape, static + light JS, no GPU |
//! | `Active::AgentBrowser` | `agent-browser` | Full Chromium, CDP, WebGPU |
//! | `Active::Edge` | `msedge` | DOM snapshot only (Windows fallback) |
//!
//! ## OSC dispatch
//!
//! ```text
//! \x1b]aphrody-browser-nav;https://example.com\x07
//! ```
//!
//! Parse with [`osc::parse_aphrody_browser_osc`], then call the matching
//! method on the [`Active`] backend via [`Active::dispatch`].

#![deny(unsafe_code)]

pub mod backend;
pub mod osc;
pub mod proto;

pub use proto::{BrowserRequest, BrowserResponse, RecordState, ScreenshotArea};

use backend::{agent_browser::AgentBrowserBackend, bxc::BxcBackend, edge::EdgeBackend};
use thiserror::Error;
use tracing::{info, instrument};

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// All errors that can arise from browser backend operations.
#[derive(Debug, Error)]
pub enum BrowserError {
    /// The backend binary could not be spawned.
    #[error("backend `{backend}` failed to spawn: {source}")]
    SpawnFailed {
        backend: String,
        #[source]
        source: std::io::Error,
    },

    /// The backend process exited unexpectedly.
    #[error("backend `{backend}` exited with code {code:?}")]
    ProcessExit {
        backend: String,
        code: Option<i32>,
    },

    /// The backend returned a malformed or unexpected response.
    #[error("backend `{backend}` protocol error: {message}")]
    ProtocolError { backend: String, message: String },

    /// The requested operation is not supported by this backend.
    #[error("backend `{backend}` does not support `{method}`: {reason}")]
    Unsupported {
        method: String,
        backend: String,
        reason: String,
    },

    /// The operation timed out.
    #[error("backend `{backend}` timed out on `{method}` after {elapsed_ms} ms")]
    Timeout {
        method: String,
        backend: String,
        elapsed_ms: u64,
    },

    /// An I/O error occurred communicating with the backend process.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// A JSON serialization/deserialization error occurred.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

// ---------------------------------------------------------------------------
// BrowserBackend trait
// ---------------------------------------------------------------------------

/// Trait implemented by every browser automation backend.
///
/// `async fn` in traits is supported by the project's nightly toolchain
/// (`nightly-2026-05-17`).  No `async-trait` crate is required.
#[allow(async_fn_in_trait)]
pub trait BrowserBackend: Send + Sync {
    /// Human-readable backend identifier (e.g. `"bxc"`, `"agent-browser"`).
    fn name(&self) -> &'static str;

    /// Navigate to `url`.  Returns the final URL after any redirects.
    async fn navigate(&mut self, url: &str) -> Result<BrowserResponse, BrowserError>;

    /// Evaluate a JavaScript expression in the current page context.
    /// `src` is the raw JS source string (decoded by the caller from base64
    /// if it arrived via OSC).
    async fn eval_js(&mut self, src: &str) -> Result<BrowserResponse, BrowserError>;

    /// Run a CSS selector query against the current DOM.
    async fn query_selector(&mut self, sel: &str) -> Result<BrowserResponse, BrowserError>;

    /// Capture a screenshot of the given [`ScreenshotArea`].
    async fn screenshot(&mut self, area: ScreenshotArea) -> Result<BrowserResponse, BrowserError>;

    /// Extract structured data from the page according to `schema`.
    async fn extract(
        &mut self,
        schema: &serde_json::Value,
    ) -> Result<BrowserResponse, BrowserError>;

    /// Install a request-interception rule.
    async fn intercept(
        &mut self,
        rule: &serde_json::Value,
    ) -> Result<BrowserResponse, BrowserError>;

    /// Start or stop a session recording identified by `id`.
    async fn record(&mut self, id: &str, state: RecordState)
        -> Result<BrowserResponse, BrowserError>;
}

// ---------------------------------------------------------------------------
// Active — enum over the three backend boxes
// ---------------------------------------------------------------------------

/// The currently active browser backend.
///
/// Wraps one of the three concrete backend types.  Use [`Active::probe`] to
/// auto-select based on binary availability, or construct a specific variant
/// directly.
pub enum Active {
    Bxc(Box<BxcBackend>),
    AgentBrowser(Box<AgentBrowserBackend>),
    Edge(Box<EdgeBackend>),
}

impl Active {
    /// Probe binary availability and return the best available backend.
    ///
    /// Priority order:
    /// 1. `bxc` — fast, lightweight, no GPU dependency.
    /// 2. `agent-browser` — full Chromium, heavier.
    /// 3. `msedge --headless` — Windows-only DOM snapshot fallback.
    ///
    /// Returns an error only if none of the three backends can be started.
    #[instrument]
    pub async fn probe() -> Result<Self, BrowserError> {
        // 1. Try bxc
        match BxcBackend::spawn().await {
            Ok(b) => {
                info!(backend = "bxc", "selected browser backend");
                return Ok(Active::Bxc(Box::new(b)));
            }
            Err(e) => {
                tracing::debug!(error = %e, "bxc not available");
            }
        }

        // 2. Try agent-browser
        match AgentBrowserBackend::spawn().await {
            Ok(b) => {
                info!(backend = "agent-browser", "selected browser backend");
                return Ok(Active::AgentBrowser(Box::new(b)));
            }
            Err(e) => {
                tracing::debug!(error = %e, "agent-browser not available");
            }
        }

        // 3. Try msedge fallback
        match EdgeBackend::spawn().await {
            Ok(b) => {
                info!(backend = "edge", "selected browser backend (fallback)");
                return Ok(Active::Edge(Box::new(b)));
            }
            Err(e) => {
                tracing::debug!(error = %e, "edge not available");
            }
        }

        Err(BrowserError::Unsupported {
            method: "probe".into(),
            backend: "none".into(),
            reason: "no browser backend available: install bxc, agent-browser, \
                     or msedge (Windows) and ensure the binary is on PATH"
                .into(),
        })
    }

    /// Return the name of the active backend.
    pub fn name(&self) -> &'static str {
        match self {
            Active::Bxc(b) => b.name(),
            Active::AgentBrowser(b) => b.name(),
            Active::Edge(b) => b.name(),
        }
    }

    /// Dispatch a parsed [`BrowserRequest`] to the active backend.
    pub async fn dispatch(
        &mut self,
        req: BrowserRequest,
    ) -> Result<BrowserResponse, BrowserError> {
        match req {
            BrowserRequest::Nav { url } => self.navigate(&url).await,
            BrowserRequest::Eval { src } => self.eval_js(&src).await,
            BrowserRequest::Dom { selector } => self.query_selector(&selector).await,
            BrowserRequest::Screenshot { area } => self.screenshot(area).await,
            BrowserRequest::Intercept { rule } => self.intercept(&rule).await,
            BrowserRequest::Extract { schema } => self.extract(&schema).await,
            BrowserRequest::Record { id, state } => self.record(&id, state).await,
        }
    }
}

// ---------------------------------------------------------------------------
// Forward BrowserBackend methods through Active
// ---------------------------------------------------------------------------

impl BrowserBackend for Active {
    fn name(&self) -> &'static str {
        Active::name(self)
    }

    async fn navigate(&mut self, url: &str) -> Result<BrowserResponse, BrowserError> {
        match self {
            Active::Bxc(b) => b.navigate(url).await,
            Active::AgentBrowser(b) => b.navigate(url).await,
            Active::Edge(b) => b.navigate(url).await,
        }
    }

    async fn eval_js(&mut self, src: &str) -> Result<BrowserResponse, BrowserError> {
        match self {
            Active::Bxc(b) => b.eval_js(src).await,
            Active::AgentBrowser(b) => b.eval_js(src).await,
            Active::Edge(b) => b.eval_js(src).await,
        }
    }

    async fn query_selector(&mut self, sel: &str) -> Result<BrowserResponse, BrowserError> {
        match self {
            Active::Bxc(b) => b.query_selector(sel).await,
            Active::AgentBrowser(b) => b.query_selector(sel).await,
            Active::Edge(b) => b.query_selector(sel).await,
        }
    }

    async fn screenshot(&mut self, area: ScreenshotArea) -> Result<BrowserResponse, BrowserError> {
        match self {
            Active::Bxc(b) => b.screenshot(area).await,
            Active::AgentBrowser(b) => b.screenshot(area).await,
            Active::Edge(b) => b.screenshot(area).await,
        }
    }

    async fn extract(
        &mut self,
        schema: &serde_json::Value,
    ) -> Result<BrowserResponse, BrowserError> {
        match self {
            Active::Bxc(b) => b.extract(schema).await,
            Active::AgentBrowser(b) => b.extract(schema).await,
            Active::Edge(b) => b.extract(schema).await,
        }
    }

    async fn intercept(
        &mut self,
        rule: &serde_json::Value,
    ) -> Result<BrowserResponse, BrowserError> {
        match self {
            Active::Bxc(b) => b.intercept(rule).await,
            Active::AgentBrowser(b) => b.intercept(rule).await,
            Active::Edge(b) => b.intercept(rule).await,
        }
    }

    async fn record(
        &mut self,
        id: &str,
        state: RecordState,
    ) -> Result<BrowserResponse, BrowserError> {
        match self {
            Active::Bxc(b) => b.record(id, state).await,
            Active::AgentBrowser(b) => b.record(id, state).await,
            Active::Edge(b) => b.record(id, state).await,
        }
    }
}
