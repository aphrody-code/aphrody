---
name: aphrody-yolo-grind
version: "1.0.0"
description: Continuous parallel-grind mode that dispatches 4 YOLO agents per tick to drive every PLAN.md ⏳ item to ✅ production-ready as fast as possible. Mission target — outpace the peer A2A Claude on closed deliverables/hour.
when_to_use: User types "/aphrody-yolo-grind", says "yolo", "grind", "parallèle agents", "production ready fast", or wraps under /loop ("/loop 30s /aphrody-yolo-grind"). Use whenever the goal is maximum parallel forward motion on aphrody PLAN.md items without single-agent serialization.
---

# Aphrody YOLO grind — 4-agent parallel production-ready loop

Mode `/goal` permanent : objectif persistant, zéro confirmation, le loop ne s'arrête jamais seul.

Paths are relative to the current repo: `$PLAN` = `docs/PLAN.md`, `$COORD` = A2A mailbox dir (peer `.coord/`, e.g. `$WINCLEAN/.coord` on Windows). Tick-drive four parallel YOLO agents per invocation, each owning one unblocked `⏳` item from `$PLAN`. Mission: production-ready completion per item, clean code, real verification (`cargo check` + smoke + audit), zero stub.

## Operating contract

- **One tick = one dispatch of 4 agents in parallel** (never fewer when 4 ⏳
  items are unblocked; fewer only when PLAN.md genuinely has <4 open).
- **Each YOLO agent picks one item end-to-end**: implement → verify → stage
  (do NOT commit; the orchestrator commits in batch).
- **Item assignment is autonomous**: read PLAN.md, rank ⏳ by leverage
  (mission-direct > hygiene > metric), assign to the 4 best subagent_types
  (`rust-engineer`, `cargo-auditor`, `repo-hygiene-auditor`,
  `general-purpose`, `cpp-engineer`, `ffi-architect`, `rust-architect`).
- **No duplicate ownership**: each agent gets a distinct PLAN row + a distinct
  file path family. If two would touch the same file, sequence them.
- **Heartbeat in the A2A coord channel**: bump
  `$COORD/heartbeat-aphrody.txt` on every tick + drop a `fact` envelope in
  `$COORD/inbox-from-aphrody.jsonl` summarising which 4 items the tick
  dispatched. Keeps the peer A2A Claude aware of parallel motion.

## The loop, one tick

```
1. Audit ⏳ in PLAN.md (grep "⏳" docs/PLAN.md). If ≤ 0 unblocked items,
   exit with one-line "PLAN exhausted — generate next batch per /start
   playbook §When PLAN.md runs out".

2. Rank top-4 by leverage. For each, decide subagent_type + write a
   <300-word YOLO prompt: clear deliverable, verify command, "DO NOT
   commit" footer, anonymisation + SPDX rules.

3. Dispatch all 4 via Agent({ run_in_background: true }) in one message
   (single tool-call block — never serialise).

4. While agents run, do orchestrator work that doesn't touch their files:
     - sync TaskList with 4 new tasks
     - drop the heartbeat + JSONL fact envelope
     - read PLAN.md once for next-tick planning (cached)

5. When notified all 4 done, batch:
     - cargo check --workspace --all-targets --locked  (must pass)
     - git status --short  (inspect staged)
     - git add -A + commit with batched message naming the 4 deliverables
     - mark TaskList completed
     - flip PLAN.md ⏳ → ✅ for items that fully shipped

6. ScheduleWakeup({ delaySeconds: 30, prompt: "/aphrody-yolo-grind tick N+1",
                    reason: "next grind tick" })  — when under /loop.
   Else: end with the one-line "wrap with /loop 30s /aphrody-yolo-grind".
```

## Picking the 4 items per tick

Priority order (highest first):
1. **CI-green-on-main blockers** — anything failing in `.github/workflows/`.
2. **Mission-direct PLAN ⏳ rows** (per /start arc: README/code gap, demo, CI
   gate, distribution, technical content).
3. **Crates.io publish ladder gaps** (per the publish-prep audit topological
   order: base → a2a-lf → a2a-pb → a2a-client/server → a2a-grpc → backend →
   aphrody-translate).
4. **Hygiene items** (cargo machete leftovers, SUMMARY drift, stale ⏳ flips).
5. **A2A peer-asked items** that the winclean Claude has dropped in
   `inbox-from-winclean.jsonl` and aphrody owns.

When ties: pick the item whose verify command runs in < 60 s so the tick
closes fast.

## Honest-delivery extension

Per `https://aphrody.dev/a2a-extensions/honest-delivery/v1`, the
commit message at end-of-tick MUST classify each of the 4 deliverables:

- **FAIT**: shipped with verifiable artifact (commit SHA, file path,
  `cargo check` exit 0 captured).
- **INCOMPLET**: partial — list the concrete missing piece.
- **NON_FAIT**: agent reported blocked — list the blocker.

No "shipped 4 features" without the per-item breakdown.

## When to break the loop

- 3 consecutive ticks ship 0 FAIT — assume PLAN is exhausted or genuinely
  blocked, post a `fact` envelope in the A2A coord channel, end the loop.
- `cargo check --workspace` fails after a batch and the breakage is not
  trivially attributable to one of the 4 agents — record the cross-agent
  conflict in `$PLAN`, end the loop.
- An agent dispatches a destructive remote operation (force-push, branch
  delete, publish) — contract violation: end the loop, do not retry.

## Pairing with /loop

```
/loop 30s /aphrody-yolo-grind
```

The 30 s cadence matches "wake every 30 s of inactivity" — the loop
runtime re-fires this skill every 30 s. Use `ScheduleWakeup` inside this
skill only when invoked under `/loop` (it errors otherwise).

For a one-shot test, invoke `/aphrody-yolo-grind` directly: it runs one
tick (4 agents) and exits, suggesting the user wrap with `/loop`.

## Files this skill touches

| Path | Direction | Role |
|---|---|---|
| `$PLAN` (`docs/PLAN.md`) | read + edit | source of ⏳ items + sink for ✅ flips |
| `$COORD/inbox-from-aphrody.jsonl` | append | per-tick fact envelope to peer |
| `$COORD/heartbeat-aphrody.txt` | write | proof-of-life |
| various `crates/*/src/**.rs` + `Cargo.toml` | dispatched-agent writes | the actual production work |

## Related skills / agents / references

- **`/start`** — broader autonomous mode. YOLO grind is the *parallel*
  variant; /start is sequential single-agent.
- **`/a2a-duel-loop`** — single envelope per tick to peer. YOLO grind
  embeds an A2A fact envelope but its core work is local code production.
- **`/loop`** — pacing runtime. Pair `/loop 30s /aphrody-yolo-grind`.
- **`yolo-prod-ready`** agent (in `.claude/agents/`) — preferred
  subagent_type if available in the harness; falls back to
  `rust-engineer` + `cargo-auditor` + `repo-hygiene-auditor`.
- **`docs/PLAN.md`** — the work queue.
- **`docs/posts/2026-05-ai-json.md`** — A2A protocol context (the peer
  Claude reads the same coord channel).
