<!-- SPDX-License-Identifier: Apache-2.0 -->
# Rust ↔ V8 — the official binding (`rusty_v8`)

Source: <https://github.com/denoland/rusty_v8> · crate <https://crates.io/crates/v8>
(checked 2026-05-22)

The canonical, officially-maintained Rust binding to V8 is the **`v8` crate**
(repo name `rusty_v8`), maintained by the **Deno** team. It is what Deno and
most Rust JS-engine work build on — there is no competing "official" binding.

- Crate: `v8` — **latest `149.0.0`** (149.1.0 was yanked).
- It mirrors the V8 C++ API (`Isolate`, `Context`, `HandleScope`, `Local<T>`,
  `Global<T>`, `FunctionTemplate`, …) in safe-ish Rust.

## Two ways to get the V8 library

### A. Prebuilt (default, recommended for aphrody)

By default the crate's build script **downloads a prebuilt static lib** from the
GitHub release matching the crate version — **Windows MSVC (`x86_64-pc-windows-
msvc`) is a supported target**. So the fastest path is simply:

```toml
# Cargo.toml
[dependencies]
v8 = "149"
```

```bash
cargo build      # downloads librusty_v8 prebuilt, links it, done
```

No depot_tools, no gn, no ninja required for this path.

### B. From source

To compile V8 from rusty_v8's own pinned checkout (e.g. to patch V8, or on an
unsupported target):

```bash
V8_FROM_SOURCE=1 cargo build -vv
```

This needs depot_tools-style prerequisites and a Python 3.

## Relevant env vars

| Var | Purpose |
|-----|---------|
| `V8_FROM_SOURCE=1` | build V8 from source instead of downloading a prebuilt lib |
| `RUSTY_V8_MIRROR` | alternate download base (URL or file path) for the prebuilt lib |
| `RUSTY_V8_ARCHIVE` | use a **specific** prebuilt archive (URL or **local path**) instead of downloading |
| `GN_ARGS` | extra `gn` args for the from-source build |
| `CLANG_BASE_PATH` | existing LLVM/clang dir (skip the auto clang download) |
| `PYTHON` | Python 3 binary (required for source builds) |

## How our `C:\src\v8` build fits in

The hand-built `v8_monolith` (see [`v8-build.md`](v8-build.md) /
[`aphrody-v8-state.md`](aphrody-v8-state.md)) is **not strictly required** if we
take path A. Its value:

- **Offline / pinned**: package the resulting lib and point
  `RUSTY_V8_ARCHIVE=C:\path\to\librusty_v8.a` (or a static `.lib`) so CI never
  hits GitHub releases.
- **Custom V8 args / patches**: drive a from-source build and pass our
  `args.gn` choices via `GN_ARGS`.

> Decision for aphrody: **default to the prebuilt `v8` crate** for the
> Rust↔V8 surface (zero build cost, MSVC target supported). Keep the
> `C:\src\v8` from-source build as the patch/offline fallback feeding
> `RUSTY_V8_ARCHIVE`. This mirrors the `bxc_rust_bridge.dll` FFI pattern: a thin
> Rust wrapper over a native engine.
