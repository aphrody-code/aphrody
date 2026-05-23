<!-- SPDX-License-Identifier: Apache-2.0 -->
# Integrating Tauri v2 into aphrody — the decision core

This is the crux of the adoption question. It compares the two integration paths
the mission defines, with evidence from both `var/tauri` and the aphrody
workspace, and recommends a concrete structure. The final verdict is in
[`README.md`](README.md); the platform/license/risk basis is in
[`risks.md`](risks.md) and the mechanics in [`architecture.md`](architecture.md).

---

## 1. What aphrody already exposes (the inputs)

The `cli` crate became a **library** with an exit-free, async core:

- `aphrody::run_async<I,T>(args) -> i32` — installs the rustls `ring` provider,
  parses argv with clap, builds a process-global `GoogleContext`, dispatches,
  returns the exit code. **Never calls `process::exit`.** stdout/stderr are
  **inherited** (not captured). `crates/cli/src/lib.rs:1474-1518`.
- `aphrody::run_from_args(args) -> i32` — sync wrapper that builds its own
  multi-thread tokio runtime and `block_on`s `run_async`.
  `crates/cli/src/lib.rs:1433-1449`.
- `dispatch(ctx, cli) -> miette::Result<()>` is the internal command router
  (`crates/cli/src/lib.rs:1003`) — private; the public surface is argv-in /
  exit-code-out only. There is **no public structured (JSON) result API**.
- `GoogleContext` is a process-global `OnceLock`
  (`crates/cli/src/lib.rs:1456-1464`), `Send + Sync`, built once and reused — so
  many commands per process is cheap (no per-call VFS/mirror rebuild).

The FFI surface (`crates/aphrody-ffi`) wraps that core for C/Bun:

- `aphrody_run(argc, argv) -> c_int` and `aphrody_run_json(json) -> c_int` —
  forward to `aphrody::run_async` on a persistent process-wide tokio runtime
  (`crates/aphrody-ffi/src/lib.rs:134,161,282`).
- **`aphrody_run_captured(args_json) -> *mut c_char`** — the important one. It
  takes a process-wide capture lock, runs the command with
  `with_captured_stdio(...)`, and returns a JSON `{code, out, err}` string
  (`crates/aphrody-ffi/src/lib.rs:191-210`).
- `with_captured_stdio` (`crates/aphrody-ffi/src/capture.rs:32`) does fd-level
  redirection: `dup2` over fds 1/2 on Unix, `SetStdHandle` on Windows
  (`capture.rs:5-16`). FFI call cost measured at p50 ~0.5 ms
  (`docs/research/gui-options-2026.md`).

Frontend assets: the M3 fusion system — Material Web (Lit) + `@aphrody-code/m3-react`
(React 19) + tokens `@aphrody-code/theme` (from `aphrody design tokens --fusion`),
Bun toolchain. The current live UI is `apps/console` (`Bun.serve` + browser).
The canonical token/icon crates `m3-tokens` and `aphrody-icons` **remain in this
repo** (`crates/m3-tokens`, `crates/aphrody-icons`); the heavier native GUI
crates (`gui` = winit+wry+wgpu, `mui-rs*`) were **extracted** to `aphrody-ts`
on 2026-05-23 to keep the CLI build lean.

---

## 2. The two paths

### Path (a) — Tauri Rust backend depends DIRECTLY on the `cli` library, `#[command]` calls `aphrody::run_async`

A new Rust crate is a Tauri app. It has `tauri = { ... }` and
`aphrody = { path = "../cli" }` as dependencies. A command does, in spirit:

```rust
#[tauri::command]
async fn aphrody_exec(args: Vec<String>) -> Result<i32, String> {
    // calls into the cli library on the Tauri async runtime
    Ok(aphrody::run_async(args).await)
}
```

Rust→Rust, in-process, **no FFI boundary**, no C ABI marshalling, no JSON
double-encode across a `*const c_char`. The webview's `invoke('aphrody_exec', …)`
reaches this command over Tauri's `ipc://` transport
(`var/tauri/crates/tauri/src/ipc/protocol.rs:38-130`), or the eval fast-path for
small results (`var/tauri/crates/tauri/src/ipc/channel.rs:35-39`).

