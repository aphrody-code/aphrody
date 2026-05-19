---
name: monorepo
description: Cross-AI monorepo architect skill for the aphrody pentalingual workspace (Bun packages/, Cargo crates/, MSVC native/). Provides per-AI directive files (claude.md, gemini.md, opencode.md) + task.json rules consumed by /monorepo slash commands across multiple AI runtimes.
version: "1.0.0"
---

# Monorepo Architect Skill

Cross-AI directive set for orchestrating the aphrody pentalingual monorepo
(TS/JS via Bun, Rust via Cargo, C++ via MSVC, Zig in `packages/omnistack`,
Python build tooling only).

## Layout

This skill ships **per-AI directive files** instead of a single playbook,
because aphrody is consumed by multiple AI runtimes (Claude Code, Gemini CLI,
OpenCode) that each have slightly different interaction patterns:

- `claude.md` — directives for Claude Code (Anthropic).
- `gemini.md` — directives for Gemini CLI (Google).
- `opencode.md` — directives for OpenCode (peer agent).
- `task.json` — shared rule set (workspaces, ts/rust/cpp layer policies,
  Turborepo orchestration).

## When invoked

Triggered by the user typing `/monorepo` (or variants like
`/monorepo optimize`, `/monorepo audit`). The slash command router picks the
runtime-appropriate directive file based on the active AI and applies the
shared `task.json` rules.

## Authoritative rules (from `task.json`)

| Layer | Rule |
|---|---|
| TS/JS | Bun Workspaces, strictly enforce `bun.lock` |
| Rust | Cargo Workspaces, share deps via `workspace.dependencies` |
| C++ | CMake with `add_subdirectory`, compile natively |
| Orchestration | Turborepo (`turbo.json`) for cross-language DAG + task caching |

## Cross-reference

The full conventions live in `CLAUDE.md` §4 (Workspace) + §2 (Language
policy). The `task.json` here is a runtime-readable mirror for AI agents
that can't parse markdown directives.
