<!-- SPDX-License-Identifier: Apache-2.0 -->

# aphrody vs alternatives

A 30-second comparison for engineers evaluating whether aphrody belongs in
their toolbox alongside (or instead of) the usual suspects.

## 1. What aphrody does

aphrody is a cross-platform Rust CLI (Linux, Windows, WASM) with an embedded
AGNTCY a2a/v0.4 manifest for cross-agent coordination. It ships a parallel
YOLO grind loop (4 subagents per tick driving items to production-ready),
Chromium / DNS / scrape forensics, MRX (Monorepo Real-time X-platform mapper),
and a single native Rust MCP server (`aphrody-mcp`). The same binary
coordinates with peer agents over the typed gRPC A2A transport (crates
`a2a-*`) and exposes itself to Claude Code, MCP clients, and arbitrary HTTP
consumers — without
any container, daemon, or vendor-lock-in. Linux 26.04 is the primary build
target; Windows 11 Canary and `wasm32-unknown-unknown` are first-class.

## 2. Quick comparison table

| Capability | aphrody | just | taskfile | gh | devcontainer | asdf |
|---|---|---|---|---|---|---|
| Cross-platform binary | ✅ Linux+Win+WASM | ✅ | ✅ | ✅ | ❌ (container only) | ✅ |
| Embedded agent manifest (AGNTCY a2a/v0.4) | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Cross-agent coordination (gRPC A2A) | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Subagent parallel orchestration | ✅ (4-agent grind) | ❌ | ❌ | ❌ | ❌ | ❌ |
| Web scraping / DNS forensics | ✅ | ❌ | ❌ | partial | ❌ | ❌ |
| Monorepo real-time scan | ✅ (mrx) | ❌ | ❌ | ❌ | ❌ | ❌ |
| Chromium profile sync | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |
| Built-in `doctor` diagnostic | ✅ | ❌ | ❌ | ❌ | ❌ | partial |
| Shell completions | ✅ (bash/zsh/fish/pwsh) | ✅ | ✅ | ✅ | ✅ | ✅ |
| Single static binary distribution | ✅ | ✅ | ✅ | ✅ | ❌ | ✅ |
| WASM target (browser-runnable) | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |

Honest take, row by row:

- **just** — Use it if you only need a task runner with a readable `justfile`.
  aphrody embeds task running via `aphrody auto` but also ships A2A coord,
  forensics, and WASM, which `just` intentionally does not.
- **taskfile** — Excellent declarative YAML-driven runner with great
  templating. Pick it if you want strict YAML semantics; aphrody trades that
  for an agent-aware execution model.
- **gh** — The official GitHub CLI is unbeatable for PR/issue/release flows.
  aphrody does not replace it; it complements it (combine `gh pr create`
  with `aphrody` for cross-agent coordination on the same branch).
- **devcontainer** — Solves a different problem: a reproducible containerised
  IDE. aphrody runs natively on the host, no Docker daemon required.
- **asdf** — The right tool for pinning runtime versions across languages.
  aphrody does not version-manage external runtimes and never will.

## 3. When to NOT use aphrody

Be honest with your stack before adopting another tool:

- **Pure task runner, no agent or cross-platform needs** — `just` or
  `taskfile` are leaner, faster to learn, and zero ceremony.
- **GitHub-only workflow** — `gh` is the official tool, integrates with
  Actions, Projects, and Codespaces, and covers features aphrody intentionally
  avoids (issues UI, PR review surface, Copilot bridge).
- **Reproducible dev environment** — `devcontainer` (or Nix, or
  `docker-compose`) is the correct solution. aphrody assumes a working host
  toolchain.
- **Multi-language runtime pinning** — `asdf`, `mise`, `volta`, or
  `rustup`/`pyenv`/`nvm` stacks own this space. aphrody calls out to whatever
  is on `PATH`.

If your project hits none of the differentiators in §2 (agent coord, MRX,
forensics, WASM), aphrody is overkill — pick the focused tool.

## 4. Migration shortcuts

- **From `just`** — drop your `justfile`, move recipes into `aphrody auto`
  hooks, and keep the same shell-command muscle memory. aphrody does not
  parse `justfile` natively; recipes are a one-time copy-paste.
- **From `gh`** — do not migrate. aphrody and `gh` solve different problems.
  Keep `gh` for PRs, releases, and gh-actions triggers; let aphrody handle
  cross-agent inbox/outbox and parallel grind orchestration on the same repo.
- **From `devcontainer`** — not a migration; different problem space. If your
  team needs both reproducibility and agent coord, run aphrody inside the
  devcontainer.
- **From `asdf`** — keep asdf for runtime pinning. aphrody's `doctor`
  command reports what it finds on `PATH`; it does not install or shim
  external toolchains.

## 5. Footer

See also:

- [`README.md`](../README.md) — install, quickstart, architecture overview.
- [`PROTOCOL.md`](PROTOCOL.md) — A2A protocol notes (now gRPC-based; the
  file-based mailbox was removed in 2026).
