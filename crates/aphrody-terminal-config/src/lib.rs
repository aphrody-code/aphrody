// SPDX-License-Identifier: Apache-2.0
//! Strict-schema configuration loader for `aphrody-terminal`.
//!
//! # Surface
//!
//! - [`TerminalConfig`] — top-level strongly-typed configuration struct, with `denyUnknownFields`
//!   so any unrecognised key in the source JSON produces a [`ConfigError::Validation`] error.
//! - [`load_default`] — resolve `~/.aphrody/terminal.json`, falling back to
//!   [`TerminalConfig::default`] when the file is missing.
//! - [`load_from_path`] — load from an explicit path with the same missing-file → default
//!   behaviour.
//! - [`validate`] — parse + validate an arbitrary JSON string.
//! - [`schema_json`] — emit the generated JSON Schema as a UTF-8 string.
//!
//! # Layering
//!
//! Higher-level callers typically:
//!
//! 1. [`load_default`] to get the base [`TerminalConfig`].
//! 2. [`shims::import_claude_json`] / [`shims::import_mcp_json`] / [`shims::import_settings_json`]
//!    to gather adjacent tool config.
//! 3. [`merge::merge`] to fold them into a single effective config.
//!
//! # Strictness
//!
//! All public structs derive `Serialize + Deserialize + JsonSchema` and use
//! `#[serde(deny_unknown_fields)]` at the root.  Sub-structs are tolerant by
//! default so the schema can grow without breaking older config files.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub mod error;
pub mod merge;
pub mod shims;

pub use error::{ConfigError, Result};

use crate::shims::McpServerEntry;

// ── LlmPaneConfig ─────────────────────────────────────────────────────────────

/// Which LLM-side panes the terminal should render.
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LlmPaneConfig {
    /// Show the sub-agents pane.
    #[serde(default = "default_true")]
    pub sub_agent: bool,
    /// Show the MCP server status pane.
    #[serde(default = "default_true")]
    pub mcp: bool,
    /// Show the hook trace pane.
    #[serde(default = "default_true")]
    pub hook: bool,
    /// Show the skills pane.
    #[serde(default = "default_true")]
    pub skill: bool,
}

impl Default for LlmPaneConfig {
    fn default() -> Self {
        Self { sub_agent: true, mcp: true, hook: true, skill: true }
    }
}

const fn default_true() -> bool {
    true
}

// ── VoiceConfig ───────────────────────────────────────────────────────────────

/// Voice (TTS/STT) settings.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VoiceConfig {
    /// Whether voice features are enabled.
    #[serde(default)]
    pub enabled: bool,
    /// TTS provider identifier (e.g. `"elevenlabs"`).
    #[serde(default)]
    pub provider: String,
}

// ── ThemeConfig ───────────────────────────────────────────────────────────────

/// Visual theme settings.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ThemeConfig {
    /// Theme name (`"dark"`, `"light"`, `"material-you-2026"`, …).
    #[serde(default)]
    pub name: String,
}

// ── PermissionsConfig ─────────────────────────────────────────────────────────

/// Permissions allow/deny lists.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PermissionsConfig {
    /// Allow-listed action descriptors.
    #[serde(default)]
    pub allow: Vec<String>,
    /// Deny-listed action descriptors.
    #[serde(default)]
    pub deny: Vec<String>,
}

// ── TerminalConfig ────────────────────────────────────────────────────────────

/// Top-level configuration object for `~/.aphrody/terminal.json`.
///
/// Unknown root-level keys are rejected; sub-structs accept unknown keys
/// silently where appropriate.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TerminalConfig {
    /// LLM panes toggles.
    #[serde(default)]
    pub llm: LlmPaneConfig,
    /// Voice settings.
    #[serde(default)]
    pub voice: VoiceConfig,
    /// Theme settings.
    #[serde(default)]
    pub theme: ThemeConfig,
    /// Permissions block.
    #[serde(default)]
    pub permissions: PermissionsConfig,
    /// Keymap (logical action → key chord).  BTreeMap so the generated
    /// schema and serialised output are deterministic.
    #[serde(default)]
    pub keymap: BTreeMap<String, String>,
    /// MCP server registry imported from `.mcp.json` (logical name → entry).
    #[serde(default, rename = "mcpServers")]
    pub mcp_servers: BTreeMap<String, McpServerEntry>,
}

// ── Loaders ───────────────────────────────────────────────────────────────────

/// Resolve `~/.aphrody/terminal.json`.
fn default_terminal_json_path() -> Result<PathBuf> {
    let home =
        dirs::home_dir().ok_or_else(|| ConfigError::validation("home directory not resolvable"))?;
    Ok(home.join(".aphrody").join("terminal.json"))
}

/// Load the default `~/.aphrody/terminal.json`, falling back to
/// [`TerminalConfig::default`] on missing file.
///
/// # Errors
///
/// Returns [`ConfigError::Io`] for unreadable existing files,
/// [`ConfigError::Parse`] for malformed JSON, or [`ConfigError::Validation`]
/// for schema violations.
pub fn load_default() -> Result<TerminalConfig> {
    let path = default_terminal_json_path()?;
    load_from_path(&path)
}

/// Load a [`TerminalConfig`] from an explicit path.
///
/// **Missing path → `Ok(TerminalConfig::default())`.**  This is a deliberate
/// design choice so the terminal can boot in a brand-new home directory.
///
/// # Errors
///
/// Returns [`ConfigError::Io`] for I/O failure on an *existing* file,
/// [`ConfigError::Parse`] for malformed JSON, or [`ConfigError::Validation`]
/// for schema violations (unknown fields, wrong types, missing required keys).
pub fn load_from_path(path: &Path) -> Result<TerminalConfig> {
    if !path.exists() {
        return Ok(TerminalConfig::default());
    }
    let text = fs::read_to_string(path).map_err(|e| ConfigError::io(path, e))?;
    parse_and_validate(&text)
}

/// Validate (parse + schema-check) a JSON string.
///
/// Returns `Ok(())` if the string deserialises into a `TerminalConfig`
/// without losing data.  Wraps `serde_json` errors into
/// [`ConfigError::Validation`] so callers get a uniform error surface.
///
/// # Errors
///
/// Returns [`ConfigError::Validation`] on any parse or schema failure.
pub fn validate(json: &str) -> Result<()> {
    parse_and_validate(json).map(|_| ())
}

fn parse_and_validate(json: &str) -> Result<TerminalConfig> {
    serde_json::from_str::<TerminalConfig>(json).map_err(|e| ConfigError::validation(e.to_string()))
}

/// Emit the JSON Schema for [`TerminalConfig`] as a pretty-printed UTF-8
/// string.
///
/// # Panics
///
/// This function never panics on well-formed input; it returns the schema as
/// a `String` because the underlying serialisation never fails for
/// schemars-generated output.
#[must_use]
pub fn schema_json() -> String {
    let schema = schemars::schema_for!(TerminalConfig);
    serde_json::to_string_pretty(&schema).unwrap_or_else(|_| {
        "{\"$schema\":\"about:blank\",\"error\":\"schema serialisation failed\"}".to_owned()
    })
}

// ── Re-exports ────────────────────────────────────────────────────────────────

pub use merge::merge;
pub use shims::{
    ClaudeShim, McpShim, PermissionsBlock, SettingsShim, Shim, import_claude_json, import_mcp_json,
    import_settings_json,
};
