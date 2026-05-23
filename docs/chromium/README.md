<!-- SPDX-License-Identifier: Apache-2.0 -->
# docs/chromium — Chromium & V8 build reference (Windows-first)

> Distilled, attributed reference for building **V8** (and the wider Chromium
> toolchain) from source on **Windows 11**, as used by aphrody to produce a
> native `v8_monolith` static library. Facts are paraphrased from the official
> Google docs and cross-checked against this machine's real checkout in
> `C:\src\v8` (depot_tools in `C:\src\depot_tools`). Article bodies are **not**
> reproduced verbatim (copyright); each section links its canonical source.

## Why this exists

aphrody needs a Windows-native V8 to unblock JS-engine work that Lightpanda /
`zig-v8-fork` cannot deliver on Windows (no `*-windows` V8 artifact upstream).
The path is therefore **V8-from-source via depot_tools + GN + ninja**, built
against **Visual Studio 2026 Insiders** (MSVC 19.5x).

## Map

| Doc | Scope |
|-----|-------|
| [`get-the-code.md`](get-the-code.md) | depot_tools, `fetch v8`, gclient sync, fast-fetch accelerators |
| [`windows-build.md`](windows-build.md) | Windows prerequisites: VS 2026, Windows SDK, env vars |
| [`v8-build.md`](v8-build.md) | `gm.py`, `gn gen`, GN args, ninja targets (`v8_monolith`, `d8`) |
| [`v8-embed.md`](v8-embed.md) | Embedder's guide: Isolate/Context/Handle, linking the monolith |
| [`rust-bindings.md`](rust-bindings.md) | Official Rust↔V8 binding (`rusty_v8` / crate `v8` 149) — prebuilt MSVC vs from-source |
| [`aphrody-v8-state.md`](aphrody-v8-state.md) | This machine's exact build state, env, and gotchas |

## Canonical sources

- Chromium Windows build: <https://chromium.googlesource.com/chromium/src/+/main/docs/windows_build_instructions.md>
- Chromium get-the-code: <https://chromium.googlesource.com/chromium/src/+/main/docs/get_the_code.md>
- V8 source checkout: <https://v8.dev/docs/source-code>
- V8 build: <https://v8.dev/docs/build> · GN build: <https://v8.dev/docs/build-gn>
- V8 embedder's guide: <https://v8.dev/docs/embed>
- depot_tools tutorial: <https://commondatastorage.googleapis.com/chrome-infra-docs/flat/depot_tools/docs/html/depot_tools_tutorial.html>

> Fetched 2026-05-22. depot_tools has **no official GitHub repo** — clone only
> from `chromium.googlesource.com/chromium/tools/depot_tools.git`.
