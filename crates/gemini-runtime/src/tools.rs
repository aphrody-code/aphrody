// SPDX-License-Identifier: Apache-2.0
//! Uniform [`Tool`] trait for runtime-invocable tools.
//!
//! The gemini runtime exposes three first-class tool sources:
//!
//! - [`WebSearchTool`] — built-in Gemini web-search via [`crate::web_search`].
//! - [`McpProxyTool`] — wraps a single tool advertised by an MCP server.
//! - User-defined tools registered through [`ToolRegistry::register`].
//!
//! All three share the same [`Tool`] surface so the upstream [`crate::GeminiRuntime`]
//! turn loop can dispatch tool calls polymorphically (boxed trait objects under
//! `Arc` so they can be cheaply cloned across turns).
//!
//! ```no_run
//! use std::sync::Arc;
//! use gemini_runtime::tools::{Tool, ToolRegistry, WebSearchTool};
//! use gemini_runtime::WebSearchConfig;
//!
//! # async fn run() {
//! let mut reg = ToolRegistry::new();
//! reg.register(Arc::new(WebSearchTool::new(
//!     reqwest::Client::new(),
//!     WebSearchConfig::with_api_key("AIza…"),
//! )));
//! let res = reg
//!     .invoke("web_search", serde_json::json!({"query": "io_uring"}))
//!     .await
//!     .expect("ok");
//! println!("{res}");
//! # }
//! ```

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use tokio::sync::Mutex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::{
    mcp_client::{McpClient, McpError},
    web_search::{WebSearchConfig, WebSearchError, web_search},
};

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors returned by tool invocations.
#[derive(Debug, Error)]
pub enum ToolError {
    /// The requested tool is not registered.
    #[error("tool `{0}` is not registered")]
    NotFound(String),

    /// Tool input arguments did not match the expected schema.
    #[error("tool `{name}` rejected arguments: {reason}")]
    BadArgs {
        /// Name of the tool that rejected the arguments.
        name: String,
        /// Human-readable reason.
        reason: String,
    },

    /// The web-search backend failed.
    #[error("web_search failed: {0}")]
    WebSearch(#[from] WebSearchError),

    /// The MCP backend failed.
    #[error("MCP backend failed: {0}")]
    Mcp(#[from] McpError),

    /// Generic tool implementation error (free-form message).
    #[error("tool error: {0}")]
    Other(String),
}

// ---------------------------------------------------------------------------
// Tool trait
// ---------------------------------------------------------------------------

/// Description of a tool's input schema, advertised to upstream consumers
/// (the gemini turn loop, the `aphrody mcp list` printer, …).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolDescriptor {
    /// Tool identifier — must be unique within a [`ToolRegistry`].
    pub name: String,
    /// One-sentence human description.
    pub description: String,
    /// JSON-Schema for the `arguments` object accepted by [`Tool::invoke`].
    pub input_schema: serde_json::Value,
}

/// Polymorphic tool surface invoked by the gemini turn loop.
#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool identifier (must match `descriptor().name`).
    fn name(&self) -> &str;

    /// Machine-readable descriptor with input schema.
    fn descriptor(&self) -> ToolDescriptor;

    /// Run the tool against `args` and return its JSON result.
    ///
    /// # Errors
    ///
    /// Any [`ToolError`] variant — backends are expected to translate their
    /// own error types via the `#[from]` conversions provided.
    async fn invoke(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError>;
}

// ---------------------------------------------------------------------------
// WebSearchTool
// ---------------------------------------------------------------------------

/// Built-in tool wrapping [`crate::web_search`]. Accepts `{ "query": "..." }`
/// and returns the [`crate::SearchResults`] JSON.
pub struct WebSearchTool {
    client: reqwest::Client,
    cfg: WebSearchConfig,
}

impl WebSearchTool {
    /// Construct a new tool. Re-uses the provided [`reqwest::Client`] across
    /// invocations (HTTP connection pool reuse).
    #[must_use]
    pub fn new(client: reqwest::Client, cfg: WebSearchConfig) -> Self {
        Self { client, cfg }
    }

    /// Static descriptor exposed by [`Tool::descriptor`].
    #[must_use]
    pub fn static_descriptor() -> ToolDescriptor {
        ToolDescriptor {
            name: "web_search".to_owned(),
            description: "Search the public web via Gemini grounding (google_search).".to_owned(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Free-form web search query."
                    }
                },
                "required": ["query"]
            }),
        }
    }
}

#[async_trait]
impl Tool for WebSearchTool {
    fn name(&self) -> &str {
        "web_search"
    }

    fn descriptor(&self) -> ToolDescriptor {
        Self::static_descriptor()
    }

    async fn invoke(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        let query = args
            .get("query")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| ToolError::BadArgs {
                name: "web_search".to_owned(),
                reason: "missing required field `query` (string)".to_owned(),
            })?;
        let result = web_search(&self.client, &self.cfg, query).await?;
        serde_json::to_value(&result).map_err(|e| ToolError::Other(e.to_string()))
    }
}

// ---------------------------------------------------------------------------
// McpProxyTool
// ---------------------------------------------------------------------------

