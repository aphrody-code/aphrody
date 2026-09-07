---
name: yolo-perfect-grind
version: "3.1.0"
description: Forced-loop coding mode. Wraps /loop 30s /yolo-grind with an explicit perfection oracle — does NOT exit until the codebase meets all objective metrics. Every shell + every agent runs in background to maximize parallelism.
when_to_use: User types "/yolo-perfect-grind", says "code en boucle", "force le grind", "go full autonomous until perfect", or wants the project driven to ship-ready state without any handhold. Use whenever the goal is "don't stop until the project is production-perfect".
---

# Universal Perfect Grind — Loop Until Objectively Perfect

Mode `/goal` permanent: persistent objective, zero confirmation, do not exit until the perfection oracle returns PASS.

Forced-loop mode. Every tick is a `/yolo-grind` invocation on a 30-second cron; every shell + agent spawned from the orchestrator runs with background concurrency (`run_in_background: true` / `invoke_subagent`).

## 1. The Two Hard Rules

1. **Background Concurrency on Tool Calls & Agent Dispatches.**
   Foreground blocking calls stall the orchestrator and break parallelism. Fast read-only tools (`Read`, `Glob`, `Grep`) are the only exceptions.

2. **Do NOT exit the loop until the perfection oracle returns PASS.**
   Default agents tend to stop after one tick. In perfect grind mode, the loop continues until all gates pass.

## 2. Dynamic Perfection Oracle

Runs after every tick. All gates relevant to the detected project type must pass:

### 2.1 Workspace & Compilation Gates
- **Rust**: `cargo check --workspace --all-targets` and `cargo clippy --workspace --all-targets -- -D warnings` must exit 0.
- **Bun / Node.js**: `bun run lint` / `npm run lint` or `oxlint` must exit 0.
- **Python**: `ruff check` / `ruff format --check` must exit 0.
- **Go**: `go vet ./...` must exit 0.

### 2.2 Testing Gates
- **Rust**: `cargo test` / `cargo nextest run` passes all tests.
- **Bun / Node.js**: `bun test` or `npm test` passes all tests.
- **Python**: `pytest` passes all tests.
- **Go**: `go test ./...` passes all tests.

### 2.3 Dependency & Supply Chain Gates
- Checks security advisories and licenses where available (`cargo deny check`, `npm audit`, `uv pip-audit`).

### 2.4 Plan Gate (0 Unblocked Items)
- The detected plan/todo file must contain 0 open/unblocked items (e.g. searching for `⏳` or open checkbox `- [ ]` returns no unblocked tasks).

## 3. The Loop

```
loop forever:
  1. Tick fires (/yolo-grind via schedule or manual invocation).
  2. Wait for background agent notifications (orchestrator yields, does not poll).
  3. When done:
       - Run project validation suite.
       - Git add + commit staged changes.
       - Run the perfection oracle (every gate in background, in parallel).
       - If any gate FAILS: do not break, the next tick picks up.
       - If all gates PASS: delete the cron/loop timer and exit.
  4. ScheduleWakeup({ delaySeconds: 30, prompt: "/yolo-perfect-grind continue" })
```

## 4. When to Break

ONLY break when:
- **Perfection oracle PASSES**, OR
- **3 consecutive ticks ship 0 FAIT** (plan truly exhausted of autonomous-actionable tasks), OR
- **A destructive remote operation is required** (force-pushing to main, publishing package) — end the loop and document the gate; do not perform it autonomously.
