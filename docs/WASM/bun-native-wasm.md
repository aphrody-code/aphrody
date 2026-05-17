# Bun — Native WASM Loading

Source : Bun 1.3+ runtime docs, verified 2026-05-17.

Bun is the **fastest runtime** in the aphrody stack to load WASM, mostly because there is no bundler / loader configuration to fight. WASM is a first-class import.

## Direct `.wasm` import

```ts
// src/api/handler.ts
import { add, Counter } from './engine.wasm'

console.log(add(2, 3))
const c = new Counter()
c.increment()
```

Bun parses the import, instantiates the module synchronously at module load, and re-exports its declared symbols. No `init()` boilerplate.

This works because Bun ships a built-in **wasm loader** that wraps `WebAssembly.instantiate` and uses the wasm-bindgen JS shim if it's adjacent (`engine.js` next to `engine.wasm`).

## `wasm-pack` output integration

```bash
wasm-pack build --target bundler --out-dir ./pkg crates/my-crate
```

Then in Bun :

```ts
import init, { Counter, transform } from './pkg/my_crate.js'

await init()                       // wasm-bindgen target=bundler still needs init()
const c = new Counter()
console.log(transform("hi"))
```

For maximum speed, prefer `--target nodejs` when shipping to a Bun runtime — it avoids the bundler glue :

```bash
wasm-pack build --target nodejs --out-dir ./pkg crates/my-crate
```

```ts
const { Counter, transform } = require('./pkg/my_crate')
```

## `bunfig.toml` — preload-style optimization

If a WASM module is on the hot path, pre-instantiate at server boot :

```toml
# bunfig.toml
preload = ["./src/preload-wasm.ts"]
```

```ts
// src/preload-wasm.ts
import './heavy.wasm'                // instantiated once, cached for the rest of the process
```

## Async `WebAssembly.instantiate` — when you need control

For arbitrary `.wasm` files (not `wasm-bindgen` output) :

```ts
const wasmBytes = await Bun.file('./module.wasm').arrayBuffer()
const { instance } = await WebAssembly.instantiate(wasmBytes, {
  env: {
    // import object — fill with the imports declared in the WASM module
    abort: () => { throw new Error('wasm abort') },
    log: (ptr: number, len: number) => {
      const mem = new Uint8Array((instance.exports as any).memory.buffer, ptr, len)
      console.log(new TextDecoder().decode(mem))
    },
  },
})

const exports = instance.exports as Record<string, Function>
console.log(exports.compute(42))
```

## Streaming compile (large modules)

```ts
const response = await fetch('https://cdn.example/heavy.wasm')
const { instance } = await WebAssembly.instantiateStreaming(response, importObject)
```

Bun supports `instantiateStreaming` since 1.0 — use it for anything > 1 MB.

## SharedArrayBuffer — threading

```toml
# bunfig.toml
[serve.static]
"Cross-Origin-Opener-Policy" = "same-origin"
"Cross-Origin-Embedder-Policy" = "require-corp"
```

These headers unlock `SharedArrayBuffer`, which is what `wasm-bindgen-rayon` needs for browser parallelism. On the Bun server side, threads work through `worker_threads` (Node-compat).

## Build-time pipeline (cross-platform)

Build once on each platform of the matrix, ship from one place :

```jsonc
// package.json
{
  "scripts": {
    "wasm:build:linux": "cargo build --release --target wasm32-unknown-unknown --manifest-path crates/my-crate/Cargo.toml",
    "wasm:bindgen":     "wasm-bindgen --target bundler --out-dir pkg target/wasm32-unknown-unknown/release/my_crate.wasm",
    "wasm:opt":         "wasm-opt -Oz pkg/my_crate_bg.wasm -o pkg/my_crate_bg.wasm",
    "build":            "bun run wasm:build:linux && bun run wasm:bindgen && bun run wasm:opt && tsc"
  }
}
```

`wasm-opt -Oz` after `wasm-bindgen` typically halves the bundle size.

## Comparison vs Node / Deno

| Runtime | `.wasm` import | Loader needed | wasm-bindgen target |
|---------|----------------|---------------|---------------------|
| Bun 1.3+ | ✓ Native | None | `nodejs` or `bundler` |
| Node 22+ | ✗ Need loader hook or `--experimental-wasm-modules` | Yes | `nodejs` |
| Deno 1.40+ | ✓ Native via `import` | None | `bundler` |

Per the org policy (`feedback-bun-only` memory), Bun is the only allowed runtime — Node is banned. WASM workflow is therefore Bun-first.

## Profiling Bun + WASM

```bash
bun --inspect server.ts
```

Hits the Chrome DevTools inspector. WASM frames show up in the perf timeline with Rust function names if you built with `wasm-bindgen --debug`.
