<!-- SPDX-License-Identifier: Apache-2.0 -->

# Architecture — aphrody

A 30-second skim for new engineers. For full detail, follow the links in
section 8.

## 1. Bird's-eye view

`aphrody` is a 65-crate Rust workspace producing one primary binary
(`aphrody`, in `crates/cli`), a parallel WebAssembly artefact
(`aphrody-wasm`), and domain-specific crates for design, messaging,
LLM infrastructure, skills, terminal, voice, and monorepo mapping.
All distributed code sits on top of a single `no_std`-compatible
primitives crate (`base`). Cross-cutting subsystems (A2A coordination,
forensics backend, monorepo mapping) live in their own layers so the
CLI surface stays thin.

```
            +-----------------------------------------+
            |  CLI (crates/cli)  - aphrody binary     |
            +--------+-----------+----------+---------+
                     |           |          |
                     v           v          v
                +---------+  +--------+  +---------+
                | backend |  |  a2a-* |  |  mrx-*  |
                +----+----+  +----+---+  +----+----+
                     |            |           |
                     +------+-----+           |
                            v                 |
                       +--------+              |
                       |  base  | <-----------+
                       +--------+
```

## 2. Layered crate map

- **L0 leaf (`no_std`-compatible)**: `base` — IDs, error types, time
  primitives, allocator-free helpers shared by every layer.
- **L1 transport**: `a2a-pb` — Protocol Buffers + tonic codegen for the
  AGNTCY A2A v0.4 protocol. Authoritative `src/gen/` committed; regen
  gated on `A2A_PB_REGEN=1`.
- **L2 high-level A2A**: `a2a` (package `a2a-lf`), `a2a-client`,
  `a2a-server`, `a2a-grpc` — typed envelopes, channel extensions,
  client/server traits, and the gRPC transport binding. Depend on
  `a2a-pb` + `base`.
- **L3 forensics + monorepo**: `backend` (process / DNS / network
  introspection, cross-platform), `mrx` (unified Monorepo Real-time
  X-platform mapper, ex `mrx-{core,detect,audit,watch,cli}`).
- **L3b domain infra**: `aphrody-llm-infra` (cost + rateguard + retry +
  cache), `aphrody-skills` (runtime + hooks + permissions),
  `aphrody-messaging` (outbound connectors + bidirectional channels),
  `aphrody-design` (sidecar + daemon), `aphrody-voice` (TTS + STT).
- **L4 CLI surface**: `aphrody` (the binary; pulls in `base`, `backend`,
  and the domain crates), `aphrody-translate` (FR translation + AI-isms
  scrub tool).
- **L5 host integrations**: `google_mcp` (MCP server bridging `backend` to
  Claude tooling), `gui` (wry + tao desktop UI, excluded from the CLI
  distributable; depends on `backend`).
- **L-wasm**: `aphrody-wasm` — parallel to L4, depends only on `base`,
  built for `wasm32-unknown-unknown` via `wasm-bindgen` for browser
  embedding.

The 65 workspace members are declared in the root `Cargo.toml`. Out-of-
workspace directories (`crates/coreutils`, `crates/util-linux`,
`crates/a2a-slimrpc`, `vendor/`) are explicitly excluded.

## 3. Data flow — A2A coordination

A2A is the file-and-HTTP protocol used between cooperating Claude
instances (this repo `C:\src\aphrody\` and the peer at `C:\winclean\`).
Each side publishes an `ai.json` manifest, drops envelopes in the peer's
inbox JSONL, and exposes an HTTP listener for low-latency exchange. The
envelope schema is encoded by `a2a-pb`, surfaced by `a2a`, and consumed
by `a2a-client` / `a2a-server`.

```
aphrody Claude                 winclean Claude
     |                              |
     |  POST :8788/msg              |
     |----------------------------->|
     |   { type:"ask", id:X }       |
     |                              |
     |             reads inbox-from-aphrody.jsonl
     |                              |
     |       reply via :8788/msg    |
     |<-----------------------------|
     |   { type:"ack", re:X }       |
     |                              |
   reads inbox-from-winclean.jsonl  |
```

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
single bump updates the whole tree). Cross-compilation goes through
`cargo-zigbuild` for Linux/Windows targets and `wasm-pack` for the WASM
artefact. CI runs `cargo ci-offline` (clippy with `-D warnings`),
`cargo xt-offline` (nextest), `cargo deny check`, and `cargo vet` on the
Linux target first. Distribution surfaces planned: `.deb`, snap, AUR,
scoop, winget, homebrew tap, and `npm` for `aphrody-wasm`.

## 6. Conventions

Conventional Commits (`feat:`, `fix:`, `refactor:`, `build:`, ...).
Apache-2.0 SPDX header on every file. No emoji in source. CLI user-facing
strings are French (enforced by `aphrody-translate` which also scrubs
AI-isms). No AI co-author trailers in commits. Linux Ubuntu 26.04 is the
mandatory build target; Windows MSVC and `wasm32-unknown-unknown` are
co-required for merge; macOS is best-effort.

## 7. Where things are NOT

To shortcut the usual "where is X?" search:

- `crates/google_os` — archived 2026-05-17 to
  `C:\google-os-archive\` when the project pivoted from a Windows-NT
  kernel emulator to a cross-platform CLI.
- `crates/bun_ffi` — archived; the V8 / JS FFI polluted the Rust
  workspace for zero CLI benefit.
- `crates/n2b` — reintegrated upstream as
  `https://github.com/aphrody-code/n2b` (branch `aphrody`); consumed via
  `workspace.dependencies`.
- `crates/google_kv`, `crates/python_ffi` — archived (no consumer / orphan
  dependencies on `vendor/bun`).
- `vendor/bun` — runtime Bun fork, kept as a path source for scripting
  but excluded from `workspace.members`.
- `crates/a2a-slimrpc` — kept out of the workspace until
  `agntcy-slim-mls` is fixed upstream.

## 8. Related

- `docs/SOURCE_OF_TRUTH.md` — consolidated platform / deliverable view.
- `docs/PLAN.md`, `docs/DESIGN.md`, `docs/ROADMAP.md` — planning surface.
- `docs/adr/0001-cross-platform-rust.md` — pivot rationale.
- `docs/adr/0002-a2a-file-based.md` — A2A protocol choice.
- `docs/adr/0003-yolo-parallel-grind.md` — grind-loop design.
- `docs/posts/2026-05-ai-json.md` — dev journal on the A2A handshake.
- `docs/posts/2026-05-yolo-grind-loop.md` — dev journal on the grind loop.
- `docs/WINCLEAN-AUDIT.md` — cross-repo audit of the peer `C:\winclean\`.