**The catch — and it is real:** `run_async` returns only an `i32` and **inherits
stdout/stderr**. The CLI's actual output (its `println!`/tables/JSON written to
stdout) would escape to the *parent process's* terminal, not be captured for the
webview. The capture machinery (`with_captured_stdio`) lives in the **FFI crate**
(`crates/aphrody-ffi/src/capture.rs:32`), not in `cli`. So Path (a) as literally
described — "call `run_async` in a command" — gives you the exit code but **loses
the output**. To make Path (a) usable you must either:
- **(a1)** lift `with_captured_stdio` into a shared crate (e.g. a new
  `aphrody-capture` or into `base`) and have the Tauri command wrap
  `run_async` in it, returning `{code, stdout, stderr}`; or
- **(a2)** add a real structured-result API to the `cli` library (a
  `run_structured(args) -> CommandOutcome` that returns data instead of printing)
  — a larger refactor, but the cleanest long-term shape and the one that also
  benefits `apps/console` and the FFI bridge.

### Path (b) — reuse the `aphrody-ffi` cdylib / the Bun server

Two sub-variants:

- **(b1) Tauri Rust backend → `aphrody-ffi`**: the Tauri command calls
  `aphrody_run_captured(json)` and forwards the `{code,out,err}` JSON to the
  webview. This *works today* — capture is solved
  (`crates/aphrody-ffi/src/lib.rs:191-210`) — but it routes Rust→C-ABI→Rust
  inside a single process, which is pure overhead (you are calling a C function
  that immediately re-enters Rust and re-parses JSON). It also serializes all
  calls behind the process-wide `capture_lock` (`lib.rs:197`), since fd
  redirection is process-global.
- **(b2) Tauri shell + Bun sidecar / Bun server**: ship `apps/console`'s
  `Bun.serve` as a sidecar binary; the webview talks HTTP to localhost Bun, which
  calls `@aphrody-code/native` (bun:ffi) into the cdylib. This duplicates a whole
  runtime (Bun standalone exe per OS, ~tens of MB) and adds an out-of-process hop
  for no benefit when the Tauri backend is *already Rust* and can reach aphrody
  directly. `docs/research/gui-options-2026.md:143-165` reaches the same
  conclusion: the Bun sidecar is "often redundant" once the host is Rust.

---

## 3. Comparison and verdict between (a) and (b)

| Dimension | Path (a) — direct `cli` lib | Path (b1) — via `aphrody-ffi` | Path (b2) — Bun sidecar |
|-----------|------------------------------|-------------------------------|--------------------------|
| **IPC hops** | webview → Tauri IPC → Rust fn (1 logical hop). | webview → Tauri IPC → Rust → C ABI → Rust (extra C round-trip). | webview → Tauri IPC → Rust → spawn/HTTP → Bun → bun:ffi → C ABI → Rust (worst). |
| **Output capture** | **Gap**: `run_async` inherits stdio; needs (a1) shared capture or (a2) structured API. | **Solved today** via `aphrody_run_captured`. | Solved (Bun captures), but most distant. |
| **Per-call overhead** | Lowest — a normal async Rust call; `GoogleContext` already cached. | + JSON marshal across C ABI + process-wide `capture_lock` serialization. | + process spawn / HTTP localhost + bun:ffi p50 ~0.5 ms + capture_lock. |
| **Concurrency** | Native async; multiple commands can run concurrently on the tokio runtime (no global lock). | Serialized behind `capture_lock` (fd redirection is process-global). | Serialized behind the same FFI capture_lock. |
| **Binary footprint** | One Rust binary (Tauri + cli + frontend embedded). No Bun, no extra cdylib at runtime. | Same Rust binary, but also links/loads the `aphrody-ffi` cdylib. | Tauri shell + a full Bun standalone exe shipped as sidecar (largest). |
| **Type safety** | Full Rust types across the call (or structured `CommandOutcome` in a2). | Stringly-typed JSON over C ABI. | Stringly-typed JSON over HTTP. |
| **Reuses existing FFI bridge** | No (intentionally — the FFI bridge stays for Bun/web/headless). | Yes. | Yes. |
| **Latency vs current bun:ffi (p50 ~0.5 ms)** | Tauri IPC adds the `ipc://` round-trip *only above the eval threshold* (8 KB JSON / 1 KB raw, `channel.rs:35-39`); below it, eval is in-webview and comparable. The Rust call itself is cheaper than the FFI path. | Strictly slower than (a): same Tauri IPC + the FFI overhead. | Slowest. |

