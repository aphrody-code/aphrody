<!-- SPDX-License-Identifier: Apache-2.0 -->
# Tauri v2 architecture — deep read (v2.11.2)

Static analysis of the Tauri source cloned at `var/tauri` (gitignored), tag
v2.11.2. Read-only. Every claim is cited as `var/tauri/<path>:<line>`.
This document maps the moving parts; the integration decision lives in
[`aphrody-integration.md`](aphrody-integration.md) and the verdict in
[`README.md`](README.md).

---

## 1. Crate topology

The workspace declares its members in `var/tauri/Cargo.toml:2-36`. The crates
that matter for an embedder:

| Crate | Role | Evidence |
|-------|------|----------|
| `tauri` | The umbrella crate. Brings runtime + macros + utils + API together, reads `tauri.conf.json` at compile time, hosts the IPC, manages updates. | `var/tauri/ARCHITECTURE.md:22-26`; src tree `var/tauri/crates/tauri/src/` |
| `tauri-runtime` | Abstract glue between `tauri` and a lower-level webview library. Pure trait layer. | `var/tauri/ARCHITECTURE.md:41-43` |
| `tauri-runtime-wry` | Concrete `tauri-runtime` impl bound to wry/tao. The only shipped runtime. | `var/tauri/ARCHITECTURE.md:45-47`; `var/tauri/crates/tauri-runtime-wry/src/lib.rs` |
| `tauri-build` | Build-script side. Resolves capabilities, emits codegen, rigs cargo. | `var/tauri/crates/tauri-build/src/lib.rs:528` (`acl::build`) |
| `tauri-codegen` | Embeds/hashes/compresses assets + icons, parses `tauri.conf.json` into a `Config` struct at compile time. | `var/tauri/ARCHITECTURE.md:32-35` |
| `tauri-macros` | `#[command]`, `generate_handler!`, `generate_context!` — thin wrappers over `tauri-codegen`. | `var/tauri/ARCHITECTURE.md:37-39` |
| `tauri-utils` | Shared config parsing, platform-triple detection, CSP injection, asset management, the ACL types. | `var/tauri/ARCHITECTURE.md:49-51` |
| `tauri-plugin` | Plugin authoring scaffolding (build-side permission inlining). | `var/tauri/Cargo.toml:10` |
| `tauri-bundler` | Produces platform installers (deb/rpm/AppImage, MSI/NSIS, dmg/app). | `var/tauri/crates/tauri-bundler/src/bundle.rs:177-186` |
| `tauri-cli` | The Rust executable behind `cargo tauri` / `@tauri-apps/cli`. dev/build/info/bundle. | `var/tauri/ARCHITECTURE.md:67-69` |

Two upstream crates are external (separate repos, vendored only in `Cargo.lock`):
**tao** (window/event-loop, a winit fork) and **wry** (webview abstraction).
`var/tauri/ARCHITECTURE.md:79-86`.

`tauri`'s own dependency wiring: `tauri-runtime` `2.11.2`, `tauri-macros`
`2.6.2`, `tauri-utils` `2.9.2`, `tauri-runtime-wry` `2.11.2` (optional, behind
the `wry` feature). `var/tauri/crates/tauri/Cargo.toml:58-66`.

---

## 2. The IPC model

This is the heart of any embedding. Tauri's IPC is a request/response plus a
streaming channel, transported over a custom URI scheme.

### 2.1 Commands (`#[tauri::command]`)

A command is a Rust `fn` (sync or async) annotated with `#[tauri::command]`.
The macro turns it into an `InvokeHandler`. Examples in
`var/tauri/examples/commands/main.rs:31-55` show the four shapes: sync,
`async fn`, `#[command(async)]` returning `impl Future`, and stateful commands
taking `State<'_, T>`. Commands can take typed args (deserialized from the JSON
body), `Window`/`Webview`/`AppHandle`/`State` injected params, and a raw
`ipc::Request` (`var/tauri/crates/tauri/src/ipc/mod.rs:165-173`). They return
anything `Serialize` (`IpcResponse` blanket impl,
`var/tauri/crates/tauri/src/ipc/mod.rs:181-187`) or an `ipc::Response` wrapping
raw bytes (`var/tauri/crates/tauri/src/ipc/mod.rs:190-205`).

The invoke payload is either JSON or raw bytes
(`InvokeBody::{Json,Raw}`, `var/tauri/crates/tauri/src/ipc/mod.rs:59-64`).
**Android caveat**: `InvokeBody::Raw` is not supported on Android — the enum is
always `Json` there; the docs recommend base64-in-a-`String` for binary payloads
(`var/tauri/crates/tauri/src/ipc/mod.rs:54-56`).

### 2.2 The transport: `ipc://` custom protocol + direct eval

The JS side calls `window.__TAURI_INTERNALS__.invoke(cmd, args, options)`
(`var/tauri/packages/api/src/core.ts:251-257`). Under the hood this either:

