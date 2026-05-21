// SPDX-License-Identifier: Apache-2.0
//! Interop with the Antigravity / Gemini agent configuration file.
//!
//! The Antigravity client reads `~/.gemini/config/config.json` (resolved from
//! the extracted `paths.js` of the 2.0.1 client).  Observed contents are a
//! flat JSON object holding agent permission grants and a browser JS execution
//! policy, for example:
//!
//! ```json
//! {
//!   "permissions": ["command(*)", "unsandboxed(*)", "mcp(*)"],
//!   "browserJsExecutionPolicy": "TURBO"
//! }
//! ```
//!
//! [`GeminiConfig`] is a **tolerant** model: unknown keys are captured into
//! [`GeminiConfig::extra`] via `#[serde(flatten)]` so that [`GeminiConfig::save`]
//! round-trips losslessly without dropping fields written by newer client
//! versions.
//!
//! ## Platform support
//!
//! | Platform    | [`config_path`]                      | [`load`] / [`save`] |
//! |-------------|--------------------------------------|---------------------|
//! | Linux/macOS | `$HOME/.gemini/config/config.json`   | Yes                 |
//! | Windows     | `%USERPROFILE%/.gemini/config/config.json` | Yes           |
//! | wasm32      | [`ConfigError::Unsupported`]         | unavailable (no fs) |
//!
//! ```
//! use antigravity_sdk::config::GeminiConfig;
//!
//! let cfg: GeminiConfig = serde_json::from_str(
//!     r#"{"permissions":["command(*)"],"browserJsExecutionPolicy":"TURBO","x":1}"#,
//! )
//! .unwrap();
//! assert_eq!(cfg.permissions, ["command(*)"]);
//! assert_eq!(cfg.browser_js_execution_policy.as_deref(), Some("TURBO"));
//! // Unknown keys survive in `extra`.
//! assert_eq!(cfg.extra.get("x"), Some(&serde_json::json!(1)));
//! ```

#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors raised by the `~/.gemini/config/config.json` interop layer.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// The user's home directory could not be resolved from the environment
    /// (`HOME` on Unix, `USERPROFILE` on Windows), or filesystem access is not
    /// available on this target (e.g. `wasm32`).
    #[error("could not resolve home directory: {0}")]
    Unsupported(&'static str),

    /// An I/O error occurred while reading or writing the config file.
    #[error("config file I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The config file contents could not be parsed as JSON, or the in-memory
    /// model could not be serialized back to JSON.
    #[error("config JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Typed, tolerant view of `~/.gemini/config/config.json`.
///
/// Known fields are surfaced as named members; every other key in the JSON
/// object is preserved in [`extra`](Self::extra) so that a load/modify/save
/// cycle never discards data.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GeminiConfig {
    /// Agent permission grants, e.g. `"command(*)"`, `"unsandboxed(*)"`,
    /// `"mcp(*)"`.  Empty when the key is absent.
    #[serde(default)]
    pub permissions: Vec<String>,

    /// Browser JS execution policy (e.g. `"TURBO"`), serialized as
    /// `browserJsExecutionPolicy`.  `None` when the key is absent.
    #[serde(default)]
    pub browser_js_execution_policy: Option<String>,

    /// Any keys not modelled above, preserved verbatim for lossless round-trips.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Resolve the path to `~/.gemini/config/config.json` for the current user.
///
/// The home directory is read from `HOME` (Unix/macOS) or `USERPROFILE`
/// (Windows) without pulling in an extra dependency.  Returns
/// [`ConfigError::Unsupported`] when no home variable is set, or on targets
/// without a filesystem (`wasm32`).
///
/// # Errors
///
/// Returns [`ConfigError::Unsupported`] if the home directory cannot be
/// resolved from the environment.
pub fn config_path() -> Result<PathBuf, ConfigError> {
    let home = home_dir()?;
    Ok(home.join(".gemini").join("config").join("config.json"))
}

/// Resolve the current user's home directory.
///
/// Prefers the platform-conventional variable: `USERPROFILE` on Windows,
/// `HOME` elsewhere.  Falls back to the other variable if the primary one is
/// unset, which keeps Unix-style shells on Windows (Git Bash, MSYS) working.
#[cfg(not(target_arch = "wasm32"))]
fn home_dir() -> Result<PathBuf, ConfigError> {
    #[cfg(target_os = "windows")]
    let primary = "USERPROFILE";
    #[cfg(not(target_os = "windows"))]
    let primary = "HOME";

    let fallback = if primary == "HOME" {
        "USERPROFILE"
    } else {
        "HOME"
    };

    if let Some(p) = std::env::var_os(primary).filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(p));
    }
    if let Some(p) = std::env::var_os(fallback).filter(|v| !v.is_empty()) {
        return Ok(PathBuf::from(p));
    }
    Err(ConfigError::Unsupported(
        "neither HOME nor USERPROFILE is set",
    ))
}

