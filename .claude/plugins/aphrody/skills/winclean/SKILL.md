---
name: winclean
description: WinClean ecosystem integration for the peer Windows 11 (24H2+) optimization repo — system scanning, debloating, native AOT C# workflows, and a bridge to the WinClean MCP server when available.
version: "1.0.0"
---

# WinClean Integration Skill (Windows-only)

Mode `/goal` permanent : décider seul, agir sans confirmation, ne pas s'arrêter avant complétion.

This skill is **Windows-only** (the WinClean peer repo targets Windows 11 24H2+). `$WINCLEAN` = peer repo root (default `C:\winclean`). No Linux/macOS equivalent — skip on those hosts.

## Capabilities
System scan/profile, debloat recommendations, NativeAOT C# + C++20 dev, peer-side Bun/TS, Python perf tuning.

## Commands (Windows-only)
- `pwsh -File $WINCLEAN\winclean.ps1 scan` — quick system scan
- `pwsh -File $WINCLEAN\winclean.ps1 profile` — full hardware + apps snapshot
- `pwsh -File $WINCLEAN\winclean.ps1 debloat` — apply debloat (auto-creates a System Restore point first, then proceeds)

## Peer-side conventions
- Bun (not npm/yarn/pnpm); oxlint/oxfmt (not eslint/prettier).
- C# = NativeAOT + System.Text.Json source generators; data files in `$WINCLEAN\var\json\`.
- Before destructive changes: create a System Restore point, then proceed (no prompt).
- Protected (PPL) services may fail silently — log and continue.
- Never use WSL on the peer machine.

## MCP integration
When the `winclean` MCP server is connected, call `get_system_profile` directly instead of invoking `winclean.ps1`.

## Cross-repo coordination (A2A)
aphrody coordinates with the peer via `ai.json` (AGNTCY a2a v1.0) and the `$WINCLEAN\.coord\` mailbox (`inbox-from-aphrody.jsonl` / `inbox-from-winclean.jsonl`). Handshake protocol in the repo `CLAUDE.md`.
