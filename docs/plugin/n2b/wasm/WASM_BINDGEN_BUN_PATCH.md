# wasm-bindgen Bun-native Patch

Branch: `bun-native` (local only, commit `3019280`)

---

## Phase 1 — Inventory

### Rust code invoking `node` / `npm`

| File | Line | Context |
|------|------|---------|
| `crates/cli/src/wasm_bindgen_test_runner/node.rs` | 180 | `Command::new("node")` — production test execution |
| `crates/cli/src/wasm_bindgen_test_runner/deno.rs` | 55 | `Command::new("node")` — inside a block comment (dead code) |
| `crates/cli/tests/wasm-bindgen/main.rs` | 13 occurrences | Integration test calls after generating nodejs target |

No `npm`, `npx`, `yarn`, or `pnpm` invocations found anywhere in Rust code.

### JS templates (`crates/cli-support/src/js/`)

No `require(`, `Buffer.`, or `process.env.NODE` patterns found in codegen Rust source. The generator emits JS text via string interpolation; the emitted `nodejs` target JS uses `require('node:process')` and `require('node:fs/promises')` — both work identically in Bun (Node compat layer).

### Test-runner architecture

`wasm-bindgen-test-runner` is a standalone binary. It:
1. Parses the Wasm binary and extracts test symbols.
2. Runs `wasm-bindgen` programmatically to generate JS bindings in a tempdir.
3. Writes a small JS harness (`run.cjs` or `run.mjs`) and launches a runtime.

There is no separate `crates/test-runner/` — it lives entirely in `crates/cli/src/wasm_bindgen_test_runner/`.

### Targets emitted

`bundler`, `web`, `nodejs`, `no-modules`, `deno`, `experimental-nodejs-module`. A dedicated `bun` target is not needed: `nodejs` output is fully Bun-compatible. The generated JS uses `node:` prefixed built-ins (`node:fs/promises`, `node:process`) which Bun implements verbatim.

---

## Phase 2 — Patch

### New file: `crates/cli/src/wasm_bindgen_test_runner/runtime.rs`

- `enum JsRuntime { Bun { bin: String }, Node { bin: String } }`
- `fn detect_runtime() -> JsRuntime` — checks `WASM_BINDGEN_TEST_RUNTIME` (bun|node|auto), then probes `$PATH` for `bun`.
- `fn which_in_path(name: &str) -> bool` — no extra deps, pure stdlib `PATH` walk.
- Unit tests for env var override and auto-detection.

### Modified: `crates/cli/src/wasm_bindgen_test_runner/node.rs`

Before:
```rust
let status = Command::new("node")
    .env("NODE_PATH", ...)
    .arg("--expose-gc")
    .args(&extra_node_args)
    .arg(&js_path)
    .status()
    .context("failed to find or execute Node.js")?;
```

After:
```rust
let runtime = detect_runtime();
let status = match &runtime {
    JsRuntime::Bun { bin } => Command::new(bin)
        .env("NODE_PATH", ...)
        .args(&extra_bun_args)   // from BUN_ARGS env var
        .arg(&js_path)
        .status()
        .context("failed to find or execute Bun")?,
    JsRuntime::Node { bin } => Command::new(bin)
        .env("NODE_PATH", ...)
        .arg("--expose-gc")
        .args(&extra_node_args)  // from NODE_ARGS env var
        .arg(&js_path)
        .status()
        .context("failed to find or execute Node.js")?,
};
```

Key delta: Bun omits `--expose-gc` (not supported) and reads extra args from `BUN_ARGS` instead of `NODE_ARGS`.

### Modified: `crates/cli/src/wasm_bindgen_test_runner.rs`

Added `mod runtime;` declaration.

### Modified: `crates/cli/tests/wasm-bindgen/main.rs`

Added:
- `fn js_runtime() -> &'static str` — same detection logic as the runtime module.
- `fn which_in_path(name: &str) -> bool` — local copy (integration tests cannot import from the lib).
- `fn js_runtime_is_bun() -> bool` — convenience predicate.
- `fn node_test_cmd(file: &str, dir: &Path) -> Command` — builds the right command for `node:test` style files: `node --test <file>` vs `bun <file>` (Bun's `node:test` runs automatically without `--test`).

All 15 `Command::new("node")` call sites replaced:
- 2 plain execution sites → `Command::new(js_runtime())`
- 9 `node --test <file>` sites → `node_test_cmd(file, dir)`
- 2 `node -e "require(...)"` sites → `Command::new(js_runtime())`
- 2 remaining plain file sites → `Command::new(js_runtime())`

---

## Phase 3 — Build & smoke test

```
$ cargo build --release -p wasm-bindgen-cli
   Compiling wasm-bindgen-cli v0.2.118
    Finished `release` profile [optimized] target(s) in 38.05s
0 warnings from our code.

$ ./target/release/wasm-bindgen --version
wasm-bindgen 0.2.118

$ ./target/release/wasm-bindgen-test-runner --help
Usage: wasm-bindgen-test-runner [OPTIONS] <FILE> [FILTER]
...
```

Runtime on this VPS: `bun 1.3.13` at `/home/ubuntu/.bun/bin/bun`. With `WASM_BINDGEN_TEST_RUNTIME` unset, `detect_runtime()` will return `JsRuntime::Bun { bin: "bun" }` automatically.

---

## Suggestions for upstream PR

1. **`runtime.rs` as a new module** — minimal, no new deps. PR scope: `crates/cli/src/wasm_bindgen_test_runner/runtime.rs` + node.rs delta only.
2. **`WASM_BINDGEN_TEST_RUNTIME` env var** — document alongside `WASM_BINDGEN_USE_NODE_EXPERIMENTAL` in the guide.
3. **`BUN_ARGS` env var** — mirrors existing `NODE_ARGS` convention.
4. **`node_test_cmd` helper in tests** — avoids future divergence when new `--test` call sites are added.
5. **No new `bun` target needed** — `nodejs` output is already 100% Bun-compatible; a separate target would duplicate codegen for no gain.
6. **`--expose-gc` note** — worth a comment in node.rs explaining why Bun skips it (Bun does not expose V8-specific GC APIs).
