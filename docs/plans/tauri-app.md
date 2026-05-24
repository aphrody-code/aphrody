<!-- SPDX-License-Identifier: Apache-2.0 -->
# Plan — aphrody desktop/mobile GUI on Tauri v2

**Status: P0 + P1 + P2 DONE** — the Tauri shell (`crates/aphrody-app`, build-excluded)
compiles the full CLI in-process and embeds the complete French React frontend
(`apps/desktop-react`, modeled on the gemini.google.com/app maquette). P3 (more
panels / packaging / mobile) next.

Decision basis (read these first): `docs/tauri/{README,architecture,aphrody-integration,risks,ui-framework}.md`
and `docs/research/{gui-options-2026,bun-vs-vite-2026,bun-rust-ffi-best-practices}.md`.

## Decision (settled by the research)

| Question | Answer | Source |
|---|---|---|
| GUI shell | **Tauri v2** (2.11.2 evaluated). Licenses clean (MIT/Apache; GTK/WebKitGTK = LGPL, dynamically linked). | `docs/tauri/` |
| Backend integration | **Path (a)** — a build-excluded crate whose `#[tauri::command]`s call `aphrody::run_async` / `run_captured` **in-process (Rust to Rust, no FFI hop)**. The `aphrody-ffi` cdylib + Bun bridge stay for the non-Rust / web / headless surface. | `aphrody-integration.md` |
| Frontend framework | **Vanilla TS + Lit / Material Web** (the `apps/console` model); SolidJS (or Svelte 5) the alternative if reactive DX grows. NOT a Rust to wasm frontend. | `ui-framework.md` |
| Frontend tooling | **Native Bun** (~5.4x faster prod build than Vite); set `NODE_ENV=production`. Vite-on-Bun = documented escape hatch only. | `bun-vs-vite-2026.md` |
| Webviews | Linux #1 = webkit2gtk-4.1 (Ubuntu 24.04+); Windows = WebView2; macOS = WKWebView; mobile later. | `risks.md` |

## Invariants

- **Core stays lean**: `aphrody-app` + tauri/wry/tao/gtk are build-**excluded** from the core workspace; never in the core `Cargo.lock`; core `cargo build` / CI unaffected.
- **Zero GPL** (confirmed: GTK/WebKitGTK are LGPL system libs, dynamically linked, no static contamination).
- The frontend MUST render on **WebKitGTK** (Linux #1) — verify there, not only on WebView2.
- One token source (`@aphrody-code/theme`); one component base (Material Web / Lit). Material Web is in maintenance mode, so keep the framework layer thin and standards-based.
- Latency: in-process Rust to Rust; command work dominates the sub-ms IPC.

## Phases

### P0 — Foundation (pure Rust, reversible, improves the core now)
- [x] **T0.1** `crates/aphrody-stdio-capture` (the name avoids a collision with the existing `aphrody-capture` = Windows screen capture): lift `with_captured_stdio` (dup2 / SetStdHandle, temp file) out of `crates/aphrody-ffi/src/capture.rs` into a new shared, host-only workspace-member crate (wasm = empty module); `aphrody-ffi` depends on it (dedup, no behaviour change). **DONE.**
- [x] **T0.2** cli lib structured entry: `pub fn run_captured(args) -> CapturedRun { code, stdout, stderr }` (sync; wraps the dispatch in `aphrody_stdio_capture::with_captured_stdio`) + `CapturedRun` (`Serialize`, for Tauri `#[command]`s) in `crates/cli/src/lib.rs`. Closes the one gap (`run_async` only returns an exit code + inherits stdio). aphrody-ffi keeps its own persistent-runtime path but shares the capture crate. **DONE.**
- [x] **T0.3** Gate: clippy `-D warnings` + nextest 16/16 (incl. 2 new capture tests) + wasm32 check + cdylib rebuild + Bun smoke. All green. **DONE.**

### P1 — `crates/aphrody-app` scaffold (Tauri, build-excluded) — DONE
- [x] **T1.1** `crates/aphrody-app` in `[workspace.exclude]` (empty `[workspace]` table -> its own `Cargo.lock`). Deps: `aphrody` (cli lib, path), `tauri` v2 + 9 shell plugins (os/dialog/clipboard/notification/process/log/window-state/single-instance/global-shortcut), `serde`, `mimalloc`. (No `aphrody-capture`: `aphrody::run_captured` already wraps the stdio capture.)
- [x] **T1.2** `tauri.conf.json`: `frontendDist = "dist"`, `withGlobalTauri`, dark 1280x832 window; `capabilities/main.json` is v2 default-deny (shell-layer perms only, no fs/shell/http/sql).
- [~] **T1.3** `#[tauri::command]` surface: `aphrody_exec(args) -> { code, stdout, stderr }` (in-process `run_captured`, serialised behind a Mutex, on the blocking pool) + `aphrody_meta()`. Typed `version`/`doctor` structs and Tauri-`Channel` streaming deferred — the UI runs `aphrody_exec` + reveals client-side, so the bounded command set needs no streaming command yet.
- [x] **T1.4** `scripts/tauri.{ps1,sh}`: build the frontend (Bun) -> copy `dist/` -> `cargo` on the excluded crate (shared target dir), without touching the core build.

### P2 — Frontend (in aphrody-ts, Bun-built) — DONE
- [x] **T2.1** `apps/desktop-react` (React 19, chosen over the vanilla shell at the user's request) imports `@aphrody-code/theme/tokens.css`, built by Bun (`NODE_ENV=production`) to a `dist/` embedded by Tauri. 100% French UI.
- [x] **T2.2** Transport-abstract client (`transport.ts`): detects `window.__TAURI__` -> `invoke('aphrody_exec')`; otherwise `fetch /api/run` (the apps/console contract). Identical bundle for both hosts (no `@tauri-apps/api` dep).
- [x] **T2.3** Multi-view app modeled on the Gemini maquette: icon rail (Console / Diagnostic / Reverse / Réseau / À propos), centered home + pill composer, Gemini-style client-side output reveal, M3 panels per command. Verified in a real webview.

### P3 — Feature surface + UX
- [ ] **T3.1** Typed M3 panels: `doctor` dashboard, version/system, the high-value command surfaces (re / forensics / chat / image / ...) as views; streaming output via Channels.

### P4 — Packaging + cross-platform CI
- [ ] **T4.1** Tauri bundler: Linux `.deb` / `.AppImage` (webkit2gtk runtime dep), Windows `.msi` / `.exe` (WebView2 bootstrap), macOS `.app` / `.dmg`.
- [ ] **T4.2** CI: build the excluded crate separately; `deny.toml` already ignores the wry/tao GTK CVEs.

### P5 — Mobile (deferred)
- [ ] **T5.1** Tauri v2 iOS / Android once desktop is solid (maturity caveats in `risks.md`).

## Success criteria

- `crates/aphrody` build / CI unchanged (lean core; `aphrody-app` excluded; no wry/tao/gtk in the core lock).
- The Tauri app runs `doctor` / `version` etc. in-process, renders the M3 fusion, on Linux (webkit2gtk) **and** Windows (WebView2).
- Zero GPL; clean `cargo deny`.
- Minimal frontend bundle (vanilla + Material Web), built by Bun.

## First step

**T0.1 + T0.2 are DONE** (`crates/aphrody-stdio-capture` + `aphrody::run_captured`) — pure Rust, reversible, and it de-duplicated `aphrody-ffi`'s capture path as a bonus. Next: **P1** (`crates/aphrody-app`, the build-excluded Tauri scaffold).
