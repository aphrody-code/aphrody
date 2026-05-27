<!-- SPDX-License-Identifier: Apache-2.0 -->
# @aphrody-code/console

A live, Material 3 web UI that drives the **entire aphrody CLI** in-process —
no subprocess, no stub. Every byte of output comes from the real command.

## How it works

```
browser (M3 frontend)  --HTTP-->  Bun server  --bun:ffi-->  aphrody-ffi cdylib
   src/app.ts                      src/server.ts             @aphrody-code/native
```

`bun:ffi` runs in Bun, not the browser, so the native library is loaded by a
small Bun server (`src/server.ts`) that exposes a local JSON API; the browser
frontend (`src/index.html` + `src/app.ts`) calls it. This is the chosen UI
architecture for aphrody: a Bun server backed by `@aphrody-code/native`, with an
M3 frontend on top.

## Run

```sh
# 1. build the native library once, in the Rust repo (C:\src\aphrody)
cargo build --release -p aphrody-ffi

# 2. start the console (from the aphrody-ts root)
bun run --filter @aphrody-code/console dev      # or: cd apps/console && bun run dev
```

Open the printed URL (default `http://localhost:4317`). Type any aphrody
command (e.g. `doctor --json`) or use the preset chips; the exit code, stdout
and stderr come back from the real engine. `PORT` overrides the port.

## API

| Route | Result |
|---|---|
| `GET /api/version` | `{ version }` of the loaded native library. |
| `POST /api/run` | body `{ args: string[] }` (no `argv[0]`) -> `{ code, stdout, stderr }`. |

## Test

```sh
bun test apps/console     # exercises the real native-backed handlers; skips if the cdylib is not built
```

## Scope

This is the first real vertical slice: a command console exposing the full CLI
surface. It is intentionally framework-light (M3 design tokens + semantic
elements). Richer panels (a `doctor` dashboard, streaming output, the Material
Web component set from `packages/material-web`) build on the same server + API.
