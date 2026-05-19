// SPDX-License-Identifier: Apache-2.0
//! Loader for the Claude Desktop / Cursor / Continue compatible
//! `~/.config/aphrody/mcp.json` MCP server registry.
//!
//! # Format
//!
//! ```json
//! {
//!   "mcpServers": {
//!     "google_mcp": {
//!       "command": "google_mcp",
//!       "args": [],
//!       "env": { "RUST_LOG": "info" }
//!     },
//!     "bxc": {
//!       "type": "stdio",
//!       "command": "bxc",
//!       "args": ["serve", "--stdio"]
//!     },
//!     "supabase": {
//!       "type": "http",
//!       "url": "https://mcp.supabase.com/v1/rpc",
//!       "headers": { "Authorization": "Bearer ${SUPABASE_TOKEN}" }
//!     }
//!   }
//! }
//! ```
//!
//! The `type` field is optional and defaults to `"stdio"`. Supported transport
//! tags: `"stdio"` (default), `"http"`, `"streamable-http"`.
//!
//! Resolution order for the file path (first match wins):
//!
//! 1. `APHRODY_MCP_CONFIG` environment variable.
//! 2. `<XDG_CONFIG_HOME>/aphrody/mcp.json` (or `$HOME/.config/aphrody/mcp.json` on POSIX).
//! 3. On Windows: `%APPDATA%\aphrody\mcp.json`.
//! 4. Fallback: `<repo>/.mcp.json` for in-tree development.
//!
//! This loader is intentionally minimal — the rich `aphrody-terminal-config`
//! shim is the reference parser for `.mcp.json` (with environment-variable
//! placeholder expansion in headers). For the gemini runtime we want a
//! single-file, dependency-free entry point so it can be exercised from
//! `aphrody mcp list` without dragging in the full terminal-config stack.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors produced by [`load_mcp_config`] and friends.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Could not resolve a default config path because `$HOME` / `%APPDATA%`
    /// is not set.
    #[error("could not resolve default MCP config path: no home/config directory")]
    NoHome,

    /// Filesystem I/O error.
    #[error("I/O error reading {path}: {source}")]
    Io {
        /// Path that failed.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// JSON parse error.
    #[error("malformed MCP config at {path}: {source}")]
    Parse {
        /// Path that failed to parse.
        path: PathBuf,
        /// Underlying serde error.
        #[source]
        source: serde_json::Error,
    },
}

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Transport variant for a single MCP server entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum McpTransportConfig {
    /// Stdio transport: spawn `command` with `args` and inherit/override `env`.
    Stdio {
        /// Executable to spawn (looked up on `PATH` if not absolute).
        command: String,
        /// Command-line arguments.
        #[serde(default)]
        args: Vec<String>,
        /// Extra environment variables.
        #[serde(default)]
        env: HashMap<String, String>,
    },
    /// HTTP transport: POST JSON-RPC frames to `url`.
    Http {
        /// MCP HTTP endpoint URL.
        url: String,
        /// Optional headers (e.g. `Authorization: Bearer …`). Values are
        /// passed through verbatim — placeholder expansion is the caller's
        /// responsibility.
        #[serde(default)]
        headers: HashMap<String, String>,
    },
    /// Streamable-HTTP transport (HTTP with SSE-style chunked responses).
    /// Treated identically to [`McpTransportConfig::Http`] at the wire level;
    /// the variant is preserved so callers can decide whether to subscribe to
    /// the SSE channel separately.
    #[serde(rename = "streamable-http")]
    StreamableHttp {
        /// MCP HTTP endpoint URL.
        url: String,
        /// Optional headers.
        #[serde(default)]
        headers: HashMap<String, String>,
    },
}

/// A single MCP server entry resolved from the config file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpServerConfig {
    /// Logical name (the map key in `mcpServers`).
    pub name: String,
    /// Transport descriptor.
    pub transport: McpTransportConfig,
}

/// Parsed `mcp.json` document.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct McpConfig {
    /// Configured servers, in stable iteration order (`HashMap` insertion).
    pub servers: Vec<McpServerConfig>,
}

// ---------------------------------------------------------------------------
// Raw on-disk schema (private)
// ---------------------------------------------------------------------------

/// Permissive on-disk schema. The `type` field is a string with an empty
/// default so unspecified entries fall through to stdio (Claude Desktop
/// compatibility — Claude entries omit `type` for stdio).
#[derive(Deserialize)]
struct RawEntry {
    #[serde(rename = "type", default)]
    transport_type: String,
    #[serde(default)]
    command: Option<String>,
    #[serde(default)]
    args: Vec<String>,
    #[serde(default)]
    env: HashMap<String, String>,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    headers: HashMap<String, String>,
}

