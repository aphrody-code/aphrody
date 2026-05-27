<!-- SPDX-License-Identifier: Apache-2.0 -->

# @aphrody-code/native

In-process bridge from the aphrody-ts (Bun) workspace to the **aphrody native
library** — the Rust crate `aphrody-ffi` in the sibling repo `C:\src\aphrody`.

It loads the `aphrody-ffi` cdylib via Bun `bun:ffi` and drives the **entire**
aphrody CLI **in-process** (no subprocess spawn, no HTTP). This is the chosen
integration between the two repos: aphrody (Rust) is the engine and the source
of truth for the C ABI; aphrody-ts consumes it here.

## Prerequisites

Build the cdylib once in the Rust repo:

```sh
# in C:\src\aphrody
cargo build --release -p aphrody-ffi      # or: cargo build -p aphrody-ffi (debug)
```

The bridge discovers the library automatically (sibling `../aphrody/target/<triple>/{release,debug}/`).
Override with `APHRODY_FFI_LIB=/path/to/aphrody_ffi.dll` or `APHRODY_REPO=/path/to/aphrody`.

## Usage

```ts
import { run, runCaptured, version, assertCompatible } from "@aphrody-code/native";

assertCompatible(); // throws if the loaded ABI != this binding's
console.log(version());

// Capture output in-process and parse it:
const r = runCaptured(["version", "--json"]);
console.log(JSON.parse(r.stdout)); // { version, commit, target, ... }

// Or inherit stdio (output goes straight to the terminal):
run(["doctor"]);
```

Arguments exclude the program name — a synthetic `argv[0]` is prepended by the
native layer.

## API

| Export                                          | Purpose                                              |
| ----------------------------------------------- | ---------------------------------------------------- |
| `version(): string`                             | aphrody version.                                     |
| `abiVersion(): number` / `EXPECTED_ABI_VERSION` | runtime / compile-time ABI.                          |
| `assertCompatible(): void`                      | throw on ABI mismatch (drift guard).                 |
| `run(args): number`                             | run any command, inherited stdio, returns exit code. |
| `runCaptured(args): { code, stdout, stderr }`   | run + capture in-process.                            |
| `lastError(): string \| null`                   | last native error on this thread.                    |
| `close(): void`                                 | release the library handle.                          |

## Drift safety

The C ABI is versioned. This binding mirrors `APHRODY_ABI_VERSION` from
`crates/aphrody-ffi/include/aphrody.h`; `assertCompatible()` rejects a library
whose runtime version differs, so an ABI change in the Rust repo surfaces here
as a clear error rather than a silent corruption. The canonical, drift-tested
binding lives in the Rust repo (`crates/aphrody-ffi/bun/index.ts`); this package
is the aphrody-ts-idiomatic, self-contained copy that resolves the sibling
repo's cdylib.

## Test

```sh
bun test apps/native      # skips gracefully if the cdylib is not built
```
