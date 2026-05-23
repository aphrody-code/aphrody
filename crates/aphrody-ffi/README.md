<!-- SPDX-License-Identifier: Apache-2.0 -->
# aphrody-ffi

**aphrody, as a native library.** A thin, stable **C ABI** over the `aphrody`
library crate so the *entire* command surface is reachable in-process from Bun
(`bun:ffi`) and any other C-ABI host (C / C++ / Zig / Python `ctypes` / .NET
P/Invoke) — not only by spawning the `aphrody` binary.

## How it works

The `aphrody` package (`crates/cli/`) is split into a **library** (`src/lib.rs`,
all command logic) and a **thin binary** (`src/main.rs`, ~25 lines). This crate
depends on that library and wraps its async entry point — `aphrody::run_async`,
driven on a persistent `tokio` runtime — behind a handful of `extern "C"`
symbols. Because cargo builds only the library target of a dependency, the
binary's `#[global_allocator]` is absent here and `aphrody-ffi` installs its own
`mimalloc`.

The wrapped command surface is **exit-free** (commands return a `SubprocessExit`
error rather than calling `process::exit`) and every FFI entry **catches
panics**, so running aphrody inside a host process never tears the host down.

## Exported symbols

| Symbol | Purpose |
|---|---|
| `uint32_t aphrody_abi_version(void)` | ABI version for the host to check. |
| `const char *aphrody_version(void)` | aphrody version (owned by lib, no free). |
| `int aphrody_run(int argc, const char *const *argv)` | run any command, inherited stdio. |
| `int aphrody_run_json(const char *args_json)` | same, args as a JSON array. |
| `char *aphrody_run_captured(const char *args_json)` | run + capture stdout/stderr; returns `{"code","stdout","stderr"}` JSON (free with `aphrody_string_free`). |
| `void aphrody_string_free(char *ptr)` | free a string this library returned. |
| `const char *aphrody_last_error(void)` | last error on this thread (owned by lib, no free). |

Arguments exclude the program name — a synthetic `argv[0]` is prepended
internally — e.g. `["doctor", "--json"]`. The C header is `include/aphrody.h`.

## The C header

`include/aphrody.h` is a hand-written, self-contained, Doxygen-documented C23
header (it also compiles clean as C17 and C++17, with `-Wall -Wextra -Werror`):

- `#define APHRODY_ABI_VERSION` — compile-time ABI revision, kept in lock-step
  with the runtime `aphrody_abi_version()` symbol (a Rust unit test parses the
  header and asserts they agree, so they cannot drift).
- `enum AphrodyStatus` — typed, `int32_t`-backed status codes (`OK` = 0,
  `USAGE` = 64, `SOFTWARE` = 70) documenting the values the FFI layer injects.
  The run functions still return a plain `int` because a wrapped command may
  exit with any process code, not only this closed set.
- `APHRODY_NODISCARD` — `[[nodiscard]]` on C23/C++17, else
  `__attribute__((warn_unused_result))` (GCC/Clang) or `_Check_return_` (MSVC),
  applied to every function returning a status or an owned pointer.
- `APHRODY_API` — `__declspec(dllimport/dllexport)` on Windows,
  `__attribute__((visibility("default")))` on GCC/Clang.
- `APHRODY_NONNULL` / `APHRODY_RETURNS_NONNULL` / `APHRODY_MALLOC` — portable
  attribute helpers that degrade to nothing on compilers without them.

These are purely additive diagnostics: the symbol set, signatures, and the ABI
version are unchanged (still `1`).

## Build

```sh
# Dynamic library for Bun / dlopen (target/release/{lib,}aphrody_ffi.{so,dll,dylib})
cargo build --release -p aphrody-ffi

# With aphrody's opt-in command features forwarded into the cdylib:
cargo build --release -p aphrody-ffi --features forensics,index
```

`crate-type = ["cdylib", "rlib"]`: the `cdylib` is what Bun loads; the `rlib`
lets `cargo test`/`nextest` unit-test the marshaling helpers in-process.

## Use from Bun

```ts
import { run, runCaptured, version } from "./bun/index.ts";

console.log(version());

// Capture output in-process and parse it:
const r = runCaptured(["version", "--json"]);
console.log(JSON.parse(r.stdout));

// Or inherit stdio (output goes straight to the terminal):
run(["doctor"]);
```

The loader resolves the cdylib from `target/{release,debug}/` relative to the
binding, or from `APHRODY_FFI_LIB` when set. A runnable smoke is in
`bun/example.ts`:

```sh
cargo build --release -p aphrody-ffi
bun run crates/aphrody-ffi/bun/example.ts
```

## Ownership rules (read before binding from another language)

- A `char *` from `aphrody_run_captured` is **owned by the caller** — release it
  with `aphrody_string_free` exactly once.
- `const char *` results (`aphrody_version`, `aphrody_last_error`) are **owned by
  the library** — never free them. `aphrody_last_error` is thread-local and only
  valid until the next library call on the same thread.
- All strings are UTF-8, NUL-terminated.

## Platforms

Native only (Linux / Windows / macOS): the wrapped surface links tokio +
reqwest + rustls, which do not build on wasm32. On `wasm32-*` the crate compiles
to an empty module so `cargo check --target wasm32-unknown-unknown` stays green.
