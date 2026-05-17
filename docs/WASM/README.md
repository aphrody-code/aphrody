# WASM — Index

Production WebAssembly reference for the **aphrody** ecosystem. Sourced from upstream library docs (wasm-bindgen 0.2+, wgpu 26+/29, Next.js 16, Bun 1.3+) verified 2026-05-17 via Context7.

## Scope policy

WASM is a **first-class target only** for these three repos :

| Repo | Why |
|------|-----|
| `aphrody-code/next.js` (fork canary) | Edge runtime, WASM-friendly server APIs |
| `aphrody-code/ui` (fork shadcn-ui → MD3 native) | Browser-side component rendering |
| `aphrody-code/A2UI` (to be rewritten in Rust) | UI shell shipped as WASM module |

Every other repo in `aphrody-code/*` ships Linux + Windows binaries only. Do **not** add WASM CI to backend / systems crates — `tokio rt-multi-thread`, FS notify, OS process control don't compile to `wasm32-unknown-unknown`.

## Decision tree — should this crate target WASM?

```
Does the crate need a browser surface (DOM, WebGPU canvas, Web Workers, IndexedDB)?
├─ Yes → target wasm32-unknown-unknown + wasm-bindgen
│        Use web-sys / js-sys for browser APIs
│
├─ Does it need POSIX-like filesystem & networking?
│   ├─ Yes → target wasm32-wasi (WASI Preview 1)
│   └─ No  → wasm32-unknown-unknown is enough
│
└─ No browser, no WASI, just compute → don't ship WASM. Linux/Windows native is faster.
```

## File map

| File | Coverage |
|------|----------|
| [`rust-wasm-fundamentals.md`](rust-wasm-fundamentals.md) | wasm-bindgen attributes, panic hook, allocator, async/Promise bridge |
| [`wgpu-webgpu.md`](wgpu-webgpu.md) | Instance/Adapter/Device/Queue/Surface init, WebGL2 fallback |
| [`nextjs-integration.md`](nextjs-integration.md) | asyncWebAssembly, edge vs node runtime, Turbopack 16 status |
| [`bun-native-wasm.md`](bun-native-wasm.md) | `import "./mod.wasm"`, WebAssembly.instantiate, bunfig |
| [`tooling.md`](tooling.md) | wasm-pack, wasm-opt, twiggy, snippets, profiling |
| [`build-targets.md`](build-targets.md) | wasm32-unknown-unknown vs wasm32-wasi vs wasm32-wasip1 vs wasm32-unknown-emscripten |

## Stack pinned versions (2026-05-17, verified web + GitHub API)

| Crate / tool | Version | Notes |
|---|---|---|
| `wasm-bindgen` | **0.2.121** (2026-05-07) | Pin **exact** in Cargo.toml — schema must match CLI |
| `wasm-bindgen-cli` | **=0.2.121** | `cargo install wasm-bindgen-cli --version =0.2.121` |
| `web-sys` | 0.3.76+ | Feature-gated per browser API ; `web_sys_unstable_apis` cfg for WebGPU |
| `js-sys` | 0.3.76+ | ES intrinsics (Array, Date, Function, Promise, …) |
| `serde-wasm-bindgen` | 0.6+ | Native Rust ↔ JS object marshalling (3-5x faster than JSON) |
| `wasm-bindgen-rayon` | 1.3+ | Rayon parallel iter via Web Workers + SharedArrayBuffer (COOP/COEP required) |
| `wasm-pack` | 0.13+ | Build + bundle (target web/bundler/nodejs/no-modules) |
| `wgpu` | **26.0.x stable** | `29.0.3` released 2026-03-26 but has major breaking changes — see [`wgpu-webgpu.md`](wgpu-webgpu.md) |
| `wasm-opt` (binaryen) | 121+ | `-O4` for release ; `-Oz` for smallest bundle (-30 to -50 %) |
| `twiggy` | 0.7+ | Bundle size analyzer + CI diff gate |
| Next.js | 16.2 | **Turbopack still does NOT support WASM bundling** — `next dev --webpack` required for now |
| Bun | 1.3.14+ | Native `import` of `.wasm` (no loader), `WebAssembly.instantiateStreaming` |

## ❌ Banned

| Crate | Why |
|-------|-----|
| `wee_alloc` | **Repo archived** (GitHub `archived: true`, last push 2023-02-28). Unbounded memory leak (issue #106). Pages never returned to host. Replacement : default Rust allocator + `wasm-opt -Oz` for size. |

## Toolchain bootstrap

```bash
# Once per machine
rustup target add wasm32-unknown-unknown wasm32-wasi
cargo install wasm-bindgen-cli wasm-pack twiggy
# binaryen (provides wasm-opt) — Linux/macOS package mgr, or :
cargo install wasm-opt   # Rust wrapper bin

# Per-project (Bun)
bun add -D @wasm-tool/wasm-pack-plugin

# Per-project (Next.js with WASM)
# next.config.ts opt-in shown in nextjs-integration.md
```

Related : [`../SOURCE_OF_TRUTH.md`](../SOURCE_OF_TRUTH.md), [`../PLAN.md`](../PLAN.md), CLAUDE.md §2 (Language policy), feedback memory `aphrody-ultimate-goals` (WASM scope rule).