- **POSTs to the `ipc://` custom protocol** — the Rust handler is
  `protocol::get()` in `var/tauri/crates/tauri/src/ipc/protocol.rs:38-130`. It
  reads `Tauri-Callback`, `Tauri-Error`, `Tauri-Invoke-Key` headers
  (`var/tauri/crates/tauri/src/ipc/protocol.rs:24-26`), dispatches via
  `webview.on_message`, and responds with a `Tauri-Response: ok|error` header
  (`:28-30`, `:107-110`). The protocol handler sets permissive CORS for the
  webview (`:48-57`). This is wired into wry via `with_asynchronous_custom_protocol`
  (`var/tauri/crates/tauri-runtime-wry/src/lib.rs:5197`).
- **or is `eval`'d directly** for small payloads. The threshold logic lives in
  `var/tauri/crates/tauri/src/ipc/channel.rs:35-39`: JSON under **8192 bytes**
  runs ~2x faster through `eval` than through fetch on WebView2 v135; raw under
  **1024 bytes** runs ~30% faster through eval on macOS. So small command
  results skip the HTTP round-trip entirely. This is the performance-tuned
  fast-path and it directly informs the latency analysis.

The IPC handler itself is registered with the webview via `with_ipc_handler`
(`var/tauri/crates/tauri-runtime-wry/src/lib.rs:5163-5169`,
`create_ipc_handler` at `:5383-5392`), and an init script is injected with
`with_initialization_script_for_main_only`
(`var/tauri/crates/tauri-runtime-wry/src/lib.rs:5174`).

### 2.3 Channels — the streaming primitive

`Channel<T>` (`var/tauri/crates/tauri/src/ipc/channel.rs:48-52`) is a Rust
handle the backend holds and pushes messages into; the JS side receives them in
order via a callback. The JS `Channel` class
(`var/tauri/packages/api/src/core.ts:77-154`) reassembles messages by index
(`#nextMessageIndex`, `#pendingMessages`) so out-of-order delivery is corrected,
and handles an `end` sentinel when the Rust channel is dropped
(`:98-105`, `ChannelInner::drop` at
`var/tauri/crates/tauri/src/ipc/channel.rs:80-86`). Wire format prefix is
`__CHANNEL__:` (`var/tauri/crates/tauri/src/ipc/channel.rs:28`,
`var/tauri/packages/api/src/core.ts:146-148`).

This is exactly the shape aphrody needs for streaming token output (chat / agy
streaming SSE / agy-loop progress).

### 2.4 Events

A separate pub/sub channel (`var/tauri/crates/tauri/src/event/`) for
backend-to-frontend and frontend-to-frontend broadcast, distinct from the
request/response commands. Used for app lifecycle and arbitrary signalling.

---

## 3. Security model — capabilities / permissions ACL (v2)

Tauri v2's headline change over v1: a capability-based Access Control List. This
is enforced at runtime by `RuntimeAuthority`
(`var/tauri/crates/tauri/src/ipc/authority.rs:27-35`), which holds
`allowed_commands` and `denied_commands` maps plus a `ScopeManager`. Every IPC
invoke is checked against the resolved ACL
(`Invoke.acl: Option<Vec<ResolvedCommand>>`,
`var/tauri/crates/tauri/src/ipc/mod.rs:218-220`).

- **Origin gating**: each command is bound to an `ExecutionContext` —
  `Local` or `Remote { url }`. The `Origin::matches` check
  (`var/tauri/crates/tauri/src/ipc/authority.rs:57-67`) means a command exposed
  to local content is *not* automatically callable from a remote URL loaded in
  the webview. Remote origins must match a URL pattern.
- **Capabilities are JSON files** under `src-tauri/capabilities/`. A concrete
  example: `var/tauri/examples/api/src-tauri/capabilities/run-app.json` lists
  granular permissions like `core:window:allow-set-title`,
  `core:webview:allow-print`, scoped permissions
  (`{"identifier":"allow-log-operation","allow":[{"event":"tauri-click"}]}`,
  `:9-16`), and plugin permissions (`sample:allow-ping-scoped`, `:21`). The
  default deny posture means a command is unreachable unless a capability grants
  it.
- **Scopes**: `ScopeObject` / `CommandScope` / `GlobalScope`
  (`var/tauri/crates/tauri/src/ipc/mod.rs:34-36`) let a permission carry
  allow/deny data (e.g. filesystem path globs). The asset-protocol scope in
  `var/tauri/examples/api/src-tauri/tauri.conf.json:31-37` shows
  `allow: ["$APPDATA/db/**"]` with `deny: ["$APPDATA/db/*.stronghold"]`.
- **Resolution at build time**: `tauri-build` resolves the capability files
  against plugin manifests and bakes a `Resolved` ACL
  (`acl::build`, `var/tauri/crates/tauri-build/src/lib.rs:528`). The
  `removeUnusedCommands` build flag
  (`var/tauri/examples/api/src-tauri/tauri.conf.json:13`) tree-shakes commands
  not referenced by any capability — smaller binary, smaller attack surface.
