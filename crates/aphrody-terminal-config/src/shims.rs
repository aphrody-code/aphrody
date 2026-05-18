// SPDX-License-Identifier: Apache-2.0
//! Read-only import shims for adjacent tooling config files.
//!
//! The shim layer lets the aphrody terminal pick up settings that already
//! exist in the user's home directory or repo root:
//!
//! | Shim            | Default path                  | Source format          |
//! |-----------------|-------------------------------|------------------------|
//! | [`ClaudeShim`]  | `~/.claude.json`              | Claude Code root cfg   |
//! | [`McpShim`]     | `./.mcp.json`                 | Model-Context-Protocol |
//! | [`SettingsShim`]| `~/.claude/settings.json`     | Claude settings        |
//!
//! Shim structs deliberately expose **immutable** views — no setters and no
//! mutable accessors — because their backing files are owned by another tool
//! and must not be rewritten from here.  Consumers merge the relevant fields
//! into a [`crate::TerminalConfig`] via [`crate::merge::merge`].
//!
//! ## Loader extraction
//!
//! The MCP loader generalises the partial implementation that lives in
//! `crates/aphrody-terminal-llm/src/mcp.rs` (function `load_mcp_json`).
//! The terminal-llm copy is left in place for backward compatibility; new
//! callers should prefer [`import_mcp_json`].

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{ConfigError, Result};

// ── ClaudeShim ────────────────────────────────────────────────────────────────

/// Read-only view of `~/.claude.json`.
///
/// Only a small set of fields are deserialised; unknown keys are tolerated
/// because the upstream Claude Code schema evolves frequently.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ClaudeShim {
    /// Optional theme hint (`"dark"`, `"light"`, …).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
    /// Free-form key/value extras propagated to TerminalConfig.keymap if the
    /// user has put any string-string pairs under `keymap`.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub keymap: HashMap<String, String>,
}

/// Import `~/.claude.json` (or the supplied path) as a [`ClaudeShim`].
///
/// Returns `Ok(ClaudeShim::default())` if the file does not exist, matching
/// the loader contract used by [`crate::load_default`].
///
/// # Errors
///
/// Returns [`ConfigError::Io`] for unreadable files (perms, mid-read failure)
/// or [`ConfigError::Parse`] for malformed JSON.
pub fn import_claude_json(path: Option<&Path>) -> Result<ClaudeShim> {
    let resolved: PathBuf = match path {
        Some(p) => p.to_path_buf(),
        None => default_claude_json_path()?,
    };
    if !resolved.exists() {
        return Ok(ClaudeShim::default());
    }
    let text = fs::read_to_string(&resolved).map_err(|e| ConfigError::io(&resolved, e))?;
    let shim: ClaudeShim = serde_json::from_str(&text)?;
    Ok(shim)
}

fn default_claude_json_path() -> Result<PathBuf> {
    let home =
        dirs::home_dir().ok_or_else(|| ConfigError::validation("home directory not resolvable"))?;
    Ok(home.join(".claude.json"))
}

// ── McpShim ───────────────────────────────────────────────────────────────────

/// Single `mcpServers` entry as it appears in `.mcp.json`.
///
/// Matches the schema used by Claude Code and re-implemented in
/// `aphrody-terminal-llm::mcp::McpJsonEntry`.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct McpServerEntry {
    /// Transport type discriminator (`"stdio"`, `"http"`, `"streamable-http"`).
    #[serde(rename = "type", default, skip_serializing_if = "String::is_empty")]
    pub transport_type: String,
    /// Stdio command.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    /// Stdio command arguments.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub args: Vec<String>,
    /// Stdio environment overrides.
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub env: HashMap<String, String>,
    /// HTTP server URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// HTTP request headers (may contain `${ENV_VAR}` placeholders).
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub headers: HashMap<String, String>,
}

/// Read-only view of `./.mcp.json` (or any path supplied by the caller).
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct McpShim {
    /// Map of logical server name → entry.
    #[serde(rename = "mcpServers", default)]
    pub servers: HashMap<String, McpServerEntry>,
}

/// Import `.mcp.json` (default: `./.mcp.json`) as a [`McpShim`].
///
/// Returns `Ok(McpShim::default())` if the file does not exist.
///
/// # Errors
///
/// Returns [`ConfigError::Io`] / [`ConfigError::Parse`] on the usual failure
/// modes.
pub fn import_mcp_json(path: Option<&Path>) -> Result<McpShim> {
    let resolved: PathBuf = match path {
        Some(p) => p.to_path_buf(),
        None => PathBuf::from(".mcp.json"),
    };
    if !resolved.exists() {
        return Ok(McpShim::default());
    }
    let text = fs::read_to_string(&resolved).map_err(|e| ConfigError::io(&resolved, e))?;
    let shim: McpShim = serde_json::from_str(&text)?;
    Ok(shim)
}

// ── SettingsShim ──────────────────────────────────────────────────────────────

/// Permissions block from `~/.claude/settings.json`.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct PermissionsBlock {
    /// List of allow-listed action descriptors (e.g. `"Bash(ls)"`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allow: Vec<String>,
    /// List of deny-listed action descriptors.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub deny: Vec<String>,
}

/// Read-only view of `~/.claude/settings.json`.
#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct SettingsShim {
    /// Permissions allow/deny lists.
    #[serde(default)]
    pub permissions: PermissionsBlock,
    /// Optional theme override.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<String>,
}

/// Import `~/.claude/settings.json` (or the supplied path) as a
/// [`SettingsShim`].
///
/// Returns `Ok(SettingsShim::default())` if the file does not exist.
///
/// # Errors
///
/// Returns [`ConfigError::Io`] / [`ConfigError::Parse`] on the usual failure
/// modes.
pub fn import_settings_json(path: Option<&Path>) -> Result<SettingsShim> {
    let resolved: PathBuf = match path {
        Some(p) => p.to_path_buf(),
        None => default_settings_json_path()?,
    };
    if !resolved.exists() {
        return Ok(SettingsShim::default());
    }
    let text = fs::read_to_string(&resolved).map_err(|e| ConfigError::io(&resolved, e))?;
    let shim: SettingsShim = serde_json::from_str(&text)?;
    Ok(shim)
}

fn default_settings_json_path() -> Result<PathBuf> {
    let home =
        dirs::home_dir().ok_or_else(|| ConfigError::validation("home directory not resolvable"))?;
    Ok(home.join(".claude").join("settings.json"))
}

// ── Shim enum (for merge precedence) ──────────────────────────────────────────

/// Tagged union over the three import shims, used by [`crate::merge::merge`]
/// to apply per-shim precedence rules.
#[derive(Clone, Debug)]
pub enum Shim {
    /// Wraps a [`ClaudeShim`].
    Claude(ClaudeShim),
    /// Wraps an [`McpShim`].
    Mcp(McpShim),
    /// Wraps a [`SettingsShim`].
    Settings(SettingsShim),
}
