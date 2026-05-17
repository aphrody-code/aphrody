<!-- SPDX-License-Identifier: Apache-2.0 -->

# ADR-0002: File-based A2A coordination with HTTP overlay

## Status

Accepted

Date: 2026-05-17

Author: aphrody-code

## Context

Two Claude Code instances coordinate cross-repo: one in `C:\src\aphrody`,
one in `C:\winclean`. They run in different conversations, different
CWDs, often non-overlapping wall-clock windows, on the same host. They
need to:

- Exchange durable messages that survive either agent crashing or being
  paused mid-conversation.
- Detect peer liveness without polling a database.
- Hand off work items (facts, plans, requests, acks) with an auditable
  cross-repo history.
- Operate offline — no public internet, no cloud broker, no SaaS.

The protocol is documented in `docs/posts/2026-05-ai-json.md`; the
manifest sits at the repo root as `ai.json` (AGNTCY `a2a/v0.4`), with
the channel-extension schema at `schemas/ai.json/v1.json`.

## Decision

We adopt a **7-channel hybrid protocol** with one canonical record and
six overlay channels:

1. **JSONL append-only inbox** (`C:\winclean\.coord\inbox-from-aphrody.jsonl`
   + `inbox-from-winclean.jsonl`) — the **durable canonical record**.
   Append-only means concurrent writes never lose data; replay is `cat`.
2. **HTTP listener on `:8788`** (`bun run .coord/listener.ts` exposing
   `/ping`, `/msg`, `/inbox`, `/ai.json`) — low-latency overlay, falls
   back gracefully when down.
3. **Heartbeat file** (`heartbeat-{aphrody,winclean}.txt`, ISO-8601) —
   cheap proof-of-life without parsing inbox.
4. **Git tags** (`aphrody-*` in winclean repo) — out-of-band signals
   that survive repo clones.
5. **Named pipe** — Windows-native fallback when HTTP is busy.
6. **Process inspection** (`ps -ef`) — detect peer activity when other
   channels are silent.
7. **Markdown doc thread** — human-readable reconciliation for audit.

Reconciliation uses a **3-deep ack handshake**: each envelope carries
`re:` chains so the receiver can prove it observed N previous messages.

## Alternatives Considered

- **Pure HTTP / gRPC**: low latency but no durability without an attached
  DB; one process crash drops the conversation; ports collide on a dev
  workstation. Adds infra without solving offline.
- **Redis pub/sub**: solves durability via AOF but adds an external
  daemon and breaks the "single-binary friendly" posture.
- **Shared filesystem with file locking**: Windows locking semantics are
  hostile (mandatory locks, sharing-mode flags); cooperation across
  Bun/Rust/PowerShell writers is unreliable. Append-only JSONL sidesteps
  the entire problem.
- **MCP (Model Context Protocol)**: designed for human↔agent and
  agent↔tool, not symmetric agent↔agent across separate conversations.

## Consequences

Positive:

- Fully offline-capable, zero external infra dependency.
- Append-only JSONL is trivially replayable and auditable.
- HTTP overlay gives sub-second latency when both agents are live.
- Multiple redundant channels mean no single point of failure.

Negative:

- Eventually consistent; the 3-deep ack handshake adds latency for
  causally-ordered work.
- Seven channels is more surface than a single bus — operators must
  know which is canonical (the JSONL inbox) and which are overlays.
- Cross-repo writes must respect peer-in-flight uncommitted edits
  (catastrophe risk on `Cargo.lock`, CLAUDE.md §6.1).

## References

- Protocol journal: `docs/posts/2026-05-ai-json.md`.
- Schema: `schemas/ai.json/v1.json`.
- Manifest: `C:\src\aphrody\ai.json` and `C:\winclean\ai.json`.
- Skill: `.claude/skills/a2a-duel-loop/scripts/duel-cycle.ts`.
- Commit `8bcbbbd97` — `/a2a-duel-loop` skill.
- Commit `7343772de` — D+14 technical post on ai.json A2A.
- CLAUDE.md §6.1 (A2A coordination cross-Claude).
