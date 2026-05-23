<!-- SPDX-License-Identifier: Apache-2.0 -->
# Tauri v2 — cross-platform reality, licenses, and risks

Companion to [`architecture.md`](architecture.md) and the verdict in
[`README.md`](README.md). Evidence is cited as `var/tauri/<path>:<line>`.

---

## 1. Cross-platform webview matrix (the real engines)

Tauri does not bundle a browser. Each OS uses its system webview, wired through
wry. The exact bindings, from the source:

| Target | Webview engine | Rust binding (pinned) | Evidence |
|--------|----------------|------------------------|----------|
| **Linux (#1)** | WebKitGTK (GTK3 stack) | `webkit2gtk = "=2.0"`, feature `v2_40`; `gtk = "0.18"`, feature `v3_24` | `var/tauri/crates/tauri-runtime-wry/Cargo.toml:40-42`; same pins in `var/tauri/crates/tauri/Cargo.toml:103-105` |
| **Windows (#2)** | WebView2 (Edge/Chromium) | `webview2-com = "0.38"`, `windows = "0.61"`, `softbuffer = "0.4"` | `var/tauri/crates/tauri-runtime-wry/Cargo.toml:30-37` |
| **macOS** (best-effort) | WKWebView | `objc2-web-kit = "0.3"` (WKWebView/WKWebViewConfiguration/WKUserContentController) | `var/tauri/crates/tauri/Cargo.toml` (apple block, the `objc2-web-kit` features) |
| **iOS** | WKWebView | `swift-rs = "1"`, `objc2-ui-kit = "0.3"` | `var/tauri/crates/tauri/Cargo.toml` (UIKit block) |
| **Android** | Android System WebView | `jni = "0.21"` | `var/tauri/crates/tauri/Cargo.toml` (android block); `var/tauri/crates/tauri-runtime-wry/Cargo.toml:57-58` |

Confirmed at the type level: on Linux the webview *is* `webkit2gtk::WebView`
(`var/tauri/crates/tauri-runtime-wry/src/webview.rs:13`), on Windows it is
`ICoreWebView2Controller` (`var/tauri/crates/tauri-runtime-wry/src/webview.rs:32-37`).

wry/tao pins: `wry = "0.55.0"`, `tao = "0.35.0"`
(`var/tauri/crates/tauri-runtime-wry/Cargo.toml:16-22`; `Cargo.lock` confirms
`wry 0.55.0`, `tao 0.35.0`, `webkit2gtk 2.0.2`, `webview2-com 0.38.0`).

### 1.1 Correction to a prior aphrody claim

`docs/research/gui-options-2026.md:72` says Linux uses "WebKitGTK 4.1". Precisely:
the **Rust crate** is `webkit2gtk 2.0.x` and the **C API feature** is `v2_40`
(`var/tauri/crates/tauri-runtime-wry/Cargo.toml:40-42`). At the system level that
binds against the `webkit2gtk-4.1` pkg-config module (the GTK3-based WebKitGTK
API). The earlier doc conflated the pkg-config module name with the engine
version. Net effect is the same — GTK3 + WebKitGTK — but the GTK3 fact is the
load-bearing one for the CVE discussion below.

### 1.2 Engine fragmentation (the M3 frontend concern)

A single M3 fusion frontend (Material Web / Lit + `m3-react`) must render on
**three different engines**: WebKitGTK (Linux, a Safari-family WebKit),
Chromium-based WebView2 (Windows), and WKWebView (macOS/iOS). This is the same
class of cross-browser surface a public website faces, but:

- WebKitGTK lags Chromium on bleeding-edge CSS/JS. Material Web targets evergreen
  browsers; most M3 components work, but anything relying on the newest Chromium
  features (some container-query / `:has()` edge cases, certain Houdini APIs)
  needs testing on WebKitGTK specifically.
- The IPC fast-path thresholds are themselves engine-specific (8192 B on
  WebView2 v135, 1024 B on macOS — `var/tauri/crates/tauri/src/ipc/channel.rs:35-39`),
  proof that Tauri itself special-cases per-engine behaviour.

Mitigation: aphrody's design system is already web-tech and already targets
evergreen browsers via `apps/console`. The incremental risk is "does it render
on WebKitGTK", testable headlessly. This is *lower* risk than a from-scratch
native renderer (which would have to reimplement M3 pixel-for-pixel).

---

## 2. Mobile maturity (iOS / Android)

Tauri v2 ships mobile as a first-class target. In the source:

- `#[cfg(mobile)]` code paths exist throughout core: `app.rs`, `lib.rs`,
  `webview/mod.rs`, `manager/window.rs`, `ipc/channel.rs`, `ipc/mod.rs`
  (grep across `var/tauri/crates/tauri/src/`).
- Dedicated modules: `var/tauri/crates/tauri/src/ios.rs`,
  `var/tauri/crates/tauri/src/path/android.rs`,
  `var/tauri/crates/tauri/src/plugin/mobile.rs`.
- Mobile plugin bridge uses `swift-rs` (iOS) and `jni` (Android)
  (`var/tauri/crates/tauri/Cargo.toml`, UIKit + android blocks).

What is shipped vs. what is rough:
- **Shipped**: window + webview + IPC + commands + channels + plugins all run on
  mobile; the `tauri-cli` has `android`/`ios` subcommands; the example crate is
  built as `["staticlib", "cdylib", "rlib"]`
  (`var/tauri/examples/api/src-tauri/Cargo.toml:9-11`) precisely so it can link
  into the mobile app shells.
- **Rough edges visible in source**: `InvokeBody::Raw` is unsupported on Android
  (`var/tauri/crates/tauri/src/ipc/mod.rs:54-56`) — binary IPC must be base64.
  Mobile pulls a *different* HTTP/TLS stack:
  `reqwest = "0.13"` + `rustls = "0.23"` with the `ring` provider
  (`var/tauri/crates/tauri/Cargo.toml`, the mobile target block). aphrody's
  CLAUDE.md §0.5 flags `reqwest 0.13 aws-lc-sys` as an upstream-blocked area;
  Tauri sidesteps it by selecting the `ring` provider, but it is still a second
  TLS stack to reason about on mobile.

Assessment: desktop is production; mobile is real and usable but younger, and is
**out of aphrody's current priority order** (Linux #1, Windows #2, wasm #3,
macOS best-effort — mobile is not listed). Mobile is upside, not a requirement.

### 2.1 wasm is N/A

Tauri is a desktop/mobile host that *embeds* a webview; it is not something you
compile *to* `wasm32`. aphrody's wasm target (#3) is the `cli` library compiled
to wasm (`run_wasm`, `crates/cli/src/lib.rs:1539`), an orthogonal surface. A
Tauri app would not ship to wasm; it would ship native shells per OS. No
conflict, but no overlap either — the Tauri decision does not touch the wasm
target.

---

## 3. License audit — zero GPL contamination in the Rust tree

aphrody's hard rule: Apache-2.0, zero GPL/LGPL linked into the distributed
binary (CLAUDE.md §7, the `unicorn-engine` ban). Findings:

- **Tauri itself**: `Apache-2.0 OR MIT`
  (`var/tauri/Cargo.toml:44`, `var/tauri/LICENSE.spdx:7-8`,
  `var/tauri/ARCHITECTURE.md:191`).
- **wry, tao**: `MIT OR Apache-2.0` (upstream tauri-apps repos; corroborated by
  web verification — see Sources). `Cargo.lock` shows `wry 0.55.0`, `tao 0.35.0`
  from crates.io.
- **webview2-com / windows crate**: MIT/Apache (Microsoft + tauri-apps).
- **gtk-rs ecosystem** pulled on Linux — `gtk`, `gdk`, `glib`, `cairo-rs`,
  `pango`, `atk`, `gdk-pixbuf`, `gdkx11`, `gdkwayland-sys`, `javascriptcore-rs`,
  `soup3` (all present in `var/tauri/Cargo.lock`): these **Rust bindings are
  MIT-licensed** (gtk-rs project policy, verified — see Sources). They are
  bindings, not the libraries.
- **muda** (menus) `0.19`, **tray-icon** `0.23`
  (`var/tauri/crates/tauri/Cargo.toml`): tauri-apps crates, MIT/Apache.
- **objc2 / objc2-\*** (Apple) `0.6` / `0.3`: MIT.

The supply-chain config imports audits from Google, Mozilla, Bytecode Alliance,
Embark, ISRG, Zcash (`var/tauri/supply-chain/config.toml:6-24`) and treats the
tauri crates as `audit-as-crates-io` (`:26-48`). No GPL/LGPL/AGPL/SSPL/CC-BY
strings appear in `var/tauri/supply-chain/audits.toml` (grep clean).

### 3.1 The one nuance: native system libraries are LGPL — but dynamically linked

On Linux the **C libraries** behind those MIT bindings — GTK3 and WebKitGTK — are
LGPL-2.1 (GTK) and LGPL-2.1/BSD (WebKitGTK). They are **system shared libraries
loaded at runtime via the distro package**, not statically linked into the
aphrody binary. LGPL's dynamic-linking exception means this does **not**
contaminate aphrody's Apache-2.0 license — it is the identical situation to any
Linux app that links libc, GTK, or Qt from the system. This is exactly why
aphrody's `deny.toml` already tolerates the GTK ecosystem (it ignores the GTK3
CVE advisories per CLAUDE.md §7). No new license problem is introduced.

