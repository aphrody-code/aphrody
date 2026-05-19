// SPDX-License-Identifier: Apache-2.0
//! High-level loader entry-points.
//!
//! These are the brief-mandated convenience functions:
//!
//! - [`load_terminal_config`] — load a [`crate::TerminalConfig`] from an explicit JSON path
//!   (delegates to [`crate::load_from_path`]).
//! - [`import_claude_json_as_config`] — `.claude.json`-shaped file → [`crate::TerminalConfig`] with
//!   the embedded `mcpServers` flattened into [`crate::TerminalConfig::mcp_servers`].
//! - [`import_mcp_json_as_config`] — `.mcp.json`-shaped file → [`crate::TerminalConfig`] with the
//!   `mcpServers` block lifted into the top-level registry.
//!
//! All three thin-wrap [`crate::shims`] + [`crate::load_from_path`] so the
//! single source of truth for parsing stays in [`crate::shims`].

use std::path::Path;

use serde::Deserialize;

use crate::{
    Result, TerminalConfig,
    error::ConfigError,
    shims::{McpServerEntry, McpShim, import_mcp_json},
};

/// Load a [`TerminalConfig`] from an explicit file path.
///
/// Missing files yield [`TerminalConfig::default`] — see
/// [`crate::load_from_path`] for the full contract.
///
/// # Errors
///
/// See [`crate::load_from_path`].
pub fn load_terminal_config(path: &Path) -> Result<TerminalConfig> {
    crate::load_from_path(path)
}

/// Read a `.claude.json`-shaped file and lift its `mcpServers` block into a
/// fresh [`TerminalConfig`].
///
/// The full Claude Code root config carries many fields that are unrelated to
/// the terminal (project list, last-used model, …); we ignore everything
/// except `mcpServers`.  Theme / keymap propagation goes through the typed
/// [`crate::shims::import_claude_json`] + [`crate::merge::merge`] pipeline.
///
/// # Errors
///
/// Returns [`ConfigError::Io`] / [`ConfigError::Parse`] on the usual failure
/// modes.
pub fn import_claude_json_as_config(path: &Path) -> Result<TerminalConfig> {
    if !path.exists() {
        return Ok(TerminalConfig::default());
    }
    let text = std::fs::read_to_string(path).map_err(|e| ConfigError::io(path, e))?;
    // Tolerant deserialise: ignore every field except `mcpServers`.
    #[derive(Deserialize)]
    struct ClaudeRoot {
        #[serde(default, rename = "mcpServers")]
        mcp_servers: std::collections::HashMap<String, McpServerEntry>,
    }
    let root: ClaudeRoot = serde_json::from_str(&text).map_err(ConfigError::Parse)?;
    let mut cfg = TerminalConfig::default();
    for (name, entry) in root.mcp_servers {
        cfg.mcp_servers.insert(name, entry);
    }
    Ok(cfg)
}

/// Read a `.mcp.json`-shaped file and lift its `mcpServers` block into a
/// fresh [`TerminalConfig`].
///
/// Thin wrapper over [`import_mcp_json`].
///
/// # Errors
///
/// Returns [`ConfigError::Io`] / [`ConfigError::Parse`] on the usual failure
/// modes.
pub fn import_mcp_json_as_config(path: &Path) -> Result<TerminalConfig> {
    let shim: McpShim = import_mcp_json(Some(path))?;
    let mut cfg = TerminalConfig::default();
    for (name, entry) in shim.servers {
        cfg.mcp_servers.insert(name, entry);
    }
    Ok(cfg)
}
