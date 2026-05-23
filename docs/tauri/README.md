<!-- SPDX-License-Identifier: Apache-2.0 -->
# Tauri v2 for aphrody — deep analysis and adoption decision

Source under analysis: **Tauri v2.11.2**, cloned at `var/tauri` (gitignored),
read-only static analysis (no `cargo build`; a release build holds the cargo
lock). Every load-bearing claim cites `var/tauri/<path>:<line>` or an aphrody
workspace path. Companion docs:

- [`architecture.md`](architecture.md) — crate topology, IPC, ACL, codegen, bundler.
- [`risks.md`](risks.md) — cross-platform engines, mobile, **license audit**, risk register.
- [`aphrody-integration.md`](aphrody-integration.md) — the two integration paths, full comparison, crate structure, pre-work.

---

## TL;DR — the decision

**Adopt Tauri v2 as aphrody's desktop GUI shell.** It is the only option that
ships a real cross-platform desktop app (Linux #1, Windows #2, macOS, plus mobile
upside) while rendering aphrody's existing M3 fusion design system **unchanged**,
under a clean **MIT OR Apache-2.0** license with **zero static GPL contamination**.

**Architecture: Path (a) — the Tauri backend depends directly on the `cli`
library and a `#[tauri::command]` calls `aphrody::run_async` in-process, Rust→Rust,
no FFI.** This is lower-latency, lower-footprint, more type-safe, and more
concurrent than routing through the `aphrody-ffi` cdylib (Path b1) or a Bun
sidecar (Path b2). The FFI cdylib + Bun bridge **stay** for the Bun/web/headless
surface — Path (a) simply declines to make the *Rust* GUI take a detour through C.

**First concrete step:** create `crates/aphrody-capture` by lifting the existing
fd-redirection helper out of `aphrody-ffi/src/capture.rs:32` into a shared crate
— this closes the one real gap in Path (a) (`run_async` inherits stdout/stderr;
`crates/cli/src/lib.rs:1474`) and removes duplication.

---

## 1. What Tauri v2 is (architecture in one screen)

Tauri is a Rust toolkit for building desktop/mobile apps with an HTML/JS frontend
rendered in the **OS's own webview** — no bundled browser
(`var/tauri/ARCHITECTURE.md:9-16`). The umbrella `tauri` crate stitches together
a runtime, macros, utilities, and a JS API, reading `tauri.conf.json` at compile
time (`var/tauri/ARCHITECTURE.md:22-26`). The pieces (full detail in
[`architecture.md`](architecture.md)):

