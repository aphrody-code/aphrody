# Rust → WASM Fundamentals

Source : wasm-bindgen 0.2+ official docs, verified 2026-05-17.

## Minimal Cargo.toml setup

```toml
[lib]
crate-type = ["cdylib", "rlib"]

[dependencies]
wasm-bindgen = "0.2"
console_error_panic_hook = { version = "0.1", optional = true }
js-sys = "0.3"
web-sys = { version = "0.3", features = ["console", "Document", "Element", "Window"] }

[features]
default = ["console_error_panic_hook"]
```

`web-sys` is feature-gated per-API for build-size sanity. Only enable what you call.

## Exposing functions to JS

```rust
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn add(a: u32, b: u32) -> u32 {
    a + b
}

#[wasm_bindgen]
pub struct Counter { value: i32 }

#[wasm_bindgen]
impl Counter {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self { Self { value: 0 } }
    pub fn increment(&mut self) -> i32 { self.value += 1; self.value }
}
```

JS side (after `wasm-pack build --target web`):

```js
import init, { add, Counter } from './pkg/my_crate.js'
await init()
console.log(add(2, 3))
const c = new Counter()
c.increment()
```

## Panic hook — MANDATORY in dev/test builds

Without this, panics in WASM produce useless `RuntimeError: unreachable` in the browser console. Install once at entry :

```rust
#[wasm_bindgen(start)]
pub fn main() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}
```

Disable for release builds to shave ~20 KB.

## Async functions

`async fn` exported via `#[wasm_bindgen]` returns a JS Promise transparently. Import direction is symmetric : a JS async function imported into Rust returns an `impl Future`.

```rust
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(catch)]                       // catch makes it Result<_, JsValue>
    async fn fetch_user(id: u32) -> Result<JsValue, JsValue>;
}

#[wasm_bindgen]
pub async fn load_user(id: u32) -> Result<JsValue, JsValue> {
    fetch_user(id).await
}
```

Use `wasm-bindgen-futures::spawn_local` for fire-and-forget tasks.

## Rust ↔ JS data — use `serde-wasm-bindgen`

JSON round-trip is slow and bloats bundle. `serde-wasm-bindgen` marshals directly to native JS Object/Array/Number :

```rust
use serde::{Serialize, Deserialize};
use wasm_bindgen::JsValue;

#[derive(Serialize, Deserialize)]
pub struct Tile { x: i32, y: i32, color: String }

#[wasm_bindgen]
pub fn tiles_from_js(input: JsValue) -> Result<JsValue, JsValue> {
    let tiles: Vec<Tile> = serde_wasm_bindgen::from_value(input)?;
    let processed: Vec<Tile> = tiles.into_iter().map(transform).collect();
    Ok(serde_wasm_bindgen::to_value(&processed)?)
}
```

Roughly **3-5x faster** than `JsValue::from_serde / into_serde` (which use JSON internally).

## Allocator choice

`wee_alloc` (the historical "tiny" allocator) is **archived since 2024** — do not use, it has known memory-corruption issues. Stick with the default Rust allocator. If bundle size is the priority, use `wasm-opt -Oz` instead.

## `wasm-bindgen` build targets

| Target flag | Output | When to use |
|---|---|---|
| `--target web` | ES modules with `init()` to bootstrap | Direct `<script type="module">` in plain HTML |
| `--target bundler` | ES modules without bootstrap | Webpack, Vite, esbuild, Turbopack, Bun bundling |
| `--target nodejs` | CommonJS for Node-style loaders | Node.js, Bun runtime, Deno-node-compat |
| `--target no-modules` | Single global IIFE | Legacy embedding, no module system |

Pick `bundler` for Next.js + Turbopack/Webpack pipelines (it generates the smallest glue).

## Unstable web APIs (`web_sys_unstable_apis`)

Many recent web APIs (WebGPU, WebTransport, parts of the File System Access API) are gated behind a compile-time cfg. Enable in `.cargo/config.toml` so it's project-wide :

```toml
[build]
rustflags = ["--cfg=web_sys_unstable_apis"]
```

Don't pass it ad-hoc on the command line for libraries — downstream crates won't see the flag and their `#[cfg(web_sys_unstable_apis)]` blocks will silently disappear.

## Profiling & debugging

- **Source maps** : pass `--keep-debug` to `wasm-bindgen` then `wasm2wat` to inspect.
- **Browser DevTools** : Chrome / Edge can step into Rust source if `wasm-bindgen --debug` is used.
- **Performance** : use `web-sys::Performance` to instrument hot paths. Avoid `console.log` in tight loops — boundary-crossing is expensive.

## Common pitfalls

1. **`String` ↔ `JsValue::from_str`** allocates twice (Rust heap + JS heap). For hot paths, pre-allocate or use `&str` borrowed views.
2. **`Vec<u8>` boundary** : use `js_sys::Uint8Array::view(&rust_bytes)` for zero-copy view (read-only on JS side, valid until next allocation in WASM memory).
3. **Closures crossing the boundary** must be `Closure::wrap`'d *and* their lifetime managed (`.forget()` if permanently registered, otherwise store on a struct field).
4. **WASM memory grows by linear pages** of 64 KiB. Plan capacity ahead with `Vec::with_capacity` to avoid grow-thrashing.