Conclusion: **license gate passes**. No GPL is statically linked. The Rust
dependency tree is uniformly MIT/Apache.

---

## 4. Risk register

| # | Risk | Severity | Evidence | Mitigation |
|---|------|----------|----------|------------|
| R1 | **WebKitGTK CVE stream (Linux #1)** — the Linux webview tracks Apple WebKit security advisories; 2025 alone had WSA-2025-0002..0010 covering dozens of CVEs (memory corruption from crafted web content). Security depends on the *user's* system WebKitGTK package being patched. | High (it is the #1 platform) | `var/tauri/crates/tauri-runtime-wry/Cargo.toml:40-42` (GTK3/WebKitGTK pin); web advisories (Sources) | aphrody renders only its own first-party, embedded frontend (no remote content by default). The ACL origin gating (`authority.rs:57-67`) blocks remote-origin IPC. Keep CSP strict (example: `tauri.conf.json:22-30`). Document the "keep your distro WebKitGTK updated" requirement. The advisory stream is identical for any Linux browser/Electron-on-Linux app — it is not Tauri-specific. |
| R2 | **GTK3 (not GTK4)** on Linux — wry/tao still bind GTK3 (`gtk v3_24`). GTK3 is in long-term maintenance. CLAUDE.md §0.5 flags `wry GTK4` as upstream-blocked (CVE pipeline). | Medium | `var/tauri/crates/tauri-runtime-wry/Cargo.toml:40-41` | aphrody already ignores GTK3 advisories in `deny.toml`. GTK3 is stable and supported on Ubuntu 26.04. Track wry's GTK4 migration but do not block on it. |
| R3 | **Webview engine fragmentation** — M3 frontend must work on WebKitGTK + WebView2 + WKWebView. | Medium | `var/tauri/crates/tauri/src/ipc/channel.rs:35-39` (per-engine special-casing) | Headless cross-engine testing of the M3 components. The design system already targets evergreen web; incremental delta is WebKitGTK quirks only. |
| R4 | **New, heavy dependency tree** — wry/tao/gtk-rs/webkit2gtk are **not currently in `aphrody/Cargo.lock`** (verified: grep returns nothing). Adopting Tauri adds the full gtk-rs + wry/tao tree to the workspace, plus new `cargo deny`/`cargo vet` entries. | Medium | `aphrody/Cargo.lock` (no wry/tao/tauri); `deny.toml` already carries async-std/wasmtime exemptions from the now-extracted `mui-rs` | Isolate Tauri behind a **dedicated crate** (or out-of-workspace) so the core `cli`/`base`/`backend` build stays lean. The UI-heavy crates were *extracted* to `aphrody-ts` on 2026-05-23 precisely to keep the CLI lean — re-introducing wry here would partially undo that unless quarantined. |
| R5 | **No cross-compilation** — each OS builds its own bundle; CI needs a Linux+Windows+macOS matrix. | Low/Medium | `var/tauri/ARCHITECTURE.md:169` | Use the official `tauri-action` GitHub workflow (matrix build). aphrody already builds cross-target via `cargo check --target` per CLAUDE.md §3; the *bundle* step is the new matrix need. |
| R6 | **Build surface grows** — `tauri-cli`, the bundler, and `tauri-build`'s codegen become part of the build. The frontend (Bun) and backend (cargo) must be orchestrated. | Low | `var/tauri/examples/api/src-tauri/tauri.conf.json:6-11` (`beforeBuildCommand`/`devUrl`) | `beforeBuildCommand: "bun run build"` integrates Bun cleanly; `frontendDist` points at the Bun `dist/`. No need for `@tauri-apps/cli` (npm) — `tauri-cli` is a pure Rust binary (`cargo install tauri-cli` / `cargo tauri`). |
| R7 | **stdout/stderr capture gap** for in-process command execution — see [`aphrody-integration.md`](aphrody-integration.md) §3. | Medium (architectural) | `crates/cli/src/lib.rs:1474` (`run_async` inherits stdio); `crates/aphrody-ffi/src/capture.rs:32` (capture lives in FFI crate) | Promote the `with_captured_stdio` helper into a shared crate, or have the Tauri command call `aphrody-ffi`'s captured entry point. Detailed in the integration doc. |
| R8 | **Coupling to webview attack surface for a security tool** — aphrody does reverse-engineering / forensics; a desktop GUI that embeds a webview enlarges the trusted computing base. | Low/Medium | General | The GUI is an *optional* host-only surface, not part of the CLI distribution. The CLI remains the primary, webview-free artifact. Gate the GUI crate so it never enters the `aphrody` binary. |

---

## 5. CVE posture summary (Linux #1)

The single most important operational fact: on Linux, the security of the
rendered content rests on the **system WebKitGTK**. This is a continuous-patch
dependency (WSA advisories land regularly). For aphrody this is *acceptable*
because:

1. The GUI renders only aphrody's own embedded, first-party frontend — no
   arbitrary remote web content is loaded by default.
2. The capability ACL denies IPC to any non-`Local` origin unless explicitly
   granted (`var/tauri/crates/tauri/src/ipc/authority.rs:57-67`).
3. The CLI — aphrody's primary product — is unaffected; the webview only exists
   in the optional desktop GUI shell.
4. This is the same posture as *every* Linux desktop app using a system webview
   (including Electron-on-Linux, GNOME Web, any GTK app). It is not a
   Tauri-specific liability.

It would be a real liability only if aphrody loaded untrusted remote pages into
the webview — which it must not, and the ACL helps enforce that.