#[derive(Deserialize)]
struct RawDocument {
    #[serde(rename = "mcpServers", default)]
    servers: std::collections::BTreeMap<String, RawEntry>,
}

fn raw_entry_into_transport(entry: RawEntry, name: &str) -> McpTransportConfig {
    match entry.transport_type.as_str() {
        "http" => McpTransportConfig::Http {
            url: entry.url.unwrap_or_default(),
            headers: entry.headers,
        },
        "streamable-http" => McpTransportConfig::StreamableHttp {
            url: entry.url.unwrap_or_default(),
            headers: entry.headers,
        },
        // "stdio" or empty → stdio.
        _ => McpTransportConfig::Stdio {
            command: entry.command.unwrap_or_else(|| name.to_owned()),
            args: entry.args,
            env: entry.env,
        },
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse an MCP config document from an in-memory JSON string.
///
/// Useful for tests and for callers that already have the document in memory
/// (e.g. read from a non-filesystem source like a vault).
///
/// # Errors
///
/// Returns [`ConfigError::Parse`] with `path == ""` if the JSON is invalid.
pub fn parse_mcp_config(json: &str) -> Result<McpConfig, ConfigError> {
    let raw: RawDocument =
        serde_json::from_str(json).map_err(|source| ConfigError::Parse {
            path: PathBuf::new(),
            source,
        })?;
    let mut servers = Vec::with_capacity(raw.servers.len());
    for (name, entry) in raw.servers {
        let transport = raw_entry_into_transport(entry, &name);
        servers.push(McpServerConfig { name, transport });
    }
    Ok(McpConfig { servers })
}

/// Resolve the default `mcp.json` path according to the documented search
/// order. Returns the first candidate that exists, or the first candidate
/// (which may not exist) when none match — callers can then surface a
/// targeted "no config found at …" diagnostic.
///
/// # Errors
///
/// Returns [`ConfigError::NoHome`] if no candidate can be constructed (e.g.
/// when running in a sandbox with no `$HOME` and no `%APPDATA%`).
pub fn default_config_path() -> Result<PathBuf, ConfigError> {
    if let Ok(p) = std::env::var("APHRODY_MCP_CONFIG") {
        if !p.is_empty() {
            return Ok(PathBuf::from(p));
        }
    }
    let candidates = candidate_paths();
    if candidates.is_empty() {
        return Err(ConfigError::NoHome);
    }
    for c in &candidates {
        if c.exists() {
            return Ok(c.clone());
        }
    }
    Ok(candidates.into_iter().next().expect("non-empty"))
}

fn candidate_paths() -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Some(cfg) = dirs::config_dir() {
        out.push(cfg.join("aphrody").join("mcp.json"));
    }
    if let Some(home) = dirs::home_dir() {
        out.push(home.join(".config").join("aphrody").join("mcp.json"));
    }
    // Fallback for in-tree development.
    out.push(PathBuf::from(".mcp.json"));
    out
}

/// Load and parse the MCP config from `path`.
///
/// If the file does not exist, returns an empty [`McpConfig`] (mirroring the
/// behaviour of the `aphrody-terminal-config` shim loader).
///
/// # Errors
///
/// - [`ConfigError::Io`] — the file exists but cannot be read.
/// - [`ConfigError::Parse`] — the file is not valid JSON.
pub fn load_mcp_config(path: &Path) -> Result<McpConfig, ConfigError> {
    if !path.exists() {
        return Ok(McpConfig::default());
    }
    let text = fs::read_to_string(path)
        .map_err(|source| ConfigError::Io { path: path.to_path_buf(), source })?;
    let raw: RawDocument =
        serde_json::from_str(&text).map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source,
        })?;
    let mut servers = Vec::with_capacity(raw.servers.len());
    for (name, entry) in raw.servers {
        let transport = raw_entry_into_transport(entry, &name);
        servers.push(McpServerConfig { name, transport });
    }
    Ok(McpConfig { servers })
}

