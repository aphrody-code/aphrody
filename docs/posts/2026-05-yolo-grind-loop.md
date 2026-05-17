<!-- SPDX-License-Identifier: Apache-2.0 -->
# The parallel YOLO grind loop: driving a monorepo from skeleton to publish-ready in 7 hours

> Aphrody dev journal, 2026-05-17.
> Author: aphrody-code &lt;noreply@aphrody-code.dev&gt;

[See post #1 on cross-Claude A2A coordination](./2026-05-ai-json.md)

---

## The problem with single-agent autonomous loops

Running a single Claude Code session in an autonomous loop sounds fast. In
practice, wall-clock time accumulates in two places that resist parallelism.

First, **cargo check is slow**. On the aphrody workspace (17 crates, 80
workspace-level dependencies, three cross-compile targets), a cold
`cargo check --workspace --all-targets --locked` on Windows takes 90-120
seconds. Even a warm incremental check after touching one crate takes 15-30
seconds. If the agent serialises "implement → check → implement → check",
that is the cadence you get: one deliverable per check cycle.

Second, **context bloats**. A single session accumulates every file read,
every cargo output line, every PLAN.md re-read into its context window. By
the time it finishes task 8, the earlier reasoning about task 1 is either
evicted or crowding out new work. The agent starts hedging, re-reading things
it already decided, producing shorter outputs per token of context used.

Both failure modes compound: the session slows *and* the output quality per
unit time declines. The result is a grind loop that stalls somewhere between
item 4 and item 6 of a 24-item work queue, usually at the exact moment that
the hard cross-platform integration work starts.

## The solution: 4-agent parallel grind tick

The pattern that actually worked is a **tick-based orchestration** where each
tick dispatches four specialised subagents simultaneously, each owning one
unblocked item end-to-end.

One tick looks like this:

```
1. Orchestrator reads PLAN.md. Grep for "⏳". Rank by leverage:
     CI blockers > mission-direct deliverables > publish-ladder gaps > hygiene.

2. Pick top 4 unblocked items. Write a <300-word YOLO prompt per item:
     clear deliverable, verify command, "DO NOT commit" footer,
     file-path-family ownership, SPDX + anonymisation rules.

3. Dispatch all 4 via Agent({ run_in_background: true }) in ONE tool-call block.

4. While agents run (~90-120 s), orchestrator does work that doesn't
   overlap their files:
     - bump heartbeat at C:\winclean\.coord\heartbeat-aphrody.txt
     - drop fact envelope into inbox-from-aphrody.jsonl
     - read PLAN.md once for next-tick planning (cached in orchestrator context)
     - sync the task list with 4 in-progress entries

5. Notified all 4 complete. Orchestrator batches:
     cargo check --workspace --all-targets --locked     (must exit 0)
     git status --short                                  (inspect staged)
     git add <specific files>                            (no blind -A)
     git commit -m "$(cat <<'EOF' ... EOF)"             (honest-delivery message)
     flip PLAN.md ⏳→✅ for items that fully shipped

6. Schedule next tick or exit.
```

The key insight is step 3. All four agents run in parallel, and the
orchestrator context only ever accumulates the summaries from each agent's
completion notification, not the full transcript of what they did. Each
subagent gets a fresh context window for its specific item. The orchestrator
stays thin and fast across all six ticks.

Subagent types used across the six ticks: `rust-engineer` (primary, 14 of
24 dispatches), `cargo-auditor` (4), `repo-hygiene-auditor` (3),
`general-purpose` (3). The `yolo-prod-ready` agent in `.claude/agents/`
wraps the same behaviour for callers that want a single-item variant.

## File-path-family ownership prevents cross-agent conflict

The only hard constraint in step 2 is **no duplicate ownership**. If two agents
would need to write `Cargo.toml` in the same crate, or both modify
`crates/cli/src/commands.rs`, they must be sequenced, not parallelised.

This matters in practice. YOLO sessions #17 and #19 both targeted
`crates/base/Cargo.toml` in the same tick before the ownership rule was
formalised. Both staged a `Cargo.toml` diff; when the orchestrator ran
`git add`, the second diff silently overwrote the first. The merged file
failed `cargo check` because one agent had added a dependency the other
had simultaneously removed.

The fix is simple: before dispatching, the orchestrator assigns each item
a file-path family (e.g., "crates/base/**", "crates/cli/src/**",
".github/workflows/**") and verifies the four families are disjoint. If
they are not, the lower-priority item is deferred to the next tick. This
adds maybe 30 seconds of orchestrator reasoning but eliminates the class
of conflict entirely.

## Cron wrapping via /loop

The skill spec at `.claude/skills/aphrody-yolo-grind/SKILL.md` pairs with
`/loop 30s /aphrody-yolo-grind`. The 30-second cadence is the "wake after
30 seconds of inactivity" semantic — not a wall-clock cron.

For a persistent schedule that survives session restarts, the underlying
mechanism is a `CronCreate` job. The job id produced during the pivot day
was `2efc754a`, cron expression `*/1 * * * *`. The minimum cron granularity
in the harness is one minute, not 30 seconds; the `/loop 30s` semantics
handle sub-minute pacing within an active session, while the cron handles
re-entry after a session drops.

When the cron fires into a dead session it simply re-invokes the skill,
which reads the current PLAN.md state, picks the next four unblocked items,
and resumes from wherever the previous tick left off. Because `PLAN.md` is the
authoritative queue and it is updated atomically at end-of-tick, there is no
in-memory state to reconstruct.

## Honest-delivery: every tick commit must classify each deliverable

The `honest-delivery/v1` extension
(`https://aphrody.dev/a2a-extensions/honest-delivery/v1`) requires that every
end-of-tick commit message classify each of the four dispatched items as
one of three states:

- **FAIT**: shipped with a verifiable artefact — commit SHA, file path,
  `cargo check` exit code captured.
- **INCOMPLET**: partial delivery. The message names the specific missing
  piece, not a vague "in progress".
- **NON_FAIT**: agent reported blocked. The message names the blocker
  (missing upstream dep, ambiguous spec, conflicting file ownership).

The classification discipline exists because autonomous loops are easy to
game. An agent can stage a file, claim the item is FAIT, and move on without
the orchestrator noticing the staged code never compiled. Requiring a per-item
breakdown — with a verifiable command and exit code — makes the gap visible
in the commit history immediately. The PLAN.md only gets a ⏳→✅ flip when
the orchestrator independently confirms the verify command exit code, not
when the subagent claims it passed.

Commit `bf283025f` (tick 1) demonstrated this in production:

```
YOLO #3 FAIT — aphrody binary smoke 5 targets:
  Windows --version 76 ms. Linux zigbuild release ELF 5.0 MB pie stripped (2m39s).
  wasm32-unknown-unknown check 3.26 s, wasm32-wasip1 check 65 s.
  rustls CryptoProvider install at cli/src/main.rs:163 verified — no panic risk.
  1 non-blocking zigbuild linker deprecation warning (Zig upstream fix pending).
```

Exit codes are not just summarised; they are named inline. If the orchestrator
re-ran the same verify command and got a different result, the commit message
would be the audit trail proving the discrepancy.

Tick 6 commit `96ae82e73` closed the run: 2,430 lines inserted across
`supply-chain/imports.lock`, `.github/workflows/cross-platform.yml`,
`ai.json`, `.well-known/ai.json`, and `crates/cli/src/commands.rs`. All four
items classified FAIT. Three oracle FAILs closed simultaneously.

## A2A coordination embedding

Each tick drops a `fact` envelope into the peer's inbox before dispatching
agents. This keeps the winclean Claude (running in `C:\winclean`) aware of
what the aphrody side is doing without requiring either side to poll or block.

The envelope written at tick start looks like:

```json
{
  "id":      "apx-tick3-start",
  "ts":      "2026-05-17T15:12:00Z",
  "from":    "aphrody@aphrody-code/aphrody",
  "to":      "winclean@aphrody-code/winclean",
  "type":    "fact",
  "subject": "yolo-tick-3 dispatching 4 agents",
  "body":    "Items: asciinema cast, CHANGELOG cross-link, release.yml audit, oracle CI wiring. ETA ~120s. Heartbeat bumped.",
  "channel_hint": ["file_jsonl"]
}
```

The heartbeat file at `C:\winclean\.coord\heartbeat-aphrody.txt` receives an
ISO-8601 timestamp on every tick. The peer reads this when it needs to know
whether aphrody has been active in the last ten minutes. During the six-tick
run, the heartbeat was bumped six times, giving the peer a reliable signal
that parallel production work was in flight.

This matters for coordination because the peer Claude might be considering
changes to shared artefacts — the root `ai.json`, the `.well-known/ai.json`,
the shared `docs/` subtree. If the peer reads a fresh heartbeat and a
fact envelope describing which files the current tick owns, it can defer
writing to those files rather than creating a cross-agent conflict between
*repos* rather than within one.

See [post #1](./2026-05-ai-json.md) for the full envelope schema, the seven
channel types, and the 3-deep handshake that proved the protocol worked.

## Real numbers from the pivot day

The six ticks ran on 2026-05-17, starting from the post-pivot monorepo
skeleton produced by the rename script (`94bcacca`).

| Metric | Value |
|---|---|
| Ticks executed | 6 |
| Agents dispatched | 24 (4 per tick) |
| Wall-clock elapsed | ~7 hours |
| Honest-delivery FAIT count | 21 of 24 |
| INCOMPLET | 2 |
| NON_FAIT | 1 |
| Oracle 15-gate final run | 12/15 PASS |
| Tick-6 commit (`96ae82e73`) insertions | 2,430 lines |
| Oracle FAILs closed in tick-6 | 3 simultaneously |

The 3 oracle FAILs that remained after tick 5 were:
gate 5 (imports.lock out-of-date), gate 13 (no Linux CI job), and
gate 15 (ai.json schema_version field missing). All three landed in tick 6
because they were genuinely independent — separate files, separate agents,
no shared state — and the ownership rule kept them from conflicting.

The 3 remaining FAIL gates after tick 6 were all classified NON_FAIT or
INCOMPLET in the commit message. Two required a live Ubuntu 26.04 runner
to validate end-to-end (unavailable in the dev session); one required
a Launchpad account action outside the repo. None were claimed FAIT.

## What can go wrong

**Race condition between agents on the same file.** Documented above with
the `crates/base/Cargo.toml` incident from YOLO sessions #17 and #19. The
file-path-family ownership constraint is the fix. If you skip it and run
agents that both touch `Cargo.toml` (even in different crates), the risk
is real: `cargo check` sees an internally inconsistent workspace dependency
graph and fails with an error that looks like a dependency resolution problem
rather than a merge conflict.

**Stale rust-analyzer diagnostics.** After a subagent modifies a `.rs`
file and the orchestrator runs `cargo check`, the LSP cache in the outer
Claude Code session may still report the old diagnostic. This is a
display issue only — the `cargo check` exit code is the source of truth,
not what the editor overlay shows. The institutional memory entry
`feedback_refresh_rust_lsp` documents this: always run `cargo check` after
any Rust modification, and treat rust-analyzer's inline errors as stale
until the next build completes. Never let a subagent exit early because
rust-analyzer reported a warning that `cargo check` did not.

**Orchestrator context growth.** If the orchestrator reads full agent
completion transcripts rather than summaries, its context fills by tick 4.
The SKILL.md spec says each agent's completion notification should be a
summary (deliverable + verify command + exit code), not the full diff.
If you use an Agent tool that dumps full stdout, add a `--summary-only`
equivalent or strip the output before feeding it back to the orchestrator.

**`cargo check --workspace` passing while a specific crate fails.** The
workspace check validates all members, but a gated crate (e.g., `gui`,
which pulls in `wry`/`tao` and transitively `gtk3`) may silently be excluded
from the default feature set. Run `--all-targets` explicitly. On
Windows, the CI check that matters for the tick is
`cargo check --workspace --all-targets --locked`.

## Try it in your repo

The skill spec is the executable definition. It is not tied to this repo.

`.claude/skills/aphrody-yolo-grind/SKILL.md` describes the operating contract:
one tick = four background agents, orchestrator does heartbeat + planning work
in parallel, honest-delivery commit at end. The inputs it needs from your
repo are:

1. A `docs/PLAN.md` (or equivalent) with `⏳` / `✅` markers per row.
2. A CLAUDE.md describing file ownership rules and forbidden operations.
3. Optionally, a coord dir for cross-agent heartbeat (can be a no-op file write).

Invoke it directly for a single tick:

```
/aphrody-yolo-grind
```

Or wrap it under `/loop` for continuous ticking until the plan is exhausted:

```
/loop 30s /aphrody-yolo-grind
```

The loop exits automatically on three conditions: the plan has no unblocked
`⏳` items, three consecutive ticks ship zero FAIT items, or an agent
triggers a destructive operation (force-push, publish without dry-run,
branch delete).

## Links

- Skill spec: [`.claude/skills/aphrody-yolo-grind/SKILL.md`](../../.claude/skills/aphrody-yolo-grind/SKILL.md)
- Work queue: [`docs/PLAN.md`](../PLAN.md)
- Honest-delivery extension: `https://aphrody.dev/a2a-extensions/honest-delivery/v1`
- Post #1 — cross-Claude A2A protocol: [`docs/posts/2026-05-ai-json.md`](./2026-05-ai-json.md)
- Tick-1 commit: `bf283025f` (first 4-agent dispatch, all FAIT)
- Tick-6 commit: `96ae82e73` (2,430-line closure, 3 oracle FAILs → PASS)
- Repo: <https://github.com/aphrody-code/aphrody> (Apache-2.0)