- **Dynamic ACL** (`dynamic-acl` feature, on by default per
  `var/tauri/crates/tauri/Cargo.toml:209-216`) keeps the raw manifests around so
  capabilities can be built at runtime (`CapabilityBuilder`,
  `var/tauri/crates/tauri/src/ipc/mod.rs:37-38`).

There is also an **isolation pattern** (`isolation` feature) that runs a
sandboxed iframe to intercept and sign IPC before it reaches Rust
(`var/tauri/examples/api/src-tauri/tauri.conf.json:16-21`, the
`"pattern":{"use":"isolation"}` block; example at `var/tauri/examples/isolation/`).

---

## 4. The JS API surface (`packages/api`)

`@tauri-apps/api` is the TS package the frontend imports. Modules in
`var/tauri/packages/api/src/`: `core.ts` (invoke/Channel/Resource/convertFileSrc),
`event.ts`, `window.ts`, `webview.ts`, `webviewWindow.ts`, `app.ts`, `dpi.ts`,
`menu.ts`, `tray.ts`, `path.ts`, `image.ts`, plus `mocks.ts` for testing.

Key entry points in `core.ts`:
- `invoke<T>(cmd, args, options)` — `:251-257`.
- `Channel<T>` — streaming, `:77-154`.
- `convertFileSrc(filePath, protocol='asset')` — turns a device path into an
  `asset://` URL the webview can load (needs CSP + asset-protocol scope),
  `:289-291`.
- `Resource` — a handle to Rust-side state in the `resources_table`, freed via
  `plugin:resources|close`, `:315-335`.
- `isTauri()` — feature-detect the Tauri runtime, `:337-340`.

When `withGlobalTauri: true`
(`var/tauri/examples/api/src-tauri/tauri.conf.json:17`), this API is also exposed
on `window.__TAURI__` without a bundler import — relevant for a Lit/web-component
frontend that prefers globals over ESM imports.

---

## 5. Compile-time pipeline (`tauri-build` + `tauri-codegen`)

1. `build.rs` calls `tauri_build::build()`. It locates capability files via a
   glob (`capabilities_path_pattern`,
   `var/tauri/crates/tauri-build/src/lib.rs:376-385`), resolves the ACL
   (`:528`), and emits cargo `rerun-if-changed` instructions.
2. `tauri-codegen` parses `tauri.conf.json` into a `Config` struct and embeds
   the frontend assets (the `frontendDist` directory) — hashed and optionally
   compressed (`var/tauri/ARCHITECTURE.md:32-35`). For aphrody this is the
   mechanism that bakes the Bun-built `dist/` into the binary.
3. `generate_context!` (a `tauri-macros` macro) produces the runtime `Context`
   carrying the embedded assets + resolved ACL; `generate_handler![...]`
   produces the `InvokeHandler` and the matching `RuntimeAuthority`
   (`runtime_authority!` macro, `var/tauri/crates/tauri/src/ipc/authority.rs:80-101`).

The frontend is **embedded into the binary** in release builds (no external web
server, no Chromium). In dev, `devUrl` + `beforeDevCommand` run the JS dev server
for HMR (`var/tauri/examples/api/src-tauri/tauri.conf.json:6-11`).

---

## 6. Updater, sidecar, bundler

- **Updater**: poll a configured server for a new signed artifact; download,
  verify checksum + signature, replace, restart
  (`var/tauri/ARCHITECTURE.md:183-187`). Now a plugin in v2
  (`tauri-plugin-updater`, not in this monorepo).
- **Sidecar**: ship an external binary alongside the app and spawn it (the
  bundler embeds it as a resource). This is how a "Tauri + Bun sidecar" topology
  would ship a standalone Bun executable. Bundler resource handling in
  `var/tauri/crates/tauri-bundler`.
- **Bundler targets** (`var/tauri/crates/tauri-bundler/src/bundle.rs:177-186`):
  Linux `Deb`/`Rpm`/`AppImage`, Windows `WindowsMsi`/`Nsis`, macOS `dmg`/`app`.
  Updater artifacts require one of `app`/`appimage`/`msi`/`nsis`
  (`:236`). **No cross-compilation** — each OS builds its own installer
  (`var/tauri/ARCHITECTURE.md:169`), so CI needs a matrix (the official
  `tauri-action` GitHub workflow does this).

---

## 7. Build profile (release)

The workspace release profile is aggressively size-tuned
(`var/tauri/Cargo.toml:49-55`): `panic = "abort"`, `codegen-units = 1`,
`lto = true`, `opt-level = "s"`, `strip = true`. This is the source of the "tiny
binary" claim — the final size is dominated by aphrody's own code, not Tauri,
because the webview is the OS's.