- **`tauri`** (core) ↔ **`tauri-runtime`** (abstract trait layer) ↔
  **`tauri-runtime-wry`** (the only concrete runtime, binds **wry**/**tao**,
  `var/tauri/crates/tauri-runtime-wry/src/lib.rs`). wry picks the webview; tao
  owns the window/event-loop (`var/tauri/ARCHITECTURE.md:79-86`).
- **IPC**: `#[tauri::command]` Rust functions (sync/async, typed args, injected
  `State`/`Window`, raw-bytes capable) invoked from JS via
  `window.__TAURI_INTERNALS__.invoke` (`var/tauri/packages/api/src/core.ts:251-257`).
  Transport is a custom `ipc://` POST handler
  (`var/tauri/crates/tauri/src/ipc/protocol.rs:38-130`) **or** an in-webview
  `eval` fast-path for small payloads (< 8 KB JSON on WebView2, < 1 KB raw on
  macOS — `var/tauri/crates/tauri/src/ipc/channel.rs:35-39`). Streaming uses
  `Channel<T>` with ordered reassembly
  (`var/tauri/crates/tauri/src/ipc/channel.rs:48-52`,
  `var/tauri/packages/api/src/core.ts:77-154`). Plus an Events pub/sub channel.
- **Security**: a capability-based ACL. `RuntimeAuthority`
  (`var/tauri/crates/tauri/src/ipc/authority.rs:27-35`) gates every invoke against
  resolved allow/deny command lists, with **origin checks** (Local vs Remote URL,
  `:57-67`) and **scopes** (path globs etc.). Capabilities are JSON files
  (`var/tauri/examples/api/src-tauri/capabilities/run-app.json`); unused commands
  are tree-shaken (`removeUnusedCommands`,
  `var/tauri/examples/api/src-tauri/tauri.conf.json:13`).
- **Build**: `tauri-build` resolves the ACL at compile time
  (`var/tauri/crates/tauri-build/src/lib.rs:528`); `tauri-codegen` embeds the
  built frontend `dist/` into the binary (`var/tauri/ARCHITECTURE.md:32-35`).
  `beforeBuildCommand` + `frontendDist` + `devUrl` orchestrate the JS build/HMR
  (`var/tauri/examples/api/src-tauri/tauri.conf.json:6-11`).
- **Distribution**: `tauri-bundler` produces deb/rpm/AppImage (Linux), MSI/NSIS
  (Windows), dmg/app (macOS) (`var/tauri/crates/tauri-bundler/src/bundle.rs:177-186`);
  an updater plugin polls for signed artifacts
  (`var/tauri/ARCHITECTURE.md:183-187`); sidecars ship external binaries.

---

## 2. Cross-platform reality (the real engines)

No bundled browser — each OS uses its system webview, wired via wry
(detail + matrix in [`risks.md`](risks.md) §1):

- **Linux (#1)**: WebKitGTK on a **GTK3** stack — `webkit2gtk = "=2.0"`/`v2_40`,
  `gtk = "0.18"`/`v3_24` (`var/tauri/crates/tauri-runtime-wry/Cargo.toml:40-42`);
  the webview type is literally `webkit2gtk::WebView`
  (`var/tauri/crates/tauri-runtime-wry/src/webview.rs:13`). (Correction: this is
  the `webkit2gtk-4.1` C API, not "WebKitGTK 4.1" as `gui-options-2026.md:72`
  states — see [`risks.md`](risks.md) §1.1.)
- **Windows (#2)**: WebView2 (Edge/Chromium) — `webview2-com = "0.38"`,
  `windows = "0.61"`; type `ICoreWebView2Controller`
  (`var/tauri/crates/tauri-runtime-wry/src/webview.rs:32-37`,
  `Cargo.toml:30-37`).
- **macOS** (best-effort): WKWebView via `objc2-web-kit`.
- **Mobile** (upside, not in aphrody's priority list): iOS WKWebView
  (`swift-rs`), Android System WebView (`jni`). `#[cfg(mobile)]` paths and
  `ios.rs`/`path/android.rs`/`plugin/mobile.rs` are real and shipped
  ([`risks.md`](risks.md) §2). Caveats: Android forbids raw-bytes IPC
  (`var/tauri/crates/tauri/src/ipc/mod.rs:54-56`) and pulls `reqwest 0.13`+`rustls`
  with the `ring` provider on mobile.
- **wasm (#3)**: N/A — Tauri *hosts* a webview, it is not a wasm compile target.
  aphrody's wasm surface (`run_wasm`, `crates/cli/src/lib.rs:1539`) is orthogonal
  and untouched by this decision.

**Engine fragmentation** is the genuine cross-platform tax: one M3 frontend must
render on WebKitGTK + WebView2 + WKWebView. Because the M3 fusion system is
already evergreen web (Material Web/Lit + `m3-react`), the only incremental risk
is WebKitGTK quirks, which are headlessly testable — far cheaper than a native
renderer reimplementing M3 ([`risks.md`](risks.md) §1.2).

---

## 3. Integration — why Path (a) wins

aphrody made `cli` a library: `aphrody::run_async(args) -> i32` is exit-free,
async, reuses a process-global `GoogleContext`, and never `process::exit`s
(`crates/cli/src/lib.rs:1474-1518`). A Tauri backend is **Rust**, so it can call
this directly. Full comparison in [`aphrody-integration.md`](aphrody-integration.md);
the summary:

| | (a) direct `cli` lib | (b1) via `aphrody-ffi` cdylib | (b2) Bun sidecar |
|--|--|--|--|
| IPC/marshal hops | fewest (Rust call) | + C-ABI round-trip + JSON re-encode | + process/HTTP hop + bun:ffi |
| Output capture | gap → fix with shared capture crate | solved (`aphrody_run_captured`, `aphrody-ffi/src/lib.rs:191`) | solved (most distant) |
| Concurrency | native async, no global lock | serialized on process-wide `capture_lock` | serialized on same lock |
| Footprint | one Rust binary + embedded frontend | + cdylib | + full Bun runtime per OS |
| Latency vs bun:ffi p50 ~0.5 ms | lower (no C marshal; eval fast-path for small results) | higher | highest |

The decisive point: routing a **Rust** GUI through a C ABI (b1) or a separate Bun
process (b2) to reach Rust that is one `use` away is backwards — those bridges
exist for *non-Rust* consumers and **stay in place** for Bun/web/headless
(`apps/console`, `@aphrody-code/native`). Path (a) is lowest-latency,
lowest-footprint, type-safe across the call, and avoids the process-global
capture lock that both (b) variants inherit. Streaming maps cleanly onto Tauri
`Channel` (`var/tauri/crates/tauri/src/ipc/channel.rs:48-52`) for chat / agy-loop
/ SSE.

**The one real gap in (a):** `run_async` inherits stdout/stderr, so a naive
`#[command]` calling it would let the CLI's output escape to the parent terminal
instead of reaching the webview. The fix is small because the fd-redirection
machinery already exists in the FFI crate
(`with_captured_stdio`, `crates/aphrody-ffi/src/capture.rs:32`: Unix `dup2`,
Windows `SetStdHandle`). Promote it to a shared crate so both the Tauri command
and the FFI layer use one implementation.

---

## 4. Licenses — gate passes (zero static GPL)

Full audit in [`risks.md`](risks.md) §3. Summary:

- **Tauri**: `Apache-2.0 OR MIT` (`var/tauri/Cargo.toml:44`,
  `var/tauri/LICENSE.spdx:7-8`).
- **wry, tao, webview2-com, windows, muda, tray-icon, objc2-\***: MIT/Apache.
- **gtk-rs bindings** on Linux (`gtk`/`gdk`/`glib`/`cairo-rs`/`pango`/`atk`/
  `javascriptcore-rs`/`soup3`, all in `var/tauri/Cargo.lock`): the **Rust
  bindings are MIT**.
- The underlying **GTK3 / WebKitGTK C libraries are LGPL**, but they are
  **system shared libraries dynamically linked at runtime** — the LGPL dynamic
  exception means **no contamination** of aphrody's Apache-2.0 binary. This is
  the identical situation to any Linux app linking libc/GTK, and is exactly why
  aphrody's `deny.toml` already tolerates the GTK ecosystem (CLAUDE.md §7).
- No GPL/AGPL/SSPL/CC-BY strings in `var/tauri/supply-chain/audits.toml`; the
  supply-chain imports Google/Mozilla/Bytecode-Alliance/Embark/ISRG/Zcash audit
  feeds (`var/tauri/supply-chain/config.toml:6-24`).

No GPL is statically linked into the aphrody binary. The license rule holds.

---

## 5. Risks and mitigations (top items; full register in [`risks.md`](risks.md) §4)

- **R1 — WebKitGTK CVE stream (Linux #1, High).** The Linux webview tracks Apple
  WebKit advisories (2025: WSA-2025-0002..0010, dozens of CVEs). Security depends
  on the user's patched system WebKitGTK. *Mitigation*: aphrody renders only its
  own embedded first-party frontend (no remote content by default); the ACL
  blocks remote-origin IPC (`var/tauri/crates/tauri/src/ipc/authority.rs:57-67`);
  strict CSP. This posture is identical to every system-webview Linux app — not
  Tauri-specific.
- **R2 — GTK3 (Medium).** wry binds GTK3, not GTK4 (CLAUDE.md flags wry-GTK4 as
  upstream-blocked). Stable on Ubuntu 26.04; advisories already ignored in
  `deny.toml`. Track GTK4 migration, don't block on it.
- **R4 — new heavy dep tree (Medium).** wry/tao/gtk-rs are **not** currently in
  `aphrody/Cargo.lock` (verified). Adopting Tauri adds them. *Mitigation*: isolate
  the Tauri shell in a **build-excluded crate** so the lean CLI core
  (`cli`/`base`/`backend`) and the cross-target binary checks stay untouched —
  mirroring the 2026-05-23 extraction of the heavy UI crates to `aphrody-ts`.
- **R5 — no cross-compilation (Low/Med).** Each OS builds its own bundle; use the
  official `tauri-action` matrix.
- **R7 — stdio capture gap (Medium, architectural).** Addressed by the shared
  capture crate (the first step).

---

## 6. Adoption plan (phased)

**Phase 0 — pre-work (small, no stubs):**
1. `crates/aphrody-capture` — lift `with_captured_stdio` + the capture lock out of
   `aphrody-ffi/src/capture.rs` into a shared crate; `aphrody-ffi` consumes it
   (removes duplication). Cross-platform already implemented.
2. `deny.toml` + `cargo vet` entries for the incoming wry/tao/gtk-rs tree, scoped
   to the GUI crate.

**Phase 1 — minimal Tauri shell (Path a):**
3. `crates/aphrody-app` (workspace member, **excluded from the lean default
   build**): `tauri = {…}`, `aphrody = { path = "../cli" }`, `aphrody-capture`,
   `m3-tokens`/`aphrody-icons` as needed. `build.rs` = `tauri_build::build()`.
4. `tauri.conf.json`: `frontendDist` → the Bun-built `dist/`,
   `beforeBuildCommand` → `bun run build`, `devUrl` for HMR
   (pattern: `var/tauri/examples/api/src-tauri/tauri.conf.json:6-11`).
5. One command: `aphrody_exec(args) -> {code, stdout, stderr}` wrapping
   `aphrody::run_async` inside `aphrody-capture`. A `capabilities/main.json`
   granting only what the GUI needs.

**Phase 2 — frontend + streaming:**
6. Point `frontendDist` at the M3 fusion build (reuse `apps/console`'s frontend
   from `aphrody-ts` as a static `dist/`). Verify the M3 components render on
   WebKitGTK (headless), WebView2, WKWebView.
7. Add `Channel`-based streaming commands for chat / agy-loop / SSE.

**Phase 3 — packaging + (optional) deeper refactor:**
8. `tauri-action` CI matrix → deb/rpm/AppImage + MSI/NSIS + dmg; wire the updater
   plugin with signed artifacts.
9. *(Optional, cleaner long-term)* add `aphrody::run_structured(args) ->
   CommandOutcome` to the `cli` library so structured commands skip stdio capture
   entirely — also upgrades `apps/console` and the FFI bridge.

**Phase 4 — mobile (deferred, upside only):** iOS/Android shells once desktop is
solid; not in the current priority order.

---

## 7. Why not the alternatives (briefly)

- **Native Rust GUI (egui/iced/mui-rs/Vello)**: would have to reimplement the M3
  fusion design system pixel-for-pixel; the heavy native-render attempt
  (`mui-rs*`) was already **extracted** from this repo on 2026-05-23 to keep the
  core lean. The M3 investment is a *web-tech* asset — keep rendering it as web.
- **Electron**: bundles a full Chromium per app (large binary, the opposite of
  aphrody's latency/footprint goals); Tauri's system-webview model is strictly
  smaller.
- **Bun sidecar / `webview-bun`**: viable but redundant once the host is Rust
  (`docs/research/gui-options-2026.md:143-165` agrees) — adds a process hop and a
  whole Bun runtime for no benefit when the backend can call aphrody directly.

---

## Decision (final)

**Yes — adopt Tauri v2.** Architecture **(a)**: a build-excluded
`crates/aphrody-app` whose `#[tauri::command]`s call `aphrody::run_async`
in-process (Rust→Rust, no FFI), serving the existing M3 fusion frontend
(`frontendDist` = Bun `dist/`), with the FFI cdylib + Bun bridge retained for the
non-Rust surface. License gate passes (MIT/Apache, zero static GPL). The principal
risk (Linux WebKitGTK CVE stream) is mitigated because aphrody renders only its
own first-party content behind the capability ACL, and the GUI is an optional
host-only shell that never enters the CLI distribution. **First concrete step:**
create `crates/aphrody-capture` by promoting the existing fd-redirection helper
(`crates/aphrody-ffi/src/capture.rs:32`) to a shared crate, closing the one real
gap in Path (a).

---

## Sources

In-repo evidence is cited inline as `var/tauri/<path>:<line>` and aphrody
workspace paths. External license/CVE verification:

- gtk-rs bindings (glib/gdk/cairo/pango/gtk) MIT — [gtk-rs.org](https://gtk-rs.org/), [gtk-rs-core](https://github.com/gtk-rs/gtk-rs-core), [gtk3-rs](https://github.com/gtk-rs/gtk3-rs)
- wry/tao/tauri MIT OR Apache-2.0; webview2-com integration — [tauri-apps/wry](https://github.com/tauri-apps/wry), [Tauri (Wikipedia)](https://en.wikipedia.org/wiki/Tauri_(software_framework)), [Tauri Architecture](https://v2.tauri.app/concept/architecture/)
- WebKitGTK security advisories (Linux webview CVE stream) — [WSA-2025-0008](https://webkitgtk.org/security/WSA-2025-0008.html), [WSA-2025-0010](https://webkitgtk.org/security/WSA-2025-0010.html), [WSA-2025-0005](https://webkitgtk.org/security/WSA-2025-0005.html)
