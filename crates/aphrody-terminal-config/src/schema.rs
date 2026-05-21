// SPDX-License-Identifier: Apache-2.0
//! Strict-schema types for LLM / browser / MCP configuration.
//!
//! These structs sit alongside the legacy [`crate::TerminalConfig`] surface
//! and expose **schema-validated** views of the bits a terminal session needs
//! at boot:
//!
//! - [`LlmConfig`] — provider + model + API-key env var + temperature.  The provider field is a
//!   closed enum: `anthropic | gemini | antigravity`.  Any other value is rejected at deserialise
//!   time, matching the project-wide provider whitelist (see top-level docs).
//! - [`McpServerSpec`] — normalised MCP server entry (transport-tagged) used by the terminal-llm
//!   probe loop.  This is the *post-shim* shape: every `mcpServers` entry coming out of
//! - [`BrowserConfig`] — agent-browser / Edge headless fallback wiring.

use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ── LlmProvider ───────────────────────────────────────────────────────────────

/// Closed enum of supported LLM providers.
///
/// Adding a new provider here is intentional: aphrody only ships first-party
/// adapters for these three (see top-level project docs for the rationale).
/// Any other identifier in `~/.aphrody/terminal.json` is rejected with
/// [`crate::ConfigError::Validation`] at load time.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LlmProvider {
    /// Anthropic Claude (claude.ai/api).
    Anthropic,
    /// Google Gemini (Gemini API / Vertex AI).
    Gemini,
    /// Vertex AI Antigravity.
    Antigravity,
}

// ── LlmConfig ─────────────────────────────────────────────────────────────────

/// LLM client configuration block.
///
/// All fields are required to keep the schema tight; sensible defaults belong
/// in the calling code, not in serialised user config.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct LlmConfig {
    /// Provider identifier (closed enum, see [`LlmProvider`]).
    pub provider: LlmProvider,
    /// Model id understood by the chosen provider (e.g. `"claude-opus-4-7"`).
    pub model: String,
    /// Environment variable holding the API key (never the raw key).
    pub api_key_env: String,
    /// Default sampling temperature (clamped 0.0..=2.0 by the runtime).
    pub default_temperature: f32,
}

// ── McpTransport / McpServerSpec ──────────────────────────────────────────────

/// Transport tag for a normalised MCP server entry.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "kebab-case", deny_unknown_fields)]
pub enum McpTransport {
    /// Spawn a subprocess and speak JSON-RPC over stdin/stdout.
    Stdio {
        /// Command to execute.
        command: String,
        /// Command arguments.
        #[serde(default)]
        args: Vec<String>,
        /// Environment overrides.
        #[serde(default)]
        env: HashMap<String, String>,
    },
    /// Connect to an HTTP MCP endpoint (streaming or request/response).
    Http {
        /// Endpoint URL.
        url: String,
        /// Request headers (may contain `${ENV_VAR}` placeholders).
        #[serde(default)]
        headers: HashMap<String, String>,
    },
}

/// Normalised MCP server specification (post-shim).
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct McpServerSpec {
    /// Logical name (the key in the `mcpServers` map).
    pub name: String,
    /// Transport-specific wiring.
    pub transport: McpTransport,
}

// ── BrowserConfig ─────────────────────────────────────────────────────────────

/// Browser-bridge wiring (agent-browser / Edge headless).
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BrowserConfig {
    /// Preferred engine identifier.  Free-form (`"agent-browser"`,
    /// `"edge-headless"`) so the consumer can dispatch without locking the
    /// schema to today's set.
    pub engine: String,
    /// CDP endpoint URL (`http://127.0.0.1:9222` style) — optional because
    /// agent-browser spawns its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cdp_endpoint: Option<String>,
}
