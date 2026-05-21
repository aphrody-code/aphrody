<!-- SPDX-License-Identifier: Apache-2.0 -->

# Architecture — aphrody

A 30-second skim for new engineers. For full detail, follow the links in
section 8.

## 1. Bird's-eye view

`aphrody` is a Rust workspace of **57 active members** (out of **71 crates
present on disk** — 14 heavy UI/web crates are excluded from the default
build, see section 7) producing one primary binary (`aphrody`, in
`crates/cli`), a parallel WebAssembly artefact (`aphrody-wasm`), and
domain-specific crates for design, messaging, LLM infrastructure, skills,
terminal, voice, memory, and monorepo mapping. All distributed code sits on
top of a single `no_std`-compatible primitives crate (`base`). Cross-cutting
subsystems (A2A coordination, forensics backend, monorepo mapping) live in
their own layers so the CLI surface stays thin.

```
            +-----------------------------------------+
            |  CLI (crates/cli)  - aphrody binary     |
            +--------+-----------+----------+---------+
                     |           |          |
                     v           v          v
                +---------+  +--------+  +---------+
                | backend |  |  a2a-* |  |   mrx   |
                +----+----+  +----+---+  +----+----+
                     |            |           |
                     +------+-----+           |
                            v                 |
                       +--------+             |
                       |  base  | <-----------+
                       +--------+
```

## 2. Layered crate map

- **L0 leaf (`no_std`-compatible)**: `base` — IDs, error types, time
  primitives, allocator-free helpers shared by every layer.
- **L1 transport**: `a2a-pb` — Protocol Buffers + tonic codegen for the
  AGNTCY A2A v0.4 protocol. Authoritative `src/gen/` committed; regen
  gated on `A2A_PB_REGEN=1`.
- **L2 high-level A2A**: `a2a` (package `a2a-lf`), `a2a-client` (package
  `a2a-client-lf`), `a2a-server` (package `a2a-server-lf`), `a2a-grpc`,
  `a2a-ui` — typed envelopes, client/server traits, the gRPC transport
  binding, and a WASM channel viewer. Depend on `a2a-pb` + `base`.
- **L3 forensics + monorepo**: `backend` (process / DNS / network
  introspection, cross-platform), `mrx` (unified Monorepo Real-time
  X-platform mapper — single crate, merged from the former
  `mrx-{core,detect,audit,watch,cli}`).
- **L3b domain infra**: `aphrody-llm-infra` (unified LLM runtime: cost +
  rateguard + retry + cache, merged from the former
  `aphrody-{cost,rateguard,retry,cache}`), `aphrody-skills` (runtime +
  hooks + permissions, merged from the former
  `aphrody-{skills-runtime,hooks,permissions}`), `aphrody-messaging`
  (outbound connectors + bidirectional channels, merged with the former
  `aphrody-channels`), `aphrody-design` (sidecar + daemon, merged from the
  former `aphrody-design-{sidecar,daemon}`), `aphrody-design-agents`
  (CLI agent spawner), `aphrody-voice` (TTS + STT, merged with the former
  `aphrody-voice-stt`), `aphrody-memory`, `aphrody-gateway`, `aphrody-mcp`,
  `aphrody-router`, `aphrody-providers`, `aphrody-prompts`,
  `aphrody-context`, `aphrody-session`, `aphrody-tools`, `aphrody-events`,
  `aphrody-secrets`, `aphrody-settings`, `aphrody-telemetry`,
  `aphrody-task-runner`, `aphrody-search`, `aphrody-cron`,
  `aphrody-marketplace`, `aphrody-skills-forge`, `aphrody-re`,
  `notebooklm`, `gemini-runtime`.
- **L3c terminal stack**: `aphrody-terminal-{vt,wasm,backend,llm,browser,
  json-out,markdown,config}` — LLM-first terminal (VT parser, WASM
  renderer, pty backend, LLM event bus, browser bridge). Plus `aphrody-tui`
  (pure-Rust ratatui-style DSL).
- **L4 CLI surface**: `aphrody` (the binary, crate dir `crates/cli`; pulls
  in `base`, `backend`, and the domain crates), `aphrody-chat` (turn-loop
  REPL), `aphrody-sdk` (public embedding SDK), `aphrody-translate` (FR
  translation + AI-isms scrub tool), `aphrody-summary` (regenerates
  `docs/SUMMARY.md` + `docs/llms.txt`).
- **L5 host integrations**: `google_mcp` (MCP server bridging `backend` to
  Claude tooling), `ievr-tools` (IEVR binary-inventory analysis),
  `m3-tokens` / `aphrody-icons` (Material Design 3 baseline tokens + icon
  font assets), `aphrody-react-reconciler` (host-side React reconciler
  primitives), `aphrody-mcp-smoke` (end-to-end MCP smoke harness).
- **L-wasm**: `aphrody-wasm`, `aphrody-terminal-wasm`, `a2a-ui` — built for
  `wasm32-unknown-unknown` via `wasm-bindgen` for browser embedding.

The 57 workspace members are declared in the root `Cargo.toml`. Heavy UI/web
crate clusters and a handful of orphans are present on disk but listed in the
`exclude` block (see section 7); `vendor/` was removed in 2026.

## 3. Data flow — A2A coordination

