<!-- SPDX-License-Identifier: Apache-2.0 -->
# Show HN launch package — aphrody

> Internal draft. D+15: 2026-06-01, Tue/Wed 13:00-16:00 UTC.
> Run `mkdir -p docs/launch` once before regenerating (this file lives there).
> Nothing posts until the pre-launch checklist is all-green.

## 1. Title candidates

All under 80 chars, factual, no superlatives.

1. `Show HN: Two Claude Codes coordinating over a file-based A2A protocol`
2. `Show HN: ai.json, an AGNTCY a2a/v0.4 manifest with file+HTTP channels`
3. `Show HN: aphrody, a cross-platform Rust CLI with an embedded a2a/v0.4 manifest`
4. `Show HN: Parallel 4-agent grind loop for Rust monorepo work (Apache-2.0)`
5. `Show HN: JSONL inbox + 8788 listener — how two AI agents share one disk`

Strongest: #1. Verifiable in 30s by opening `ai.json` and
`schemas/ai.json/v1.json`; nothing equivalent on crates.io or GitHub.

## 2. Body draft

Two Claude Code sessions, running in parallel on the same machine against two
separate repos, converged within twenty minutes on a file-based agent-to-agent
protocol — and we wrote down what worked.

aphrody is a cross-platform Rust CLI (Linux 26.04, Windows 11,
`wasm32-unknown-unknown`). The interesting part is the coordination layer the
two agents built without ever talking through a network: an `ai.json` manifest
at the repo root (AGNTCY a2a/v0.4 `CollaborationManifest`), a JSONL inbox at
`.coord/inbox-from-<peer>.jsonl`, an optional Bun listener on `:8788` exposing
`/ping`, `/msg`, `/inbox`, `/ai.json`, and a shared envelope with four verbs —
`ping`, `ask`, `fact`, `ack` — plus a `re:` field for threading.

The pattern looks novel: MCP is human-to-agent, gRPC A2A frameworks assume a
network, and most "multi-agent" demos run in one process. This one survives a
network outage because the canonical state is files on disk, and it survives a
process crash because envelopes are appended to JSONL before the listener
acknowledges. HTTP is convenience, not dependency.

Full design write-up — seven channels evaluated, 3-deep `ack` handshake:
https://aphrody-code.github.io/aphrody/posts/2026-05-ai-json/

Repo: https://github.com/aphrody-code/aphrody · Apache-2.0 ·
`cargo install aphrody` after the D+15 publish ladder.

Honest limitation: WASM compiles but `wasm-pack`'s global install is broken
under Bun (vendor shim issue) — workable via `cargo build --target
wasm32-unknown-unknown` until the shim is patched upstream.

## 3. Pre-launch checklist

Every box ticked before the HN submit button is pressed.

- [ ] Repo flipped public on GitHub (`aphrody-code/aphrody`)
- [ ] README ↔ code claims aligned (D+3 audit re-run, no regressions)
- [ ] CI green on `main` — Linux, Windows, `wasm32-unknown-unknown` PASS
- [ ] `cargo ci-offline` and `cargo deny check` green on Linux
- [ ] asciinema cast renders in README (D+7 done, re-verified)
- [ ] Both technical posts live on GitHub Pages
- [ ] crates.io publish ladder done — at minimum `base`, `a2a-pb`, `aphrody`
- [ ] `cargo install aphrody` works from a clean Ubuntu 26.04 VM
- [ ] `ai.json`, `.well-known/ai.json`, `schemas/ai.json/v1.json` valid JSON
- [ ] No AI co-author trailers on any commit reachable from `main`
- [ ] LICENSE = Apache-2.0, SPDX headers on all source files
- [ ] Time slot: Tue/Wed 13:00-16:00 UTC (peak HN US-morning)
- [ ] Reply plan: submitter on standby first 4h, tone honest+terse
- [ ] Backup tag pushed (`launch-d15`)
- [ ] One-paragraph follow-up ready for the "but why Rust" reply

## 4. Top-comment templates

Submitter posts the first comment within 5 minutes to seed the thread.

**Q: Why not just use HTTP and skip the files?**

The file layer is the durable one. The HTTP listener (`listener.ts`, Bun,
~120 LOC) is optional — kill it and coordination still works, every envelope
is appended to `.coord/inbox-from-<peer>.jsonl` first, the listener just
tails. JSONL is the source of truth for replay and audit; HTTP is faster for
live conversation. Network-free fallback was not aesthetic, it was the day
the two agents could not reach each other through any socket on the box.

**Q: How is this different from Anthropic's MCP?**

MCP is human-to-agent (tools, resources, prompts surfaced to one model). This
is agent-to-agent: two Claude Code processes, separate context, separate
repos, coordinating through a shared filesystem and an optional local HTTP
endpoint. Envelopes are symmetric — either side can `ask`, either side `ack`s
— and the manifest is discovered the AGNTCY way at `.well-known/ai.json`.
Complementary, not competing.

**Q: Is this AGNTCY-compliant?**

Yes. `ai.json` at the repo root is a valid AGNTCY a2a/v0.4
`CollaborationManifest`, `schema_version` 1.0.0. Channel extensions
(`file_jsonl`, `http_jsonrpc`, `heartbeat_file`, `git_tag`, `named_pipe`,
`process_inspect`, `markdown_doc`) live in `.well-known/ai.json` and point
back via `canonical_manifest`. Schema: `schemas/ai.json/v1.json`.

## 5. Cross-post plan

Stagger — never simultaneous, never the same wording.

| Day | Time (UTC) | Channel   | Title variant                              |
|-----|-----------|------------|--------------------------------------------|
| 0   | 13:00      | HN         | Title #1                                   |
| 0   | 14:30      | Lobste.rs  | Title #2, tags `rust` `ai`                 |
| 1   | 14:00      | /r/rust    | Title #3, link post #1 not the repo first  |

Contact: noreply@aphrody-code.dev. Handle: `aphrody-code`. No marketing
voice in replies — honest, terse, link to the code.
