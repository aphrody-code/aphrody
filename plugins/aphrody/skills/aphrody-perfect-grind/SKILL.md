---
name: aphrody-perfect-grind
version: "2.0.0"
description: Forced-loop coding mode. Wraps /loop 30s /aphrody-yolo-grind with an explicit perfection oracle — does NOT exit until the codebase meets all objective metrics. Every shell + every agent runs in background to maximize parallelism.
when_to_use: User types "/aphrody-perfect-grind", says "code en boucle", "force le grind", "go full autonomous until perfect", or wants the project driven to ship-ready state without any handhold. Use whenever the goal is "don't stop until the project is production-perfect".
---

# Universal perfect grind — Loop until objectively perfect

Mode `/goal` permanent: persistent objective, zero confirmation, do not exit until the perfection oracle returns PASS.

Forced-loop mode. Every tick is a `/aphrody-yolo-grind` invocation on a 30-second cron; every shell + agent spawned from the orchestrator runs with `run_in_background: true`.

## 1. The Two Hard Rules

1. **`run_in_background: true` on EVERY Bash call AND every Agent dispatch.**
   Foreground tool calls block the orchestrator and break parallelism. The only exception is short read-only tools (`Read`, `Glob`, `Grep`, `TaskUpdate`) — those are inherently fast and don't need backgrounding.

2. **Do NOT exit the loop until the perfection oracle returns PASS.**
   Default Claude wants to hand back after a clean tick. Not here.

## 2. Dynamic Perfection Oracle

Runs after every tick. All gates relevant to the detected project type must pass:

### 2.1 Workspace & Compilation Gates
- **Rust**: `cargo check --workspace --all-targets --locked` and `cargo clippy --workspace --all-targets --locked -- -D warnings` must exit 0.
- **Bun / Node.js**: `bun run lint` / `npm run lint` or `oxlint` must exit 0.
- **Python**: `ruff check` / `ruff format --check` must exit 0.
- **Go**: `go vet ./...` must exit 0.

### 2.2 Testing Gates
- **Rust**: `cargo test` / `cargo nextest run` passes all tests.
- **Bun / Node.js**: `bun test` or `npm test` passes all tests.
- **Python**: `pytest` passes all tests.
- **Go**: `go test ./...` passes all tests.

### 2.3 Dependency & Supply Chain Gates
- Checks security/bans/licenses where available (e.g. `cargo deny check` / `npm audit` / `uv pip-audit`).
- Detects unused dependencies where applicable (e.g. `cargo machete` or equivalent).

### 2.4 Plan Gate (0 Unblocked Items)
- The detected plan/todo file must contain 0 open/unblocked items (e.g. searching for `⏳` or open checkbox `- [ ]` returns no unblocked tasks).

## 3. The Loop

```
loop forever:
  1. Tick fires (/aphrody-yolo-grind via cron or manual invocation).
  2. Wait for background agent notifications (orchestrator yields, does not poll).
  3. When done:
       - Run project validation suite (background).
       - Git add + commit staged changes (foreground OK, fast).
       - Run the perfection oracle (every gate in background, in parallel).
       - If any gate FAILS: do not break, the next tick will pick up.
       - If all gates PASS: delete the cron/loop timer and exit.
  4. ScheduleWakeup({ delaySeconds: 30, prompt: "/aphrody-perfect-grind continue" })
```

## 4. When to Break

ONLY break when:
- **Perfection oracle PASSES**, OR
- **3 consecutive ticks ship 0 FAIT** (plan truly exhausted of autonomous-actionable tasks), OR
- **A destructive remote operation is required** (force-pushing to main, publishing package) — end the loop and document the gate; do not perform it autonomously.

NEVER break when:
- A single agent reports INCOMPLET — next tick covers it.
- A single oracle gate fails — next tick fixes it.
- "I think the user is happy" — wrong; perfect is objective, not vibes.

## 5. A2A Peer Coordination (If Enabled)

If A2A directory `.coord/` is detected:
- Bumps heartbeat file `.coord/heartbeat-<agent>.txt` on every tick.
- Appends to `.coord/inbox-from-<agent>.jsonl` with tick states.
- When the oracle passes: posts a final `fact` envelope to peer containing the full oracle results, commit SHA, and timestamp.s/aphrody-yolo-grind/SKILL.md`** — the actual tick logic.
