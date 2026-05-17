// SPDX-License-Identifier: Apache-2.0
//! Cross-platform OS abstractions for the `cli` binary.
//!
//! Inspired by Chromium's `base/base_paths.h` and Android's `Environment`,
//! this module exposes per-target user-data paths so commands stay portable
//! between Windows, Linux, macOS, and WASI without `#[cfg(windows)]` scattered
//! through business logic.

use std::{
    env,
    ffi::OsString,
    path::{Path, PathBuf},
};

use miette::{IntoDiagnostic, Result, miette};

/// Operating system identifier — emitted in `version` output and in logs.
#[must_use]
// Only called from Chromium-forensics command (Windows-gated). Avoid dead-code
// warnings on Linux/macOS/wasm where the call site is cfg-stripped.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) const fn os_short_name() -> &'static str {
    if cfg!(windows) {
        "windows"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "freebsd") {
        "freebsd"
    } else if cfg!(target_family = "wasm") {
        "wasm"
    } else {
        "unknown"
    }
}

/// User-local application data root.
///
/// - Windows : `%LOCALAPPDATA%`  (typically `C:\Users\<user>\AppData\Local`).
/// - macOS   : `$HOME/Library/Application Support`.
/// - Linux   : `$XDG_DATA_HOME` if set, else `$HOME/.local/share`.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) fn local_app_data() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        return env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .ok_or_else(|| miette!("LOCALAPPDATA is not set"));
    }
    #[cfg(target_os = "macos")]
    {
        return home_dir().map(|h| h.join("Library").join("Application Support"));
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(xdg) = env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
            return Ok(PathBuf::from(xdg));
        }
        return home_dir().map(|h| h.join(".local").join("share"));
    }
    #[cfg(target_family = "wasm")]
    {
        return Ok(PathBuf::from("/tmp"));
    }
    #[cfg(not(any(windows, unix, target_family = "wasm")))]
    {
        Err(miette!("unsupported platform for local_app_data()"))
    }
}

/// User config root.
///
/// - Windows : `%APPDATA%`  (Roaming, NOT Local).
/// - macOS   : `$HOME/Library/Preferences`.
/// - Linux   : `$XDG_CONFIG_HOME` if set, else `$HOME/.config`.
#[allow(dead_code)]
pub(crate) fn config_dir() -> Result<PathBuf> {
    #[cfg(windows)]
    {
        return env::var_os("APPDATA")
            .map(PathBuf::from)
            .ok_or_else(|| miette!("APPDATA is not set"));
    }
    #[cfg(target_os = "macos")]
    {
        return home_dir().map(|h| h.join("Library").join("Preferences"));
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(xdg) = env::var_os("XDG_CONFIG_HOME").filter(|v| !v.is_empty()) {
            return Ok(PathBuf::from(xdg));
        }
        return home_dir().map(|h| h.join(".config"));
    }
    #[cfg(target_family = "wasm")]
    {
        return Ok(PathBuf::from("/tmp"));
    }
    #[cfg(not(any(windows, unix, target_family = "wasm")))]
    {
        Err(miette!("unsupported platform for config_dir()"))
    }
}

/// User home directory.
pub(crate) fn home_dir() -> Result<PathBuf> {
    let var: OsString = if cfg!(windows) {
        env::var_os("USERPROFILE")
            .or_else(|| env::var_os("HOME"))
            .ok_or_else(|| miette!("neither USERPROFILE nor HOME is set"))?
    } else {
        env::var_os("HOME").ok_or_else(|| miette!("HOME is not set"))?
    };
    Ok(PathBuf::from(var))
}

/// Google Chrome stable user-data directory, per OS.
///
/// Returns `None` on platforms / OSes where Chrome is not commonly installed
/// at a well-known path (e.g. wasm, freebsd) — caller should treat as "no
/// Chrome data available".
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) fn chrome_user_data() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let base = local_app_data().ok()?;
        return Some(base.join("Google").join("Chrome").join("User Data"));
    }
    #[cfg(target_os = "macos")]
    {
        let base = local_app_data().ok()?;
        return Some(base.join("Google").join("Chrome"));
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        // On Linux, Chrome stores config under ~/.config/google-chrome
        return config_dir().ok().map(|c| c.join("google-chrome"));
    }
    #[cfg(not(any(windows, unix)))]
    {
        return None;
    }
}

/// Google Chrome Canary (SxS) user-data directory, per OS.
#[cfg_attr(not(target_os = "windows"), allow(dead_code))]
pub(crate) fn chrome_canary_user_data() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let base = local_app_data().ok()?;
        return Some(base.join("Google").join("Chrome SxS").join("User Data"));
    }
    #[cfg(target_os = "macos")]
    {
        let base = local_app_data().ok()?;
        return Some(base.join("Google").join("Chrome Canary"));
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        return config_dir().ok().map(|c| c.join("google-chrome-unstable"));
    }
    #[cfg(not(any(windows, unix)))]
    {
        return None;
    }
}

/// Ensure a directory exists (creating it recursively if needed).
#[allow(dead_code)]
pub(crate) fn ensure_dir(path: &Path) -> Result<()> {
    std::fs::create_dir_all(path).into_diagnostic()
}
