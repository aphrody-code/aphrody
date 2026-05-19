<!-- SPDX-License-Identifier: Apache-2.0 -->
# GEMINI.md — DEPRECATED (2026-05-19)

> **This file is deprecated.** All operational directives — language policy,
> workspace layout, validation pipeline, supply-chain rules, A2A integration —
> are now consolidated in **[`CLAUDE.md`](./CLAUDE.md)** and apply equally to
> Claude Code AND Gemini CLI.

## Why deprecated

The previous version of this file (committed before the 2026-05-18 pivot to
**100 % Rust**, cf. memory `feedback_aphrody_rust_only`) tolerated
Bun/TypeScript for scripting and MCP servers. That tolerance was **revoked**
on 2026-05-18.

Since then, `CLAUDE.md` has become the single source of truth for both AI
agents, and shipping two divergent directive files creates contradictions.

## Where to find the canonical directives

| Topic | Read in |
|---|---|
| Role + Mission | `CLAUDE.md` §0 (priorities: Linux #1, Windows #2, WASM #3) |
| Language policy (Rust-only, bxc exception) | `CLAUDE.md` §2 + §0.3 |
| Workspace (54 members, 67 crates/) | `CLAUDE.md` §4 |
| Validation pipeline (`cargo ci-offline`, …) | `CLAUDE.md` §3 |
| Supply-chain (`cargo deny`, `cargo vet`) | `CLAUDE.md` §5 |
| A2A coordination (`ai.json` AGNTCY v1.0) | `CLAUDE.md` §6.1 |
| Active mission queue | `docs/PLAN.md` (Cap 2026-05-19+ : Apex Autonomous Agent) |

## For Gemini CLI users

Treat this file as a pointer. Read `CLAUDE.md` end-to-end before acting in
this repo. The cross-platform priorities, the Rust-only rule, the bxc
exception, the binary install convention (`~/.local/bin`, NOT
`aphrody self install-path` on Windows), and the supply-chain gates apply
identically.