A2A is the protocol used between cooperating Claude instances (this repo
`C:\src\aphrody\` and the peer at `C:\winclean\`). The envelope schema is
encoded by `a2a-pb`, surfaced by `a2a`, and consumed by `a2a-client` /
`a2a-server` over gRPC. The legacy file-based mailbox (`ai.json` root manifest
plus the `ai/` heartbeat/outbox/inbox tree) was **removed in 2026** in favour
of the typed gRPC transport; only the winclean compatibility mirror under
`C:\winclean\.coord\` remains for cross-repo coordination.

## 4. Data flow — YOLO grind loop

The `/aphrody-yolo-grind` skill dispatches 4 background agents per tick
against distinct file families. Each tick is autonomous: the orchestrator
reads `PLAN.md`, fans out work, waits for completions, validates with
`cargo check`, commits with the honest-delivery format, and flips
`PLAN.md` markers from waiting to done.

```
t=0   orchestrator reads PLAN.md (waiting items)
t=1   dispatches 4 agents (bg, parallel)
t=2   ...4 agents work on distinct file families...
t=60  all 4 notify completion
t=61  orchestrator: cargo check -> git commit (honest-delivery format)
t=62  flip PLAN.md waiting -> done
t=63  next tick
```

## 5. Build and dist

One Cargo workspace, ~80 dependencies centralised in
`[workspace.dependencies]` so every member uses `{ workspace = true }` (a
single bump updates the whole tree). `.cargo/config.toml` pins `jobs = 8` and
a 30 GB `sccache` cache (`SCCACHE_CACHE_SIZE = "30G"`). Cross-compilation goes
through `cargo-zigbuild` for Linux/Windows targets and `wasm-pack` for the
WASM artefact. CI runs `cargo ci-offline` (clippy with `-D warnings`),
`cargo xt-offline` (nextest), `cargo deny check`, and `cargo vet` on the
Linux target first.

Deployment of the built binary is handled by `scripts/deploy.ps1`
(Windows) and `scripts/deploy.sh` (Linux/macOS), which build
`-p aphrody --release` and copy the binary into `~/.local/bin/`. The unified
MCP server `aphrody-mcp` is rebuilt manually:
`cargo build --release -p aphrody-mcp` followed by a copy into `~/.local/bin/`.
Distribution surfaces planned: `.deb`, snap, AUR, scoop, winget, homebrew tap,
and `npm` for `aphrody-wasm`.

## 6. Conventions

Conventional Commits (`feat:`, `fix:`, `refactor:`, `build:`, ...).
Apache-2.0 SPDX header on every file. No emoji in source. CLI user-facing
strings are French (enforced by `aphrody-translate` which also scrubs
AI-isms). Linux Ubuntu 26.04 is the mandatory build target; Windows MSVC and
`wasm32-unknown-unknown` are co-required for merge; macOS is best-effort.

## 7. Excluded from the default workspace (present on disk)

These crates exist under `crates/` but are listed in the `Cargo.toml`
`exclude` block. They are **not deleted** — they are kept out of the default
build because they pull heavy UI/web toolchains (wgpu/vello/winit/wasmtime,
Next.js/SWC/lightningcss/napi) that dominated `cargo nextest run --workspace`
on a 4c/8t/16 GB machine, and the `aphrody` binary does not depend on any of
them. Rebuild them by re-listing them in `members` temporarily or with an
ad-hoc workspace.

- `gui` — aggregates the `mui-rs*` and `tuono*` clusters (wry + tao desktop).
- `agui-bridge` — consumes `mui-rs-components`.
- `mui-rs`, `mui-rs-core`, `mui-rs-components`, `mui-rs-macros`,
  `mui-rs-motion`, `mui-rs-renderer` — native MD3 renderer (wgpu, vello,
  winit, wasmtime, parley, fontique).
- `tuono`, `tuono_internal`, `tuono_lib`, `tuono_lib_macros` — Next.js SSR
  integration (swc_core, lightningcss, mdxjs, napi).
- `aphrody-x-client` — self-rooted workspace, pending `agent-twitter-client`
  stabilisation.
- `a2a-slimrpc` — blocked on the upstream `agntcy-slim-mls` nightly
  lifetime / async-trait incompatibility.

`crates/coreutils` and `crates/util-linux` are still referenced in the
`exclude` block for historical reasons but no longer exist on disk.

## 8. Where things are NOT (history)

To shortcut the usual "where is X?" search, the following were **removed**:

- `crates/google_os` — archived 2026-05-17 to `C:\google-os-archive\` when
  the project pivoted from a Windows-NT kernel emulator to a cross-platform
  CLI.
- `crates/bun_ffi`, `crates/google_kv`, `crates/python_ffi` — archived
  (Bun/JS FFI and orphans with no in-tree consumer).
- `crates/aphrody-xtask` — removed 2026-05-21; its dev tasks now live in
  `scripts/deploy.{ps1,sh}` or are run directly with `cargo`.
- The 11 `n2b-*` crates and `bxc-engine` — removed 2026-05-21. The
  `aphrody n2b` subcommand is now a thin façade that spawns an optional
  external binary (resolved via `APHRODY_N2B_BIN` then `PATH`); there is no
  longer any compile-time `n2b` / `bxc` dependency.
- 18 duplicate crates merged into their canonical homes:
  `aphrody-{cache,cost,rateguard,retry}` → `aphrody-llm-infra`;
  `aphrody-channels` → `aphrody-messaging`;
  `aphrody-{hooks,permissions,skills-runtime}` → `aphrody-skills`;
  `aphrody-design-{daemon,sidecar}` → `aphrody-design`;
  `aphrody-voice-stt` → `aphrody-voice`;
  `mrx-{core,detect,audit,watch,cli}` → `mrx`;
  plus the orphan crates `aphrody-shell` and `aphrody-sandbox`.

## 9. Related

- `docs/SOURCE_OF_TRUTH.md` — consolidated platform / deliverable view.
- `docs/PLAN.md`, `docs/ROADMAP.md` — planning surface.
- `docs/cargo/WORKSPACE.md` — fine-grained workspace description.
- `docs/cargo/CRATES.md` — per-crate inventory.
