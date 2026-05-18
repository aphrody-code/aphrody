// SPDX-License-Identifier: Apache-2.0
//! Wire types for the aphrody-browser-* OSC protocol.
//!
//! Every variant maps 1-to-1 to a `BrowserBackend` method.  The payloads are
//! serializable so callers can log / replay requests without re-parsing the
//! raw OSC byte stream.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Screenshot area
// ---------------------------------------------------------------------------

/// Which portion of the page to capture.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScreenshotArea {
    /// The visible viewport.
    Viewport,
    /// A specific element identified by a CSS selector.
    Element { selector: String },
    /// The full scrollable page.
    Fullpage,
}

// ---------------------------------------------------------------------------
// Record state
// ---------------------------------------------------------------------------

/// Whether to start or stop a session recording.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecordState {
    Start,
    Stop,
}

// ---------------------------------------------------------------------------
// Inbound requests (parsed from OSC sequences)
// ---------------------------------------------------------------------------

/// A fully-parsed `aphrody-browser-*` OSC request, ready to dispatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum BrowserRequest {
    /// `aphrody-browser-nav;<url>`
    Nav { url: String },
    /// `aphrody-browser-eval;<base64-js>`
    Eval { src: String },
    /// `aphrody-browser-dom;<base64-selector>`
    Dom { selector: String },
    /// `aphrody-browser-screenshot;<area>`
    Screenshot { area: ScreenshotArea },
    /// `aphrody-browser-intercept;<base64-json-rule>`
    Intercept { rule: serde_json::Value },
    /// `aphrody-browser-extract;<base64-json-schema>`
    Extract { schema: serde_json::Value },
    /// `aphrody-browser-record;<id>;<start|stop>`
    Record { id: String, state: RecordState },
}

// ---------------------------------------------------------------------------
// Outbound responses
// ---------------------------------------------------------------------------

/// Structured response from any `BrowserBackend` operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BrowserResponse {
    /// Navigation completed; `url` is the final (post-redirect) URL.
    Navigated { url: String },
    /// JavaScript evaluation result (JSON-serializable value).
    EvalResult { value: serde_json::Value },
    /// DOM query result as a JSON tree.
    DomResult { nodes: serde_json::Value },
    /// PNG screenshot bytes, base64-encoded.
    Screenshot { area: ScreenshotArea, png_b64: String },
    /// Intercept rule installed; returns the rule ID assigned by the backend.
    InterceptInstalled { rule_id: String },
    /// Extracted structured data matching the caller-supplied schema.
    Extracted { data: serde_json::Value },
    /// Recording state transition acknowledged.
    RecordAck { id: String, state: RecordState },
}
