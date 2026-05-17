<!-- SPDX-License-Identifier: Apache-2.0 -->

# `gui` — Aphrody Desktop UI

## What is `gui`?

`gui` is the desktop UI for Aphrody — a Rust binary built on
[`wry`](https://crates.io/crates/wry) (Tauri's webview) and
[`tao`](https://crates.io/crates/tao) (cross-platform window/event loop).
Rendering is delegated to the system webview: WebView2 on Windows, WebKitGTK
on Linux, system WebKit on macOS.

The HTML shell (`src/index.html`) loads Material Web Components and talks to
the Rust process over `wry`'s IPC channel.

## Status

`gui` is **NOT** part of the distributable `aphrody` CLI binary. It ships as a
separate desktop executable. The crate has `publish = false` and is not
published to crates.io.

## Install

```bash
git clone https://github.com/aphrody-code/aphrody.git
cd aphrody
cargo run -p gui --release
```

A window titled `aphrody` opens at 1280x720 with the Material 3 UI. Prompts
round-trip through IPC into the `backend` crate.

## Public API

`gui` is a binary crate — its entry points live in
[`src/main.rs`](src/main.rs):

- `main() -> anyhow::Result<()>` — initializes `tracing_subscriber`, builds a
  Tokio `Runtime`, creates the `tao` window, attaches a `wry` WebView from
  the bundled `index.html`, and runs the event loop until
  `WindowEvent::CloseRequested`.
- `enum IpcMessage` — serde-tagged IPC envelope. Variants: `Prompt(String)`.
- `fn dispatch_prompt(rt: &Arc<Runtime>, prompt: String)` — routes IPC
  prompts: `dns:<domain>` runs `backend::dns::DnsRecon::run_osint`, `mirror`
  boots `backend::Md3Mirror::start_mirroring`, anything else is treated as
  a domain for DNS OSINT.

Global allocator: `mimalloc::MiMalloc`.

## Cross-platform notes

- **Linux** — requires GTK3 (`libgtk-3-dev`, `libwebkit2gtk-4.1-dev`),
  pulled in transitively by `tao`/`wry`.
- **Windows** — requires WebView2 Runtime (preinstalled on Windows 11).
- **macOS** — uses the system WebKit; no extra install.
- **WebAssembly** — NOT supported. `gui` opens a native OS window; the DOM is
  not a substitute. For the browser target, see `aphrody-wasm`.

## Why excluded from the CLI binary?

The CLI binary (`aphrody`) must stay `cargo install`-friendly across Linux,
Windows, and WebAssembly with a small footprint. `gui` is excluded for:

1. **CVE backlog** — per `CLAUDE.md` §7: *"GTK3 CVE (RUSTSEC-2024-04xx):
   tirés par tao/wry sur Linux, ignorés dans `deny.toml` jusqu'à migration
   GTK4. Le binaire `cli` n'est PAS lié à GTK — seul `crates/gui` l'est."*
2. **Binary size** — bundling a webview stack pushes the artifact past 100 MB.
3. **Desktop dependency footprint** — GTK3 / WebView2 / WebKit are not
   suitable for `cargo install` distribution.

## License

Apache-2.0. See the workspace `LICENSE` and the SPDX header in each source file.

## Related crates

- [`aphrody`](../cli) — cross-platform CLI binary (Linux, Windows, WASM).
- [`aphrody-wasm`](../aphrody-wasm) — browser/WebAssembly target.
- [`backend`](../backend) — forensics/network primitives consumed via IPC.