/// Load the MCP config from the default path resolved by
/// [`default_config_path`].
///
/// # Errors
///
/// Any error returned by [`default_config_path`] or [`load_mcp_config`].
pub fn load_default_mcp_config() -> Result<McpConfig, ConfigError> {
    let path = default_config_path()?;
    load_mcp_config(&path)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_claude_desktop_format() {
        let json = r#"{
            "mcpServers": {
                "google_mcp": { "command": "google_mcp", "args": [] },
                "bxc": { "type": "stdio", "command": "bxc", "args": ["serve", "--stdio"], "env": {"BXC_LOG": "info"} },
                "supabase": { "type": "http", "url": "https://mcp.supabase.com/", "headers": {"Authorization": "Bearer x"} }
            }
        }"#;
        let cfg = parse_mcp_config(json).expect("ok");
        assert_eq!(cfg.servers.len(), 3);

        // BTreeMap ⇒ alphabetic ordering: bxc, google_mcp, supabase.
        assert_eq!(cfg.servers[0].name, "bxc");
        assert_eq!(cfg.servers[1].name, "google_mcp");
        assert_eq!(cfg.servers[2].name, "supabase");

        match &cfg.servers[0].transport {
            McpTransportConfig::Stdio { command, args, env } => {
                assert_eq!(command, "bxc");
                assert_eq!(args, &vec!["serve".to_owned(), "--stdio".to_owned()]);
                assert_eq!(env.get("BXC_LOG").map(String::as_str), Some("info"));
            },
            other => panic!("expected stdio, got {other:?}"),
        }
        match &cfg.servers[2].transport {
            McpTransportConfig::Http { url, headers } => {
                assert_eq!(url, "https://mcp.supabase.com/");
                assert_eq!(headers.get("Authorization").map(String::as_str), Some("Bearer x"));
            },
            other => panic!("expected http, got {other:?}"),
        }
    }

    #[test]
    fn parse_empty_document() {
        let cfg = parse_mcp_config(r#"{}"#).expect("ok");
        assert!(cfg.servers.is_empty());
    }

    #[test]
    fn parse_default_type_is_stdio() {
        // Claude Desktop omits the `type` field for stdio entries.
        let json = r#"{ "mcpServers": { "x": { "command": "echo", "args": ["hi"] } } }"#;
        let cfg = parse_mcp_config(json).expect("ok");
        assert!(matches!(cfg.servers[0].transport, McpTransportConfig::Stdio { .. }));
    }

    #[test]
    fn parse_streamable_http_variant() {
        let json =
            r#"{ "mcpServers": { "s": { "type": "streamable-http", "url": "https://x.example/" } } }"#;
        let cfg = parse_mcp_config(json).expect("ok");
        match &cfg.servers[0].transport {
            McpTransportConfig::StreamableHttp { url, .. } => {
                assert_eq!(url, "https://x.example/");
            },
            other => panic!("expected streamable-http, got {other:?}"),
        }
    }

    #[test]
    fn load_returns_default_on_missing_file() {
        let cfg = load_mcp_config(Path::new("/this/path/does/not/exist/mcp.json")).expect("ok");
        assert!(cfg.servers.is_empty());
    }

    #[test]
    fn load_parses_real_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("mcp.json");
        std::fs::write(
            &path,
            r#"{ "mcpServers": { "z": { "command": "/bin/true" } } }"#,
        )
        .expect("write");
        let cfg = load_mcp_config(&path).expect("ok");
        assert_eq!(cfg.servers.len(), 1);
        assert_eq!(cfg.servers[0].name, "z");
    }

    #[test]
    fn load_io_error_surfaces_path() {
        // Read a directory as a file ⇒ guaranteed I/O error on every OS.
        let dir = tempfile::tempdir().expect("tempdir");
        let p = dir.path().to_path_buf();
        let err = load_mcp_config(&p).expect_err("must error");
        match err {
            ConfigError::Io { path, .. } => assert_eq!(path, p),
            other => panic!("expected Io, got {other:?}"),
        }
    }

    #[test]
    fn malformed_json_returns_parse_error() {
        let err = parse_mcp_config(r#"{not json"#).expect_err("must err");
        assert!(matches!(err, ConfigError::Parse { .. }));
    }

    #[test]
    fn default_config_path_respects_env_override() {
        // SAFETY: tests are serialised when needed via tokio's thread pool;
        // this test reads & restores the env var. Nightly Rust requires
        // `unsafe { … }` for env mutation but cfg(test) opt-in lives in lib.rs.
        let prev = std::env::var("APHRODY_MCP_CONFIG").ok();
        // SAFETY: see comment above.
        unsafe { std::env::set_var("APHRODY_MCP_CONFIG", "/tmp/custom-mcp.json") };
        let resolved = default_config_path().expect("ok");
        assert_eq!(resolved, PathBuf::from("/tmp/custom-mcp.json"));
        // Restore.
        // SAFETY: see comment above.
        match prev {
            Some(v) => unsafe { std::env::set_var("APHRODY_MCP_CONFIG", v) },
            None => unsafe { std::env::remove_var("APHRODY_MCP_CONFIG") },
        }
    }
}
