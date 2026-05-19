// SPDX-License-Identifier: Apache-2.0
//! Aphrody desktop GUI — wry + tao + Material Design 3.
//!
//! The library exposes the cross-platform IPC contract used by both the
//! native binary (`main.rs`) and any future WebView-driven test harness.
//!
//! # wasm32 cfg gate
//!
//! Per CLAUDE.md §0 cible #3 + the task spec, this crate **must not**
//! compile on `wasm32-unknown-unknown` — `wry` + `tao` pull native window
//! handles that don't exist in the browser. The lib body is empty under
//! wasm; the binary entrypoint adds its own gate so a stray
//! `cargo check -p gui --target wasm32-unknown-unknown` fails fast with a
//! clear error instead of dragging native deps into the wasm graph.

#![forbid(unsafe_code)]
#![cfg(not(target_arch = "wasm32"))]

use serde::{Deserialize, Serialize};

/// IPC message sent from the WebView to the native host.
///
/// Two variants today:
///
/// - `prompt` — user-entered text from the chat input.
/// - `cmd`    — structured command (`tokens`, `ping`, ...).
///
/// `serde(tag, content)` keeps the wire format `{type: "...", content: ...}`
/// which is what the inline JS in `index.html` produces.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "content", rename_all = "lowercase")]
pub enum IpcMessage {
    /// Free-form prompt entered in the chat input.
    Prompt(String),
    /// Structured command from JS-side `window.ipc.postMessage({type:"cmd", content:"tokens"})`.
    Cmd(String),
}

/// Reply payload sent back from the native host via
/// `WebView::evaluate_script("window.__aphrodyReply(...)")`.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum IpcReply {
    /// Tokens payload (response to `cmd: "tokens"`).
    Tokens {
        /// CSS variables block (`:root { --md-sys-color-* }`).
        css: String,
        /// JSON-friendly token map (`{ "primary": "#6750A4", ... }`).
        colors: serde_json::Value,
    },
    /// Acknowledgement of an unknown command.
    Unknown {
        /// The command name we did not recognise.
        cmd: String,
    },
}

/// Build the JSON payload returned by `cmd: "tokens"`.
///
/// Sources:
///   - `m3_tokens::color::BASELINE_DARK` — canonical M3 dark palette
///   - `m3_tokens::export_css`           — emits the `:root { … }` block
///
/// Both come from the workspace `m3-tokens` crate so the GUI's palette
/// stays in lock-step with the TUI (`aphrody-tui::m3::Palette`) and the
/// wgpu surface (`aphrody-wgpu-material`).
#[must_use]
pub fn tokens_reply() -> IpcReply {
    let theme = m3_tokens::color::BASELINE_DARK;
    let css = m3_tokens::export_css(&theme);
    let colors = serde_json::json!({
        "primary":            format!("#{:06X}", theme.primary & 0x00FF_FFFF),
        "on_primary":         format!("#{:06X}", theme.on_primary & 0x00FF_FFFF),
        "primary_container":  format!("#{:06X}", theme.primary_container & 0x00FF_FFFF),
        "secondary":          format!("#{:06X}", theme.secondary & 0x00FF_FFFF),
        "tertiary":           format!("#{:06X}", theme.tertiary & 0x00FF_FFFF),
        "error":              format!("#{:06X}", theme.error & 0x00FF_FFFF),
        "background":         format!("#{:06X}", theme.background & 0x00FF_FFFF),
        "on_background":      format!("#{:06X}", theme.on_background & 0x00FF_FFFF),
        "surface":            format!("#{:06X}", theme.surface & 0x00FF_FFFF),
        "on_surface":         format!("#{:06X}", theme.on_surface & 0x00FF_FFFF),
        "surface_variant":    format!("#{:06X}", theme.surface_variant & 0x00FF_FFFF),
        "on_surface_variant": format!("#{:06X}", theme.on_surface_variant & 0x00FF_FFFF),
        "outline":            format!("#{:06X}", theme.outline & 0x00FF_FFFF),
        "outline_variant":    format!("#{:06X}", theme.outline_variant & 0x00FF_FFFF),
        "inverse_surface":    format!("#{:06X}", theme.inverse_surface & 0x00FF_FFFF),
        "inverse_on_surface": format!("#{:06X}", theme.inverse_on_surface & 0x00FF_FFFF),
        "inverse_primary":    format!("#{:06X}", theme.inverse_primary & 0x00FF_FFFF),
    });
    IpcReply::Tokens { css, colors }
}

/// Dispatch a parsed IPC command. Returns `Some(reply)` for commands that
/// produce a synchronous reply (today: `"tokens"`), `None` otherwise (e.g.
/// long-running prompts that complete asynchronously via tracing).
#[must_use]
pub fn dispatch_cmd(cmd: &str) -> IpcReply {
    match cmd {
        "tokens" => tokens_reply(),
        other => IpcReply::Unknown { cmd: other.to_owned() },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipc_message_round_trip_prompt() {
        let msg = IpcMessage::Prompt("hello".to_owned());
        let s = serde_json::to_string(&msg).unwrap();
        let back: IpcMessage = serde_json::from_str(&s).unwrap();
        assert!(matches!(back, IpcMessage::Prompt(t) if t == "hello"));
    }

    #[test]
    fn ipc_message_round_trip_cmd() {
        let msg = IpcMessage::Cmd("tokens".to_owned());
        let s = serde_json::to_string(&msg).unwrap();
        assert!(s.contains("\"type\":\"cmd\""));
        let back: IpcMessage = serde_json::from_str(&s).unwrap();
        assert!(matches!(back, IpcMessage::Cmd(c) if c == "tokens"));
    }

    #[test]
    fn tokens_reply_contains_primary() {
        let reply = tokens_reply();
        match reply {
            IpcReply::Tokens { css, colors } => {
                assert!(css.contains("--md-sys-color-primary:"));
                // BASELINE_DARK primary = 0xFFD0BCFF
                assert_eq!(colors["primary"], "#D0BCFF");
            },
            IpcReply::Unknown { .. } => panic!("tokens_reply must return Tokens"),
        }
    }

    #[test]
    fn dispatch_cmd_unknown_returns_unknown() {
        let reply = dispatch_cmd("does-not-exist");
        match reply {
            IpcReply::Unknown { cmd } => assert_eq!(cmd, "does-not-exist"),
            IpcReply::Tokens { .. } => panic!("expected Unknown"),
        }
    }
}