/// Wraps a single tool advertised by an MCP server. Tool invocations are
/// proxied verbatim through `tools/call` on the shared [`McpClient`].
///
/// The client is held under `Arc<Mutex<…>>` because [`McpClient`] is not
/// `Clone` (it owns a `Child` process / `reqwest::Client`) and the gemini
/// runtime may invoke multiple tools concurrently.
pub struct McpProxyTool {
    client: Arc<Mutex<McpClient>>,
    descriptor: ToolDescriptor,
}

impl McpProxyTool {
    /// Build a new proxy tool bound to `mcp_tool_name` on the supplied
    /// shared [`McpClient`].
    #[must_use]
    pub fn new(client: Arc<Mutex<McpClient>>, descriptor: ToolDescriptor) -> Self {
        Self { client, descriptor }
    }
}

#[async_trait]
impl Tool for McpProxyTool {
    fn name(&self) -> &str {
        &self.descriptor.name
    }

    fn descriptor(&self) -> ToolDescriptor {
        self.descriptor.clone()
    }

    async fn invoke(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
        // `tokio::sync::Mutex` is required because the lock is held across
        // an `.await` point: the stdio transport tracks a per-connection
        // monotonic `next_id` and must read responses serially.
        let mut guard = self.client.lock().await;
        let res = guard.call_tool(&self.descriptor.name, args).await?;
        Ok(res)
    }
}

// ---------------------------------------------------------------------------
// ToolRegistry
// ---------------------------------------------------------------------------

/// In-memory registry of available [`Tool`] implementations.
///
/// Used by the gemini turn loop to dispatch tool calls emitted by the model
/// and by the `aphrody mcp call` CLI subcommand.
#[derive(Default, Clone)]
pub struct ToolRegistry {
    inner: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    /// Create a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a tool. Returns the previous entry if one existed under the
    /// same name (caller can decide to log a warning).
    pub fn register(&mut self, tool: Arc<dyn Tool>) -> Option<Arc<dyn Tool>> {
        let key = tool.name().to_owned();
        self.inner.insert(key, tool)
    }

    /// Return the tool registered under `name`, if any.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.inner.get(name).cloned()
    }

    /// List all registered tools' descriptors.
    #[must_use]
    pub fn list(&self) -> Vec<ToolDescriptor> {
        let mut out: Vec<ToolDescriptor> = self.inner.values().map(|t| t.descriptor()).collect();
        out.sort_by(|a, b| a.name.cmp(&b.name));
        out
    }

    /// Invoke a tool by name.
    ///
    /// # Errors
    ///
    /// - [`ToolError::NotFound`] — no tool registered under `name`.
    /// - Whatever the tool itself returns.
    pub async fn invoke(
        &self,
        name: &str,
        args: serde_json::Value,
    ) -> Result<serde_json::Value, ToolError> {
        let tool = self.get(name).ok_or_else(|| ToolError::NotFound(name.to_owned()))?;
        tool.invoke(args).await
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    struct EchoTool;

    #[async_trait]
    impl Tool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }

        fn descriptor(&self) -> ToolDescriptor {
            ToolDescriptor {
                name: "echo".to_owned(),
                description: "Return arguments unchanged".to_owned(),
                input_schema: serde_json::json!({"type": "object"}),
            }
        }

        async fn invoke(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
            Ok(args)
        }
    }

    #[test]
    fn web_search_descriptor_shape() {
        let d = WebSearchTool::static_descriptor();
        assert_eq!(d.name, "web_search");
        assert!(d.description.to_lowercase().contains("search"));
        let schema = &d.input_schema;
        assert_eq!(schema["type"], "object");
        assert_eq!(schema["properties"]["query"]["type"], "string");
        assert_eq!(schema["required"][0], "query");
    }

    #[tokio::test]
    async fn registry_invokes_registered_tool() {
        let mut reg = ToolRegistry::new();
        reg.register(Arc::new(EchoTool));
        let v = reg.invoke("echo", serde_json::json!({"x": 1})).await.expect("ok");
        assert_eq!(v["x"], 1);
    }

    #[tokio::test]
    async fn registry_not_found_returns_error() {
        let reg = ToolRegistry::new();
        let err = reg.invoke("missing", serde_json::Value::Null).await.unwrap_err();
        assert!(matches!(err, ToolError::NotFound(_)));
    }

    #[test]
    fn registry_list_is_sorted() {
        let mut reg = ToolRegistry::new();
        reg.register(Arc::new(EchoTool));
        reg.register(Arc::new(EchoTool)); // same name → dedup
        let list = reg.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].name, "echo");
    }

    #[tokio::test]
    async fn web_search_tool_bad_args_missing_query() {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let tool = WebSearchTool::new(
            reqwest::Client::new(),
            WebSearchConfig::with_api_key("AIza-test"),
        );
        let err = tool.invoke(serde_json::json!({})).await.unwrap_err();
        match err {
            ToolError::BadArgs { name, reason } => {
                assert_eq!(name, "web_search");
                assert!(reason.contains("query"));
            },
            other => panic!("expected BadArgs, got {other:?}"),
        }
    }
}