**Verdict: Path (a), specifically (a1) now and (a2) as the clean follow-up.**

Rationale:
1. The Tauri backend is **Rust**. Routing through a C ABI (b1) or a separate Bun
   process (b2) to reach Rust that is one `use` away is architecturally
   backwards — it exists only to serve *non-Rust* consumers (Bun/web/headless),
   which keep using `aphrody-ffi` unchanged.
2. (a) is the lowest-latency, lowest-footprint, most type-safe path, and it
   avoids the process-global `capture_lock` serialization that both (b) variants
   inherit — letting independent commands run concurrently on tokio.
3. The only thing (a) lacks today is output capture, and the fix is small:
   the redirection logic already exists (`crates/aphrody-ffi/src/capture.rs`);
   promote it to a shared crate so both the Tauri command and the FFI layer use
   one implementation (removing today's duplication). That is strictly better
   than depending on the FFI crate just for capture.
4. The FFI cdylib + `@aphrody-code/native` Bun bridge **remain** for the
   Bun/web/headless surface (`apps/console`, scripts). Path (a) does not delete
   them; it just declines to route the *Tauri* GUI through them.

The IPC latency tradeoff is acceptable: Tauri's own design routes small results
(< 8 KB on WebView2) through in-webview `eval` rather than the `ipc://` fetch
(`var/tauri/crates/tauri/src/ipc/channel.rs:35-39`), so for typical command
results the added cost over raw bun:ffi is the webview message dispatch, not an
HTTP round-trip. For streaming output, Tauri `Channel`
(`var/tauri/crates/tauri/src/ipc/channel.rs:48-52`) maps directly onto aphrody's
streaming commands (chat, agy-loop, SSE) — a capability the FFI string-return
path does not have.

---

## 4. Crate structure

Recommended: a **dedicated crate inside the workspace but excluded from the lean
default build set**, mirroring how the heavy UI crates were handled before
extraction. Two viable shapes:

### Option 1 (recommended) — `crates/aphrody-app`, workspace member, host-only feature-gated

```
crates/aphrody-app/           # the Tauri desktop/mobile shell
  Cargo.toml                  # tauri = {…}, aphrody = { path = "../cli" }, m3-tokens, aphrody-icons
  build.rs                    # tauri_build::build()
  tauri.conf.json             # frontendDist -> the Bun dist/, beforeBuildCommand -> bun run build
  capabilities/main.json      # ACL: grant only the commands the GUI needs
  src/lib.rs                  # #[tauri::command]s wrapping the cli lib
  src/main.rs                 # tauri::Builder::default()…run()
  ui/  (or reference aphrody-ts) # the M3 fusion frontend build output target
```

- It depends on `cli` directly (Path a). It links `m3-tokens` + `aphrody-icons`
  only if it needs token values Rust-side; otherwise tokens flow through the
  frontend (`@aphrody-code/theme`).
- **Keep it out of the default lean workspace build** the same way the old GUI
  crates were excluded, so `cargo ci-offline` on the CLI core does not pull
  wry/tao/gtk-rs (Risk R4 in [`risks.md`](risks.md)). Either exclude it from the
  workspace `default-members`, or gate the whole crate behind a host-only build
  so Linux #1 / Windows #2 / wasm #3 CI of the *binary* is untouched.

### Option 2 — out-of-workspace sibling (`C:\src\aphrody-app`)

Mirror the 2026-05-23 extraction philosophy: keep `aphrody` (this repo) strictly
the CLI, and put the Tauri shell in a sibling repo that depends on the published
`aphrody` crate (or a path dep during dev). Cleanest separation, but loses
in-tree atomic refactors and complicates the shared-capture-crate move. Given
that Path (a) wants a *shared* capture crate living in `aphrody`, Option 1
(in-workspace, build-excluded) is the better fit.

**Recommendation: Option 1.** It keeps the Rust→Rust integration in one
workspace (enabling the `aphrody-capture` shared-crate refactor), while
build-exclusion preserves the lean core that the extraction was meant to protect.

---

## 5. Serving the M3 frontend + token sharing

- **Build wiring**: set `tauri.conf.json` `build.frontendDist` to the Bun build
  output (e.g. `../ui/dist`) and `build.beforeBuildCommand` to `bun run build`
  (pattern: `var/tauri/examples/api/src-tauri/tauri.conf.json:6-11`, which uses
  `pnpm build`). In dev, `build.devUrl` points at the Bun dev server for HMR.
  `tauri-codegen` embeds the built `dist/` into the binary at compile time
  (`var/tauri/ARCHITECTURE.md:32-35`) — release ships a single self-contained
  binary, no external server.
- **Frontend source**: reuse the existing M3 fusion frontend. The cleanest is to
  have `apps/console`'s frontend (in `aphrody-ts`) emit a static `dist/` that
  `aphrody-app` consumes as `frontendDist`. The same Lit/`m3-react` components
  render in the webview unchanged (they are evergreen-web; see WebKitGTK note in
  [`risks.md`](risks.md) §1.2).
- **Tokens**: `@aphrody-code/theme` (generated by `aphrody design tokens --fusion`)
  is consumed by the frontend exactly as today — Tauri serves whatever the Bun
  build produces. If any Rust-side surface needs token *values* (e.g. a native
  menu accent), read them from the in-repo `crates/m3-tokens` crate. So tokens
  have one source (`aphrody design tokens`) feeding both the web layer
  (`@aphrody-code/theme`) and the Rust layer (`m3-tokens`).
- **`withGlobalTauri`**: if the Lit frontend prefers `window.__TAURI__` over ESM
  imports, set `app.withGlobalTauri: true`
  (`var/tauri/examples/api/src-tauri/tauri.conf.json:17`).

---

## 6. Latency and binary size — concrete expectations

- **Latency**: a GUI command is `webview.invoke` → (eval fast-path for < 8 KB
  results, else `ipc://` fetch) → Tauri dispatch → `aphrody::run_async`. The Rust
  call is cheaper than today's bun:ffi path (no C marshalling). For small
  results, the in-webview eval keeps it sub-millisecond-class on the transport;
  the dominant cost is the command's own work, identical to the CLI. Streaming
  uses `Channel` (incremental, ordered) rather than one big return.
- **Binary size**: the release profile is size-tuned
  (`var/tauri/Cargo.toml:49-55`: `opt-level="s"`, `lto`, `strip`,
  `panic="abort"`). Tauri ships **no browser** — the webview is the OS's — so the
  binary is dominated by aphrody's own code + the embedded frontend assets, not
  by a bundled Chromium. This is the key advantage over Electron and over a
  Bun-sidecar topology (which ships a whole Bun runtime per OS).
- **`removeUnusedCommands: true`**
  (`var/tauri/examples/api/src-tauri/tauri.conf.json:13`) tree-shakes any
  `#[command]` not granted by a capability — smaller binary and smaller IPC
  attack surface.

---

## 7. The required pre-work for Path (a)

A short, well-scoped list (no stubs):

1. **`crates/aphrody-capture`** (new, tiny): move `with_captured_stdio` +
   `CaptureLock` out of `aphrody-ffi/src/capture.rs` into a shared crate.
   `aphrody-ffi` depends on it (removing duplication); `aphrody-app` depends on
   it for its commands. Cross-platform: Unix `dup2`, Windows `SetStdHandle`
   (already implemented, `capture.rs:5-16`).
2. **(optional, better) `aphrody::run_structured`** in the `cli` library: a
   public async fn returning a `CommandOutcome { code, stdout, stderr }` (or a
   richer typed result for commands that can yield structured data), so the GUI
   does not rely on stdio capture at all for those commands. This also upgrades
   `apps/console` and the FFI bridge. Larger, can follow phase 1.
3. **`crates/aphrody-app`**: the Tauri shell (Option 1 above), build-excluded
   from the lean workspace; commands wrap `run_async` via `aphrody-capture` (and
   later `run_structured`); `Channel`-based streaming commands for chat/agy-loop.
4. **CI matrix** for the bundle step (Linux/Windows/macOS) via `tauri-action`;
   the existing `cargo check --target` gates for the *binary* stay as-is.
5. **`deny.toml` / `cargo vet`** entries for the new wry/tao/gtk-rs tree, scoped
   to the `aphrody-app` crate so the core stays clean (Risk R4).
