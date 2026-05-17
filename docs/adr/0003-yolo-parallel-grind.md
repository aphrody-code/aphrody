<!-- SPDX-License-Identifier: Apache-2.0 -->

# ADR-0003: 4-agent parallel YOLO grind per tick

## Status

Accepted

Date: 2026-05-17

Author: aphrody-code

## Context

The `aphrody` project drives autonomous work to close `PLAN.md` items
(⏳ → ✅) across eight crates. Single-agent loops have a recurring
failure mode:

- Context bloats as the agent reads more files per tick, hitting
  compaction and losing thread continuity.
- Wall-clock latency dominates throughput because a single agent
  serialises across reads, builds, and verification.
- Stalls happen mid-investigation; recovery is hand-driven.

The repository is a Rust workspace of 17 members where most items fall
into disjoint file-path families (CLI core, WASM bindings, A2A protocol,
build hygiene). That makes parallelism cheap to reason about.

## Decision

Each YOLO tick dispatches **exactly 4 sub-agents in a single tool-call
block**. Each agent:

1. Owns one ⏳ item from `PLAN.md`.
2. Operates on a **distinct file-path family** (e.g. `crates/cli/**`,
   `crates/a2a-pb/**`, `docs/**`, `.github/workflows/**`) — the
   orchestrator chooses at tick boundary to avoid edit collisions.
3. Runs in background (`run_in_background: true`, per memory
   `feedback_always_background`).
4. Reports a completion envelope the orchestrator batches into a single
   verify + commit step once all four notify.

The orchestrator runs `cargo ci-offline` + `cargo deny check` after the
batch lands and either commits the four scoped changes as separate
commits or rolls back the whole tick.

## Alternatives Considered

- **Serial single-agent loop**: 3–4× slower wall-clock for the same set
  of items; context-bloat is unmitigated; observed during early
  `/aphrody-perfect-grind` iterations before the 4-way split.
- **8+ parallel agents**: tested informally — `cargo` lock contention on
  `Cargo.lock` and `target/` rises sharply, diminishing returns after
  agent 5, race risk on shared files (`PLAN.md`, workspace `Cargo.toml`)
  climbs faster than throughput.
- **Background worker pool with shared state**: the Claude harness has
  no persistent shared state across sub-agents; emulating it via files
  reintroduces ADR-0002 cost without cross-repo benefit.
- **Single agent with longer turn budget**: same failure modes;
  compaction loss observed within ~6 long turns.

## Consequences

Positive:

- Measured 3–4× throughput on PLAN.md closures per real-time hour vs
  single-agent baseline.
- Each sub-agent has a small focused context (one ⏳, one file-path
  family) — compaction risk low.
- Disjoint file-path families act as natural mutex domains; no
  cross-agent write conflicts observed in the tick batches landed via
  commits `bf283025f` and `96ae82e73`.

Negative:

- Per-tick wall-clock is bounded by the slowest of four agents — a
  stuck agent stalls batch verify until timeout.
- Race risk remains nonzero on shared infra files (`PLAN.md`, root
  `Cargo.toml`, `Cargo.lock`); mitigated but not eliminated by
  file-path-family ownership. Operator hand-arbitrates on conflict.
- Larger artefact (4 commits per tick) makes `git log` denser; mitigated
  by Conventional Commits and the `yolo tick N` rubric.

## References

- Skill: `.claude/skills/aphrody-yolo-grind/`.
- Forced-loop wrapper: `.claude/skills/aphrody-perfect-grind/`
  (commit `4fb5793ab`).
- Companion narrative: `docs/posts/2026-05-yolo-grind-loop.md`.
- Commit `50bbba056` — `aphrody-yolo-grind` skill + `yolo-prod-ready`
  agent.
- Commit `bf283025f` — first landed 4-way tick.
- Commit `96ae82e73` — yolo tick 6, 4-way oracle FAIL closure.
- Related: ADR-0002 (file-based A2A — cross-repo analogue of this
  intra-repo coordination).
