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
// Windows: no shell/console window flash on launch. Without this attribute
// the linker defaults to the console subsystem (rustc default for bins),
// which allocates a conhost + PseudoConsole that flashes a black rectangle
// before the wry window appears.
#![cfg_attr(all(target_os = "windows", not(debug_assertions)), windows_subsystem = "windows")]

use std::sync::Arc;

use gui::{IpcMessage, IpcReply, dispatch_cmd, tokens_reply};
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy},
    window::WindowBuilder,
};
use tokio::runtime::Runtime;
use tracing::{error, info, warn};
use wry::{WebView, WebViewBuilder};

/// Custom event delivered to the tao event loop from any tokio worker.
/// `WebView` is `!Send` on Windows (COM thread-affinity), so workers cannot
/// call `evaluate_script` directly — they wrap the reply in this event and
/// the main thread renders it.
#[derive(Debug, Clone)]
enum UserEvent {
    Reply(IpcReply),
}

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
        IpcReply::Unknown { .. } | IpcReply::Text { .. } | IpcReply::Error { .. } => String::new(),
    };
    SHELL_HTML.replace("<!--APHRODY_M3_CSS-->", &format!("<style>{css}</style>"))
}

/// Dispatch a user chat prompt to the unified `aphrody chat` orchestrator
/// and stream the reply back to the WebView via the [`UserEvent`] channel.
///
/// `aphrody chat` composes every aphrody building block (gemini-runtime,
/// tools, memory, session, permissions, hooks, prompts, router, cost,
/// context, events) — the full backend stack, not just the bare Gemini API
/// like `aphrody a2a` does. That is the surface the GUI must expose to
/// fulfil "TOUT le backend aphrody".
///
/// The reply round-trip uses [`EventLoopProxy<UserEvent>`] because the
/// `WebView` handle is `!Send` on Windows (COM thread-affinity) so we
/// cannot call `evaluate_script` from a tokio worker.
fn dispatch_prompt(
    rt: &Arc<Runtime>,
    proxy: EventLoopProxy<UserEvent>,
    prompt_id: String,
    prompt: String,
) {
    let trimmed = prompt.trim().to_owned();
    if trimmed.is_empty() {
        return;
    }
    let rt = Arc::clone(rt);
    rt.spawn(async move {
        info!("dispatch_prompt id={prompt_id} bytes={}", trimmed.len());
        let output = tokio::process::Command::new("aphrody")
            .arg("chat")
            .arg("--prompt")
            .arg(&trimmed)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await;

        let reply = match output {
            Ok(out) if out.status.success() => {
                let raw = String::from_utf8_lossy(&out.stdout).into_owned();
                IpcReply::Text {
                    prompt_id: prompt_id.clone(),
                    content: raw.trim().to_owned(),
                    done: true,
                }
            },
            Ok(out) => {
                let code = out.status.code().unwrap_or(-1);
                let stderr = String::from_utf8_lossy(&out.stderr).into_owned();
                error!("aphrody chat id={prompt_id} exited {code}: {}", stderr.trim());
                IpcReply::Error {
                    prompt_id: prompt_id.clone(),
                    message: format!("aphrody chat exited {code}: {}", stderr.trim()),
                }
            },
            Err(e) => {
                error!("failed to spawn aphrody chat id={prompt_id}: {e:#}");
                IpcReply::Error {
                    prompt_id: prompt_id.clone(),
                    message: format!("failed to spawn aphrody chat: {e}"),
                }
            },
        };

        if let Err(e) = proxy.send_event(UserEvent::Reply(reply)) {
            warn!("failed to send UserEvent::Reply id={prompt_id}: {e:#}");
        }
    });
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

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();
    // Frameless, full-Material — no Win32 title bar, no system close/min/max
    // chrome. The custom title bar lives in `index.html` (M3 styled), drag
    // initiated via IPC `cmd: "window:drag"` → `window.drag_window()`.
    let window = WindowBuilder::new()
        .with_title("aphrody")
        .with_decorations(false)
        .with_inner_size(tao::dpi::LogicalSize::new(1280.0, 760.0))
        .with_min_inner_size(tao::dpi::LogicalSize::new(640.0, 480.0))
        .build(&event_loop)?;
    let window = Arc::new(window);

    // Default: real Gemini mirror (pixel-perfect by construction — Google's
    // own UI renders) PLUS an injected aphrody side panel (`init_script`)
    // that surfaces every aphrody crate (chat, skills, tools, memory, voice,
    // mcp, channels, cron, marketplace…) via IPC commands. Best of both
    // worlds: visual fidelity + full backend access.
    //
    // Opt-in to the local-only native shell (no network):
    //   APHRODY_GUI_USE_LOCAL=1                  -> embedded index.html only
    //   APHRODY_GUI_URL=https://example          -> mirror a different URL
    let use_local = std::env::var("APHRODY_GUI_USE_LOCAL").is_ok();
    let mirror_url = std::env::var("APHRODY_GUI_URL")
        .unwrap_or_else(|_| "https://gemini.google.com/app".to_owned());
    info!("gui shell: {}", if use_local { "local HTML (aphrody only)".to_owned() } else { format!("mirror {} + aphrody overlay", mirror_url) });

    let html = page_with_m3_css();
    let rt_ipc = Arc::clone(&rt);
    // We need a handle to the WebView inside the IPC closure so we can call
    // `evaluate_script` for synchronous replies. We build the view, then
    // wrap it in an Arc<Option<...>> shared with the closure — `WebView`
    // is not Clone on every backend, so the indirection keeps lifetimes
    // honest.
    let webview_holder: Arc<std::sync::OnceLock<WebView>> = Arc::new(std::sync::OnceLock::new());
    let webview_for_ipc = Arc::clone(&webview_holder);

    // Userscript injected on every page load. Hooks Gemini's prompt UI and
    // diverts every send to our local Rust backend (`aphrody a2a`) via the
    // existing IPC channel. Strategy :
    //   1. Watch the document for the prompt input element. Gemini renders
    //      a contenteditable with role="textbox" and an aria-label that
    //      starts with "Demander à" (fr) or "Ask" (en) ; selector is
    //      attribute-based to survive class-name churn.
    //   2. Watch the document for the send button (aria-label starts with
    //      "Envoyer" / "Send"). On click — and on Enter inside the
    //      textbox — read the text, prevent the default Google round-trip,
    //      and forward via window.ipc.postMessage.
    //   3. Inject a small status badge in the top-right corner so the user
    //      can verify the hook is live ("aphrody backend ⚡").
    // All elements created here use the `aphrody-…` class prefix so they
    // are unambiguous from Gemini's own DOM and easy to style/remove later.
    // Overlay injected on every page load of the mirrored Gemini app.
    // Adds a floating aphrody panel on the right side of the page with
    // buttons to every aphrody crate surface (chat, skills, tools, memory,
    // voice STT/TTS, MCP, channels, cron, marketplace, RE, scan, DNS).
    // Each click sends a `cmd: "panel:<action>"` IPC, which the Rust host
    // dispatches to the matching crate and streams the result back.
    //
    // The overlay does NOT alter the Gemini DOM — it is positioned with
    // fixed z-index above the page, so the page itself remains visually
    // pixel-perfect. Hides via a small chevron toggle.
    let init_script = r#"
        (() => {
          if (window.__aphrodyOverlay) return;
          window.__aphrodyOverlay = true;

          window.__aphrodyReply = window.__aphrodyReply || function (p) { window.__lastReply = p; };

          const CRATES = [
            { id: 'chat',        label: 'Chat',         tag: 'aphrody-chat',         desc: 'turn-loop orchestrator' },
            { id: 'skills',      label: 'Skills',       tag: 'skills-runtime',       desc: 'SKILL.md aggregator' },
            { id: 'tools',       label: 'Tools',        tag: 'aphrody-tools',        desc: '9 builtin tools' },
            { id: 'memory',      label: 'Memory',       tag: 'aphrody-memory',       desc: 'JSONL/HNSW/LanceDB' },
            { id: 'voice-stt',   label: 'Voice STT',    tag: 'voice-stt',            desc: 'Whisper / ElevenLabs' },
            { id: 'voice-tts',   label: 'Voice TTS',    tag: 'aphrody-voice',        desc: 'ElevenLabs / Discord' },
            { id: 'mcp',         label: 'MCP servers',  tag: 'aphrody-mcp',          desc: 'list + call tools' },
            { id: 'channels',    label: 'Channels',     tag: 'aphrody-channels',     desc: 'Slack / Telegram / Matrix' },
            { id: 'cron',        label: 'Cron',         tag: 'aphrody-cron',         desc: 'interval / daily / cron' },
            { id: 'marketplace', label: 'Marketplace',  tag: 'aphrody-marketplace',  desc: 'skills + awesome curator' },
            { id: 'dns',         label: 'DNS OSINT',    tag: 'backend::dns',         desc: 'recon agressive' },
            { id: 're',          label: 'Reverse Eng',  tag: 'aphrody-re',           desc: 'PE/ELF triage' },
            { id: 'scan',        label: 'Scan repo',    tag: 'aphrody scan',         desc: 'tree + manifests' },
            { id: 'chromium',    label: 'Chromium',     tag: 'forensics',            desc: 'cookies + master key' },
            { id: 'bxc',         label: 'bxc',          tag: 'bxc-engine',           desc: 'browser-in-process' },
          ];

          function postIpc(msg) {
            try { window.ipc.postMessage(JSON.stringify(msg)); }
            catch (e) { console.error('[aphrody] ipc failed', e); }
          }

          function mountOverlay() {
            if (document.getElementById('aphrody-overlay')) return;
            const root = document.createElement('div');
            root.id = 'aphrody-overlay';
            root.innerHTML = `
              <style>
                #aphrody-overlay {
                  position: fixed; top: 8px; right: 8px; z-index: 2147483647;
                  width: 280px; max-height: calc(100vh - 16px);
                  background: rgba(27, 28, 29, 0.92); color: #e3e3e3;
                  border: 1px solid rgba(168, 199, 250, 0.18);
                  border-radius: 16px; backdrop-filter: blur(12px);
                  font-family: 'Google Sans Flex', system-ui, sans-serif;
                  display: flex; flex-direction: column;
                  box-shadow: 0 8px 24px rgba(0,0,0,0.45);
                  overflow: hidden; transition: width 0.2s ease, height 0.2s ease;
                }
                #aphrody-overlay.collapsed { width: 44px; height: 44px; }
                #aphrody-overlay.collapsed .aphrody-head-text,
                #aphrody-overlay.collapsed .aphrody-list,
                #aphrody-overlay.collapsed .aphrody-foot { display: none; }
                .aphrody-head {
                  display: flex; align-items: center; gap: 8px;
                  padding: 10px 12px; cursor: pointer;
                  border-bottom: 1px solid rgba(255,255,255,0.06);
                }
                .aphrody-dot {
                  width: 8px; height: 8px; border-radius: 50%;
                  background: #a8c7fa; box-shadow: 0 0 8px #a8c7fa;
                  flex-shrink: 0;
                }
                .aphrody-head-text { font-size: 12px; font-weight: 600; letter-spacing: 0.3px; color: #a8c7fa; }
                .aphrody-list {
                  flex: 1; overflow-y: auto; padding: 6px;
                  display: flex; flex-direction: column; gap: 2px;
                }
                .aphrody-list::-webkit-scrollbar { width: 6px; }
                .aphrody-list::-webkit-scrollbar-thumb { background: #34373a; border-radius: 3px; }
                .aphrody-btn {
                  display: flex; align-items: center; gap: 10px;
                  padding: 8px 10px; border-radius: 10px;
                  background: transparent; border: none; cursor: pointer;
                  color: #e3e3e3; font: 500 13px/1.3 inherit; text-align: left;
                  transition: background 0.12s ease;
                }
                .aphrody-btn:hover { background: rgba(168, 199, 250, 0.08); }
                .aphrody-btn-label { flex: 1; min-width: 0; }
                .aphrody-btn-name { font-size: 13px; color: #e3e3e3; }
                .aphrody-btn-desc { font-size: 10px; color: #9aa0a6; }
                .aphrody-foot {
                  padding: 8px 12px;
                  border-top: 1px solid rgba(255,255,255,0.06);
                  font-size: 10px; color: #9aa0a6; text-align: center;
                }
                .aphrody-foot code { background: rgba(255,255,255,0.05); padding: 1px 4px; border-radius: 3px; }
              </style>
              <div class="aphrody-head" id="aphrody-head">
                <div class="aphrody-dot"></div>
                <div class="aphrody-head-text">backend aphrody · ${CRATES.length} crates</div>
              </div>
              <div class="aphrody-list" id="aphrody-list"></div>
              <div class="aphrody-foot">
                UI mirror gemini.google.com · backend <code>aphrody</code>
              </div>
            `;
            (document.body || document.documentElement).appendChild(root);

            const list = root.querySelector('#aphrody-list');
            for (const c of CRATES) {
              const b = document.createElement('button');
              b.className = 'aphrody-btn';
              b.innerHTML = `
                <div class="aphrody-btn-label">
                  <div class="aphrody-btn-name">${c.label}</div>
                  <div class="aphrody-btn-desc">${c.desc}</div>
                </div>
              `;
              b.addEventListener('click', () => postIpc({ type: 'cmd', content: 'panel:' + c.id }));
              list.appendChild(b);
            }

            root.querySelector('#aphrody-head').addEventListener('click', () => {
              root.classList.toggle('collapsed');
            });
          }

          if (document.readyState === 'complete' || document.readyState === 'interactive') mountOverlay();
          else document.addEventListener('DOMContentLoaded', mountOverlay);
          new MutationObserver(() => { if (!document.getElementById('aphrody-overlay')) mountOverlay(); })
            .observe(document.documentElement, { childList: true, subtree: false });

          console.log('[aphrody] overlay armed — ' + CRATES.length + ' crates exposed');
        })();
    "#;

    let builder = if use_local {
        WebViewBuilder::new().with_html(html)
    } else {
        WebViewBuilder::new()
            .with_url(&mirror_url)
            .with_initialization_script(init_script)
    };

    let proxy_for_ipc = proxy.clone();
    let webview = builder
        .with_ipc_handler(move |request| {
            let body = request.body();
            match serde_json::from_str::<IpcMessage>(body) {
                Ok(IpcMessage::Prompt { id, text }) => {
                    dispatch_prompt(&rt_ipc, proxy_for_ipc.clone(), id, text);
                },
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

        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            },
            Event::UserEvent(UserEvent::Reply(reply)) => {
                if let Some(wv) = webview_holder.get() {
                    deliver_reply(wv, &reply);
                } else {
                    warn!("UserEvent::Reply received before WebView was registered");
                }
            },
            _ => {},
        }
    });
}
