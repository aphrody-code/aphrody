# WASM Tooling Reference

Pinned versions verified 2026-05-17. Every tool below is open-source and free.

## Build pipeline

```
cargo build --target wasm32-…
    ↓
wasm-bindgen <output>.wasm --target <web|bundler|nodejs|no-modules>
    ↓
wasm-opt -Oz <pkg>/_bg.wasm   (binaryen)
    ↓
deliver to bundler (Webpack, Bun, Vite)
```

`wasm-pack` automates the first two steps. Use it for the common case ; drop to raw `cargo + wasm-bindgen` only when you need an exotic combination.

## wasm-pack

```bash
cargo install wasm-pack --version "^0.13"

# Output to pkg/ next to Cargo.toml
wasm-pack build --target bundler --release crates/my-crate

# With wasm-opt baked in (default in release builds)
wasm-pack build --release --target bundler

# Generate TypeScript .d.ts (default)
wasm-pack build --release --target bundler --out-name my_crate
```

Flags worth knowing :
- `--target` : `web | bundler | nodejs | no-modules | deno` — pick once, don't mix.
- `--out-dir` : default `pkg/` ; override per-workspace if you have many crates.
- `--scope <npm-org>` : sets `@npm-org/crate-name` in the generated package.json.
- `--no-default-features` / `--features ...` : pass through to cargo.
- `--dev` vs `--release` : dev keeps debug symbols, useful for source-mapped stack traces.

## wasm-opt (binaryen)

The single biggest size win after `cargo build --release`. Typical results :

| Optimization | Bundle size delta |
|---|---|
| `wasm-opt -O0` | baseline |
| `wasm-opt -O3` | -15 to -30% |
| `wasm-opt -O4` | -20 to -40% |
| `wasm-opt -Oz` | smallest, -30 to -50% (favors size over speed) |

```bash
cargo install wasm-opt              # Rust port of binaryen's wasm-opt
# OR (Linux native binaryen)
sudo apt install binaryen           # Ubuntu 24.04+
# OR (Windows)
winget install WebAssembly.Binaryen
```

Run after wasm-bindgen :
```bash
wasm-opt -Oz pkg/my_crate_bg.wasm -o pkg/my_crate_bg.wasm
```

## twiggy — bundle size analyzer

```bash
cargo install twiggy

twiggy top -n 20 pkg/my_crate_bg.wasm        # heaviest items
twiggy dominators pkg/my_crate_bg.wasm       # what's keeping symbols alive
twiggy diff baseline.wasm pkg/my_crate_bg.wasm  # PR-time size diff
```

Wire `twiggy diff` into CI :

```yaml
- name: Bundle size regression
  run: |
    cargo build --release --target wasm32-unknown-unknown
    wasm-bindgen --target bundler --out-dir new target/wasm32-unknown-unknown/release/my_crate.wasm
    twiggy diff baseline/my_crate_bg.wasm new/my_crate_bg.wasm | tee diff.txt
    [ -z "$(grep '^+' diff.txt | head -1)" ] || echo "::warning::WASM bundle grew"
```

## wasm-bindgen-cli — version MUST match the crate exactly

The CLI version must match the `wasm-bindgen` crate version in `Cargo.toml`
**byte-for-byte**. The bindgen schema changes on every publish — mismatches
produce errors at instantiation time, sometimes silently broken bindings.

Current pin (2026-05-17) : `0.2.121` (released 2026-05-07).

```bash
# One-shot install matching the workspace
cargo install wasm-bindgen-cli --version "=0.2.121" --locked
```

To keep the workspace and CLI in sync automatically, the aphrody root provides :

```bash
# Reads the exact resolved version from Cargo.lock and installs it
bash scripts/install-wasm-bindgen-cli.sh
```

The workspace pins the crate exactly :

```toml
# Cargo.toml [workspace.dependencies]
wasm-bindgen = "=0.2.121"      # EXACT — do not relax
```

Why exact and not caret ? Because Cargo's caret `"0.2"` resolves to the latest
0.2.x at lock time, and a freshly cloned dev machine running
`cargo install wasm-bindgen-cli` (without `--version`) will pull a *different*
latest, producing schema-mismatch errors only at runtime in the browser.

The error looks like :
```
wasm-bindgen: schema version mismatch ; expected 0.2.121, found 0.2.122
```

When bumping wasm-bindgen :

1. Update `wasm-bindgen = "=NEW.VER.SION"` in the root `Cargo.toml`.
2. `cargo update -p wasm-bindgen`
3. `scripts/install-wasm-bindgen-cli.sh` (or `cargo install wasm-bindgen-cli --version =NEW.VER.SION --force`).
4. Re-run any `wasm-pack build` to regenerate the JS shims.

## wasm-tools — inspection

```bash
cargo install wasm-tools

wasm-tools print module.wasm | head            # WAT (text format)
wasm-tools validate module.wasm                # spec compliance
wasm-tools strip module.wasm -o stripped.wasm  # remove custom sections
wasm-tools component new …                     # Component Model authoring
```

## Component Model (post-MVP WASM)

Cargo-component for authoring components :
```bash
cargo install cargo-component
cargo component new my-component --lib
cargo component build --release
```

Worth keeping an eye on — the Component Model unifies interface bindings across host runtimes. Not production-default yet on browsers ; **WASI Preview 2** runtimes (wasmtime 18+, jco) are the early consumers.

## wasm-bindgen-rayon (parallel iter in browser)

```toml
[dependencies]
wasm-bindgen-rayon = "1.3"
rayon = "1.12"
```

Requires the page to be served with `Cross-Origin-Opener-Policy: same-origin` + `Cross-Origin-Embedder-Policy: require-corp` (SharedArrayBuffer).

```rust
use wasm_bindgen::prelude::*;
use rayon::prelude::*;

#[wasm_bindgen]
pub async fn init_threads(num_threads: usize) -> Result<(), JsValue> {
    wasm_bindgen_rayon::init_thread_pool(num_threads).await
}

#[wasm_bindgen]
pub fn sum(input: &[u32]) -> u64 {
    input.par_iter().map(|x| *x as u64).sum()
}
```

JS side :
```js
await init()
await wasm.init_threads(navigator.hardwareConcurrency)
console.log(wasm.sum(new Uint32Array([1,2,3,4,5])))
```

## Source maps & DevTools

```bash
wasm-bindgen --keep-debug …          # preserves DWARF debug info
```

Chrome / Edge DevTools can step into Rust source if the project served the matching `.wasm` + `.wasm.map` + the original `.rs` files reachable on the dev server. Production builds should drop `--keep-debug` and add `strip = true` to `[profile.release]` to save space.

## Recommended `[profile.release]`

```toml
[profile.release]
opt-level = 3            # speed-leaning ; switch to "z" for size-leaning
lto = "thin"             # or "fat" for absolute minimum
codegen-units = 1        # better optimizations, slower build
strip = true             # drop symbols
panic = "abort"          # smaller, no unwind tables
```

Then `wasm-opt -Oz` on top.
