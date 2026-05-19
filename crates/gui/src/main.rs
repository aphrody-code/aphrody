// SPDX-License-Identifier: Apache-2.0
//! Aphrody desktop GUI binary — `tao` window + `wry` WebView shell.
//!
//! The shell ships a Material Design 3 surface whose colour palette is
//! injected at runtime from the workspace `m3-tokens` crate (see
//! [`gui::tokens_reply`]). The WebView exposes a single IPC channel
//! (`window.ipc.postMessage(JSON)`) carrying [`gui::IpcMessage`] values;
//! `cmd: "tokens"` triggers a synchronous reply via
//! `evaluate_script("window.__aphrodyReply(...)")`, while `prompt`
//! payloads dispatch into the backend DNS / mirror tasks asynchronously.
//!
//! Linux: relies on system GTK3 (CVE acknowledged in `deny.toml`, cf.
//! CLAUDE.md §7). Windows: WebView2 runtime. macOS: WKWebView.
//! WASM: this binary is gated off via `#![cfg(not(target_arch = "wasm32"))]`.

#![cfg(not(target_arch = "wasm32"))]
#![forbid(unsafe_code)]

use std::sync::Arc;

use backend::{Md3Mirror, dns::DnsRecon};
use gui::{IpcMessage, IpcReply, dispatch_cmd, tokens_reply};
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use tokio::runtime::Runtime;
use tracing::{error, info, warn};
use wry::{WebView, WebViewBuilder};

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

/// Embed the static shell HTML at compile time.
const SHELL_HTML: &str = include_str!("index.html");

/// Build the page string by splicing the runtime M3 CSS into the static
/// shell. The placeholder `<!--APHRODY_M3_CSS-->` is replaced verbatim so
/// the rest of the document stays byte-identical.
fn page_with_m3_css() -> String {
    let reply = tokens_reply();
    let css = match reply {
        IpcReply::Tokens { css, .. } => css,
        IpcReply::Unknown { .. } => String::new(),
    };
    SHELL_HTML.replace("<!--APHRODY_M3_CSS-->", &format!("<style>{css}</style>"))
}

fn dispatch_prompt(rt: &Arc<Runtime>, prompt: String) {
    let trimmed = prompt.trim().to_owned();

    if let Some(domain) = trimmed.strip_prefix("dns:") {
        let domain = domain.trim().to_owned();
        let rt = Arc::clone(rt);
        rt.spawn(async move {
            match DnsRecon::new().run_osint(&domain).await {
                Ok(records) => {
                    info!("DNS OSINT for {}: {} record(s) found", domain, records.len());
                    for r in &records {
                        info!("  {}", r);
                    }
                },
                Err(e) => error!("DNS OSINT failed for {}: {:#}", domain, e),
            }
        });
    } else if trimmed.starts_with("mirror") {
        let rt = Arc::clone(rt);
        rt.spawn(async move {
            match Md3Mirror::new() {
                Ok(mirror) => {
                    if let Err(e) = mirror.start_mirroring().await {
                        error!("Md3Mirror mirroring failed: {:#}", e);
                    }
                },
                Err(e) => error!("Md3Mirror init failed: {:#}", e),
            }
        });
    } else {
        // Default: treat the whole prompt as a domain for DNS OSINT.
        let rt = Arc::clone(rt);
        rt.spawn(async move {
            match DnsRecon::new().run_osint(&trimmed).await {
                Ok(records) => {
                    info!("DNS OSINT for {}: {} record(s) found", trimmed, records.len());
                    for r in &records {
                        info!("  {}", r);
                    }
                },
                Err(e) => error!("DNS OSINT failed for {}: {:#}", trimmed, e),
            }
        });
    }
}

/// Hand a synchronous IPC reply back to the WebView by evaluating
/// `window.__aphrodyReply(<json>)`. The reply payload is JSON-encoded
/// twice (once via serde, once via the host's JS escaping) so the script
/// is safe for arbitrary token values.
fn deliver_reply(webview: &WebView, reply: &IpcReply) {
    let json = match serde_json::to_string(reply) {
        Ok(j) => j,
        Err(e) => {
            warn!("failed to encode IPC reply: {e}");
            return;
        },
    };
    // `JSON.parse(<string>)` shields us from JS injection — the encoded
    // value is always a JSON string literal, never a bare object.
    let script = format!(
        "if (window.__aphrodyReply) {{ window.__aphrodyReply(JSON.parse({})); }}",
        serde_json::Value::String(json),
    );
    if let Err(e) = webview.evaluate_script(&script) {
        warn!("evaluate_script(reply) failed: {e}");
    }
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let rt = Arc::new(Runtime::new()?);

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("aphrody")
        .with_inner_size(tao::dpi::LogicalSize::new(1280.0, 720.0))
        .build(&event_loop)?;

    let html = page_with_m3_css();
    let rt_ipc = Arc::clone(&rt);
    // We need a handle to the WebView inside the IPC closure so we can call
    // `evaluate_script` for synchronous replies. We build the view, then
    // wrap it in an Arc<Option<...>> shared with the closure — `WebView`
    // is not Clone on every backend, so the indirection keeps lifetimes
    // honest.
    let webview_holder: Arc<std::sync::OnceLock<WebView>> = Arc::new(std::sync::OnceLock::new());
    let webview_for_ipc = Arc::clone(&webview_holder);

    let webview = WebViewBuilder::new()
        .with_html(html)
        .with_ipc_handler(move |request| {
            let body = request.body();
            match serde_json::from_str::<IpcMessage>(body) {
                Ok(IpcMessage::Prompt(p)) => dispatch_prompt(&rt_ipc, p),
                Ok(IpcMessage::Cmd(cmd)) => {
                    let reply = dispatch_cmd(&cmd);
                    if let Some(wv) = webview_for_ipc.get() {
                        deliver_reply(wv, &reply);
                    } else {
                        warn!("cmd '{cmd}' arrived before WebView was registered");
                    }
                },
                Err(e) => error!("invalid IPC message: {} ({:#})", body, e),
            }
        })
        .build(&window)?;
    // Safe to ignore the second-set error: OnceLock::set returns Err only
    // if a value was already stored, which cannot happen on the main path.
    let _ = webview_holder.set(webview);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        if let Event::WindowEvent { event: WindowEvent::CloseRequested, .. } = event {
            *control_flow = ControlFlow::Exit;
        }
    });
}
