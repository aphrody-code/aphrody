<!-- SPDX-License-Identifier: Apache-2.0 -->

# Frequently Asked Questions

Pre-answers to the questions most likely to surface in a Show HN thread, an
issue tracker, or a `#general` Slack ping. Cross-links go to authoritative
docs; if an answer here disagrees with the linked source, the linked source
wins.

### Why another CLI? Don't `just` and `taskfile` already do this?

aphrody is not a task runner that wandered into agent territory. It is an
A2A (agent-to-agent) coordination spine that happens to bundle task running
because every dev tool eventually needs one. The defining surface is the
embedded AGNTCY `a2a/v0.4` manifest, the file-based mailbox protocol, and
the parallel YOLO grind loop. See [`COMPARISON.md`](./COMPARISON.md) for a
row-by-row honest take against `just`, `taskfile`, `gh`, `devcontainer`,
and `asdf`.

### What is AGNTCY `a2a/v0.4` and why should I care?

AGNTCY (Agent Network Connectivity) is a consortium spec for agent-to-agent
message exchange and capability discovery. aphrody embeds an `ai.json`
manifest at the repo root and exposes it via `.well-known/ai.json` so peer
agents (Claude Code, MCP clients, custom HTTP consumers) can negotiate
channels without out-of-band config. The manifest is parsed by `serde` at
compile time, so a malformed schema fails the build rather than crashing
at runtime.

### Is the file-based protocol actually durable?

Yes. Each peer writes append-only JSONL into the other peer's `inbox-from-*`
file with a three-deep ack handshake (offer, accept, settle). Crashes on
either side leave the mailbox replayable; no broker, no socket lifecycle,
no race window between durable write and ack. The full design write-up
lives in [`posts/2026-05-ai-json.md`](./posts/2026-05-ai-json.md).

### Does it work on Windows?

Yes. Windows 11 Insider Canary is a first-class build target and runs the
full test suite (`cargo xt-offline`). Linux Ubuntu 26.04 is priority #1 and
gates merges; Windows is priority #2; `wasm32-unknown-unknown` is priority
#3. macOS is best-effort and never blocks a merge.

### Why nightly Rust? What about stability?

Edition 2024 plus the `#[cfg(target_os)]` plumbing we use for cross-platform
gating need nightly features that are not slated to stabilise before 2027.
The toolchain is pinned in `rust-toolchain.toml`, so every contributor and
CI run uses the exact same nightly. We re-pin on a known-good cadence
rather than chasing the latest channel build.

### Why is the repo currently private?

Pre-launch hardening. Flipping to public is on the punch list for the Show
HN milestone (D+15). The decision to launch public is gated on supply-chain
green (`cargo deny check`, `cargo vet`) and a clean SECURITY policy review.

### How do I report a security issue?

Do not open a public GitHub issue. Follow the disclosure procedure in
[`SECURITY.md`](../SECURITY.md) — GitHub private advisories preferred,
encrypted email as fallback.

### Can I use aphrody in production?

Not yet. The current line is `v1.0.0-canary` and is explicitly dev-grade:
APIs may shift, error messages may improve, telemetry hooks are off by
design. The `v1.0.0` stable target is Q3 2026. See
[`ROADMAP.md`](./ROADMAP.md) for the quarterly outlook.

### Does it phone home? Any telemetry?

Zero telemetry by design. aphrody makes no outbound network call unless you
explicitly invoke one (DNS resolution, web scrape, A2A peer dispatch). No
crash uploads, no anonymous usage stats, no opt-out flag because there is
nothing to opt out of.

### How fast is `aphrody doctor`?

Under 50ms on a warm cache for the standard diagnostic profile. For the
broader monorepo scan via `mrx`, the production benchmark is 1.4 seconds
across 19,213 files on a stock Ryzen 7 laptop. The full numbers and the
methodology are in [`BENCHMARKS.md`](../BENCHMARKS.md).

### What is the parallel YOLO grind loop?

A four-subagent autonomous Claude Code orchestration mode. Each tick
dispatches four background agents in parallel, each driving one
`PLAN.md` ⏳ item toward production-ready. The design rationale, the
contention model, and the failure modes we hit getting there are
documented in
[`posts/2026-05-yolo-grind-loop.md`](./posts/2026-05-yolo-grind-loop.md).

### How do I contribute?

Read [`CONTRIBUTING.md`](../CONTRIBUTING.md). Short version: open an issue
before a non-trivial PR, run `cargo ci-offline && cargo deny check` before
pushing, follow Conventional Commits, and never introduce mock or fake
data. Linux is the gating target — if it does not build on Linux, it does
not merge.
