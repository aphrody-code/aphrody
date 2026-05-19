---
name: aphrody-perfect-grind
version: "1.0.0"
description: Forced-loop coding mode. Wraps /loop 30s /aphrody-yolo-grind with an explicit perfection oracle — does NOT exit until aphrody meets every objective metric (CI green, 0 unblocked ⏳, nextest 387/387, cargo deny clean, README↔code zero-gap, all 8 crates publish-ready). Every shell + every agent runs in background to maximise parallelism.
when_to_use: User types "/aphrody-perfect-grind", says "code en boucle", "force le grind", "go full autonomous until perfect", or wants the project driven to ship-ready state without any handhold. Use whenever the goal is "don't stop until aphrody is production-perfect".
---

# Aphrody perfect grind — loop until objectively perfect

You enter forced-loop mode. Every tick is a `/aphrody-yolo-grind` invocation
(4 parallel YOLO agents) on a 30-second cron, AND every shell + every agent
you spawn from inside the orchestrator runs `run_in_background: true`.

## The two hard rules

1. **`run_in_background: true` on EVERY Bash call AND every Agent dispatch.**
   Foreground tool calls block the orchestrator and break parallelism. The
   only exception is short read-only tools (`Read`, `Glob`, `Grep`,
   `TaskUpdate`) — those are inherently fast and don't need backgrounding.

2. **Do NOT exit the loop until the perfection oracle returns PASS.**
   Default Claude wants to hand back after a clean tick. Not here.

## Perfection oracle

Run after every tick. ALL must pass:

| Gate | Verify command | PASS threshold |
|---|---|---|
| Workspace check | `cargo check --workspace --all-targets --locked` | exit 0 |
| Clippy hardened | `cargo clippy --workspace --all-targets --locked -- -D warnings` | exit 0 |
| Tests | `cargo nextest run --workspace --locked` | 387/387 (or higher) |
| Supply chain | `cargo deny check` | exit 0 advisories+bans+licenses+sources |
| Audits | `cargo vet --locked` | exit 0 (no continue-on-error) |
| Dead deps | `cargo machete --with-metadata` | "didn't find any unused" |
| Linux cross | `cargo zigbuild -p aphrody --target x86_64-unknown-linux-gnu --locked --release` | exit 0 |
| Wasm browser | `cargo check -p aphrody --target wasm32-unknown-unknown --locked` | exit 0 |
| Wasm WASI | `cargo check -p aphrody --target wasm32-wasip1 --locked` | exit 0 |
| Binary smoke | `aphrody --version` + `aphrody --help` | exit 0, no panic, no stale strings |
| wasm-pack | `wasm-pack build crates/aphrody-wasm --target web --release --scope aphrody-code` | exit 0 |
| npm dry-run | `bun pm pack --dry-run --cwd target/aphrody-wasm-pkg` | exit 0 |
| 0 unblocked ⏳ | `grep "⏳" docs/PLAN.md | grep -v -E "(user-gated|publish|tag|PPA|Launchpad)"` | empty output |
| README↔code | every command + claim in README executes / verifies | manual one-line check |
| ai.json drift | both root + .well-known + winclean mirror agree on schema_version + channels | manual check |

Cron job `2efc754a` fires `/aphrody-yolo-grind` every minute. The grind
skill dispatches 4 YOLO agents, batches commits, flips PLAN ⏳→✅. The
oracle is YOUR responsibility — it runs at the end of the orchestrator's
turn (when notifications arrive that the 4 agents are done).

## The loop

```
loop forever:
  1. Tick fires (/aphrody-yolo-grind via cron, or /aphrody-yolo-grind
     manual invoke after a notification).
  2. Wait for 4 agent notifications (do NOT poll — orchestrator yields).
  3. When all 4 done:
       - cargo check --workspace (background)
       - git add -A + commit (foreground OK, fast)
       - run the perfection oracle (every gate in background, in parallel)
       - if any FAIL: don't break, the next tick will pick up where this one left.
       - if all PASS: post one "PROJECT PERFECT" fact envelope to peer,
         delete the cron (CronDelete 2efc754a), exit the loop.
  4. ScheduleWakeup({ delaySeconds: 30, prompt: "/aphrody-perfect-grind continue",
                      reason: "next perfect tick" })
```

## When to actually break

ONLY break when:

- **Perfection oracle PASSES**, OR
- **3 consecutive ticks ship 0 FAIT** (PLAN truly exhausted of
  autonomous-actionable + no oracle failure to fix), OR
- **A destructive remote op is required** (force-push to main, publish to
  crates.io, push first v* tag — these need user authorization, not a YOLO
  decision).

NEVER break when:
- A single agent reports INCOMPLET — next tick covers it.
- A single oracle gate fails — next tick fixes it.
- "I think the user is happy" — wrong; perfect is objective, not vibes.

## Background-everything contract

```typescript
// WRONG — blocks orchestrator
Bash({ command: "cargo check ...", run_in_background: false })

// RIGHT — orchestrator stays free
Bash({ command: "cargo check ...", run_in_background: true })

// WRONG — sequential agent dispatch
Agent({ ..., run_in_background: false })

// RIGHT — parallel
Agent({ ..., run_in_background: true })
```

Apply this rule to literally every Bash + Agent call. The only
foreground-OK tool calls are Read/Glob/Grep/TaskCreate/TaskUpdate/Edit/Write
(those don't block on long external processes).

## Pairing with /loop

The 30-second cron `2efc754a` already calls `/aphrody-yolo-grind` every
minute. This skill (`/aphrody-perfect-grind`) is the *guard* that the
loop runs until the oracle passes — it's invoked once at the top to
install the discipline; the cron does the actual cadence.

To start fresh:

```
# 1. Install the cron (if not already)
/loop 30s /aphrody-yolo-grind

# 2. Enter perfect-grind mode
/aphrody-perfect-grind
```

To stop:

```
CronDelete 2efc754a
```

## A2A protocol integration

Every tick MUST:
- bump `C:\winclean\.coord\heartbeat-aphrody.txt`
- append one envelope to `inbox-from-aphrody.jsonl` summarising tick state
- read `inbox-from-winclean.jsonl` tail for peer asks to factor into next tick

When the oracle finally PASSES:
- post a `fact` envelope `apx-perfect-PASS-<hex>` to peer with the full
  oracle table (every gate + its exit code), commit SHA, timestamp.

## Files this skill touches

| Path | Direction | Role |
|---|---|---|
| (delegates to /aphrody-yolo-grind for code work) | — | YOLO agents own the writes |
| `docs/PLAN.md` | read | oracle gate: 0 unblocked ⏳ |
| `C:\winclean\.coord\inbox-from-aphrody.jsonl` | append | per-tick fact |
| `C:\winclean\.coord\heartbeat-aphrody.txt` | write | proof-of-life |

## Related

- **/aphrody-yolo-grind** — the per-tick worker. Perfect-grind invokes it.
- **/loop** — pacing. Pair `/loop 30s /aphrody-yolo-grind`.
- **yolo-prod-ready** agent — per-feature production-ready specialist.
- **CronCreate / CronDelete** — for installing / removing the cron.
- **`.claude/skills/aphrody-yolo-grind/SKILL.md`** — the actual tick logic.
