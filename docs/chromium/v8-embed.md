<!-- SPDX-License-Identifier: Apache-2.0 -->
# Embedding V8

Source: <https://v8.dev/docs/embed> (fetched 2026-05-22, distilled — no verbatim code)

## Linking the monolith

After `ninja -C out\x64.release v8_monolith`, link an embedder against the
single static lib:

- Libraries: `v8_monolith` (monolithic build folds in `v8_libbase` +
  `v8_libplatform`).
- Includes: the repo root and `include/` (`-I. -Iinclude`).
- Compiler: C++20, **`-fno-rtti`**, and the defines matching your `args.gn`
  (e.g. `V8_COMPRESS_POINTERS` only if you enabled it; **do not** define
  `V8_ENABLE_SANDBOX` for our sandbox-off build).
- Runtime data: ship `icudtl.dat` (ICU) unless ICU is disabled; with
  `v8_use_external_startup_data = false` the snapshot is already embedded.

On Windows/MSVC the equivalent flags are `/std:c++20 /GR-` and linking
`v8_monolith.lib` plus the system libs V8 pulls in (`winmm`, `dbghelp`, etc.).

## Core object model

| Concept | One-liner |
|---------|-----------|
| **Isolate** | An isolated VM instance with its own heap. Usually one per app/thread. |
| **Context** | A sandboxed JS execution environment inside an Isolate; you must *enter* it before running script. |
| **HandleScope** | A stack-scoped container that owns handles and frees them on destruction. |
| **`Local<T>`** | Stack handle to a JS object; dies with its `HandleScope`. |
| **`Persistent<T>`** | Long-lived handle; freed explicitly via `Reset()`. |
| **`EscapableHandleScope`** | Lets a function return one `Local` to the outer scope via `Escape()`. |

## Minimal hello-world flow

1. Initialise the platform + `Isolate`.
2. Open a `HandleScope`.
3. Create a `Context` and enter it.
4. Compile a source string (`Script::Compile`) and `Run()` it.
5. Wrap in `TryCatch` for exceptions; convert the result to a UTF-8 string.
6. Leave the scope/context; dispose the `Isolate` and platform.

C++ objects are exposed to JS via **object templates** with internal fields and
accessor/method callbacks.

## aphrody integration target

The monolith is the bridge for a future Rust↔V8 FFI surface (mirrors how
`bxc_rust_bridge.dll` wraps DOM parsing). Keep the sandbox **off** and STL =
MSVC so the lib links cleanly into the Rust/MSVC toolchain. See
[`aphrody-v8-state.md`](aphrody-v8-state.md).
