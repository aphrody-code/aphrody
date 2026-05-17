<!-- SPDX-License-Identifier: Apache-2.0 -->
# WASM Build Targets

Rust offers four wasm targets. Pick the right one — they are not interchangeable.

## Decision matrix

| Target | When | Runtime | Filesystem | Sockets | Threads |
|--------|------|---------|------------|---------|---------|
| `wasm32-unknown-unknown` | Browser, edge | Browser, Edge runtimes (Vercel, Cloudflare Workers, Deno Deploy) | None | None | Via Web Workers + SharedArrayBuffer |
| `wasm32-wasi` (alias `wasm32-wasip1`) | Server-side serverless, CLI sandbox | wasmtime, wasmer, Node `--experimental-wasi`, Bun | Yes, virtualised | No | No (Preview 1) |
| `wasm32-wasip2` | Modern WASI Component Model | wasmtime 18+, jco | Yes | Yes (Sockets API) | Limited |
| `wasm32-unknown-emscripten` | Legacy C/C++ port shim | Browser via Emscripten runtime | Emulated | Emulated | Yes (pthreads) |

For aphrody, the only two that matter are :
- `wasm32-unknown-unknown` — for browser, edge, embedded WASM modules.
- `wasm32-wasi` — for server-side WASM (none currently planned, but kept compatible).

## Why not `wasm32-unknown-emscripten`

Emscripten bundles a large runtime (~100 KB of glue JS) and assumes a POSIX-ish environment. Rust toolchain support is best-effort. We treat it as legacy — don't add new targets.

## `wasm32-unknown-unknown` — characteristics

- **No standard library facilities for I/O.** `println!` is a no-op unless you wire it to `console.log` via `web-sys`.
- **`std::time::Instant` panics.** Use `web_sys::Performance::now()` instead.
- **`std::thread::spawn` panics.** Use `wasm-bindgen-rayon` or `web_sys::Worker`.
- **`std::sync::Mutex` works**, but lock-free types (`AtomicBool`, etc.) are preferred — no kernel involvement.
- **Allocation** is via the default Rust allocator — `wee_alloc` is archived, don't use it.

## `wasm32-wasi` — characteristics

- **POSIX-like API.** `std::fs`, `std::env`, `std::time` work as expected within the WASI sandbox.
- **No networking in Preview 1.** Need wasip2 for sockets.
- **No threading in Preview 1.**
- **Capabilities-based security.** Host runtime grants filesystem mounts, env vars explicitly (`wasmtime --dir=. --env FOO=bar program.wasm`).

## Cross-compiling — install targets

```bash
rustup target add wasm32-unknown-unknown
rustup target add wasm32-wasi               # Preview 1
rustup target add wasm32-wasip2             # Preview 2 (nightly only as of 2026-05)
```

## Cargo per-target deps

Conditional dependencies based on target keep the workspace clean :

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
web-sys = { version = "0.3", features = ["console"] }

[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
tokio = { version = "1.52", features = ["full"] }
```

## CI matrix

```yaml
strategy:
  matrix:
    target: [wasm32-unknown-unknown, wasm32-wasi]
runs-on: ubuntu-latest
steps:
  - uses: dtolnay/rust-toolchain@stable
    with:
      targets: ${{ matrix.target }}
  - run: cargo check --workspace --target ${{ matrix.target }}
```

The aphrody-code/.github org template ships `ci-rust-wasm.yml` ready to copy into the 3 WASM-eligible repos.

## Per-crate opt-out

Some crates in a workspace will never compile to wasm32 (anything depending on `tokio rt-multi-thread`, `mio`, native FS notify, etc.). To prevent CI from failing on them :

```toml
# In the non-portable crate's Cargo.toml
[package.metadata.docs.rs]
targets = ["x86_64-unknown-linux-gnu"]
```

Or `--exclude` the crate in the wasm CI step :

```bash
cargo check --workspace --exclude mrx-watch --exclude mrx-cli --target wasm32-unknown-unknown
```

## When to NOT build wasm

If the crate :
- Spawns OS threads.
- Calls into `windows::Win32::*` or `nix::*` syscalls.
- Uses `tokio` with `rt-multi-thread`.
- Watches the filesystem via `notify` (the underlying inotify/ReadDirectoryChangesW/FSEvents APIs aren't available).
- Talks to native FFI (`bindgen`, raw `extern "C"` to system libs).

→ **don't add wasm to its CI**. It will fail. The non-portable crate is fine — just leave it Linux/Windows.