/// No filesystem on `wasm32`; the home directory cannot be resolved.
#[cfg(target_arch = "wasm32")]
fn home_dir() -> Result<PathBuf, ConfigError> {
    Err(ConfigError::Unsupported(
        "home directory resolution is unavailable on wasm32",
    ))
}

impl GeminiConfig {
    /// Load and parse the config from the default `~/.gemini/config/config.json`
    /// location.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Unsupported`] if the path cannot be resolved,
    /// [`ConfigError::Io`] if the file cannot be read, or
    /// [`ConfigError::Json`] if the contents are not valid JSON.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load() -> Result<Self, ConfigError> {
        Self::load_from(&config_path()?)
    }

    /// Load and parse the config from an explicit path.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Io`] if the file cannot be read, or
    /// [`ConfigError::Json`] if the contents are not valid JSON.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_from(path: &Path) -> Result<Self, ConfigError> {
        let raw = std::fs::read_to_string(path)?;
        Self::from_json_str(&raw)
    }

    /// Parse the config from a JSON string.  Useful on `wasm32` where the
    /// host supplies the file contents out-of-band.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Json`] if the string is not valid JSON.
    pub fn from_json_str(raw: &str) -> Result<Self, ConfigError> {
        Ok(serde_json::from_str(raw)?)
    }

    /// Serialize the config to pretty JSON.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Json`] if serialization fails.
    pub fn to_json_string(&self) -> Result<String, ConfigError> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Serialize the config to pretty JSON and write it to `path`.
    ///
    /// Parent directories are created if they do not already exist so a fresh
    /// `~/.gemini/config/` tree can be initialized in one call.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError::Json`] if serialization fails, or
    /// [`ConfigError::Io`] if the directory or file cannot be written.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent)?;
            }
        }
        let json = self.to_json_string()?;
        std::fs::write(path, json)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
  "permissions": ["command(*)", "unsandboxed(*)", "mcp(*)"],
  "browserJsExecutionPolicy": "TURBO",
  "telemetryEnabled": false,
  "futureSetting": {"nested": [1, 2, 3]}
}"#;

    #[test]
    fn parses_known_and_unknown_fields() {
        let cfg = GeminiConfig::from_json_str(SAMPLE).unwrap();
        assert_eq!(cfg.permissions, ["command(*)", "unsandboxed(*)", "mcp(*)"]);
        assert_eq!(cfg.browser_js_execution_policy.as_deref(), Some("TURBO"));
        assert_eq!(
            cfg.extra.get("telemetryEnabled"),
            Some(&serde_json::json!(false))
        );
        assert_eq!(
            cfg.extra.get("futureSetting"),
            Some(&serde_json::json!({"nested": [1, 2, 3]}))
        );
    }

    #[test]
    fn defaults_are_empty_when_keys_absent() {
        let cfg = GeminiConfig::from_json_str("{}").unwrap();
        assert!(cfg.permissions.is_empty());
        assert!(cfg.browser_js_execution_policy.is_none());
        assert!(cfg.extra.is_empty());
    }

    #[test]
    fn camel_case_round_trip_preserves_unknown_keys() {
        let original = GeminiConfig::from_json_str(SAMPLE).unwrap();
        let serialized = original.to_json_string().unwrap();
        // The camelCase key must survive serialization.
        assert!(serialized.contains("browserJsExecutionPolicy"));
        // Unknown keys must be re-emitted.
        assert!(serialized.contains("telemetryEnabled"));
        assert!(serialized.contains("futureSetting"));

        let reparsed = GeminiConfig::from_json_str(&serialized).unwrap();
        assert_eq!(original, reparsed);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn load_from_save_round_trip_on_tempfile() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config").join("config.json");

        let original = GeminiConfig::from_json_str(SAMPLE).unwrap();
        original.save(&path).unwrap();

        // save() must have created the parent directory.
        assert!(path.exists());

        let loaded = GeminiConfig::load_from(&path).unwrap();
        assert_eq!(original, loaded);

        // Unknown keys preserved end-to-end through the filesystem.
        assert_eq!(
            loaded.extra.get("telemetryEnabled"),
            Some(&serde_json::json!(false))
        );
        assert_eq!(
            loaded.extra.get("futureSetting"),
            Some(&serde_json::json!({"nested": [1, 2, 3]}))
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn config_path_ends_with_expected_suffix() {
        // SAFETY: single-threaded test; sets an env var to make the path
        // deterministic regardless of the host environment.
        #[cfg(target_os = "windows")]
        unsafe {
            std::env::set_var("USERPROFILE", r"C:\Users\tester");
        }
        #[cfg(not(target_os = "windows"))]
        unsafe {
            std::env::set_var("HOME", "/home/tester");
        }

        let path = config_path().unwrap();
        assert!(path.ends_with("config.json"));
        let as_str = path.to_string_lossy().replace('\\', "/");
        assert!(as_str.ends_with(".gemini/config/config.json"), "{as_str}");
    }
}
