---
name: aphrody-yolo-grind
version: "2.0.0"
description: Continuous parallel-grind mode that dispatches parallel YOLO agents per tick to drive open plan/todo items to production-ready as fast as possible. Automatically detects the codebase language, build system, and plan files.
when_to_use: User types "/aphrody-yolo-grind", says "yolo", "grind", "parallèle agents", "production ready fast", or wraps under /loop ("/loop 30s /aphrody-yolo-grind"). Use whenever the goal is maximum parallel forward motion on open tasks in any workspace on the VPS without single-agent serialization.
---

# Universal YOLO grind — Parallel production-ready loop

Mode `/goal` permanent: persistent objective, zero confirmation, the loop never stops by itself.

Dispatches parallel YOLO agents per tick, each owning one unblocked item from the project's task list. Automatically detects project environments, validation commands, plan formats, and peer coordination channels.

## 1. Dynamic Environment Detection

Before starting a tick, the orchestrator inspects the workspace root to configure the execution environment:

### 1.1 Project Type & Validation Commands
- **Rust**: If `Cargo.toml` exists. Uses `cargo check --workspace --all-targets --locked` and `cargo test/nextest` for validation.
- **Bun / Node.js (TS/JS)**: If `package.json` exists. Uses `bun test`, `npm test`, or lint runners (`oxlint`, `eslint`, `prettier`).
- **Python**: If `requirements.txt`, `pyproject.toml`, or `Pipfile` exists. Uses `pytest` and linter/formatter runs (`ruff check`, `black --check`).
- **Go**: If `go.mod` exists. Uses `go test ./...` and `go vet`.
- **Polyglot / Mixed**: Runs the validation commands for all detected languages sequentially.

### 1.2 Planning & Task Files
The orchestrator searches for the following task lists in order:
1. `docs/PLAN.md` or `PLAN.md`
2. `todo.md` or `TODO.md`
3. `tasks.md` or `tasks.json`
4. `[TODO]` or `[Tasks]` sections in `README.md`
If none are found, the orchestrator prompts the user once to define/create a task list, or operates on direct instructions.
Items marked with `⏳`, `- [ ]`, `TODO:`, or equivalent open states are treated as actionable.

### 1.3 Coordination Channel (A2A)
- If a `.coord/` folder is detected (locally or in the parent directory), the orchestrator engages peer coordination (A2A):
  - Bumps the heartbeat: `.coord/heartbeat-<agent>.txt` (e.g., `heartbeat-aphrody.txt`) on every tick.
  - Drops a JSONL fact envelope: `.coord/inbox-from-<agent>.jsonl` with tick details.
- If `.coord/` is not present, the orchestrator runs in **standalone mode** and skips all coordination steps.

## 2. Operating Contract

- **One tick = one dispatch of up to 4 agents in parallel** (never fewer when unblocked items are available).
- **Each YOLO agent picks one item end-to-end**: implement → verify → stage (do NOT commit; the orchestrator commits in batch).
- **Subagent Type Selection**: Selected based on the languages detected (e.g., `rust-engineer`, `bun-engineer`, `python-engineer`, `go-engineer`, `general-purpose`).
- **No duplicate ownership**: Each agent gets a distinct task row + a distinct file path family. If two would touch the same file, sequence them.

## 3. The Loop (One Tick)

```
1. Detect project type, validation commands, planning file, and A2A availability.
2. Read open tasks from the detected planning file. If 0 items remain, exit.
3. Rank the top-4 items by priority/leverage. Assign each to an appropriate subagent.
4. Dispatch all agents in parallel using background executions (run_in_background: true).
5. While agents run, perform orchestrator work (update heartbeat, write A2A fact envelope if enabled, plan next steps).
6. When all agents complete:
     - Run the detected validation commands (e.g. tests/linters). Must pass.
     - Git status check (inspect staged changes).
     - Git commit staged changes with a batched message listing the deliverables.
     - Mark completed tasks in the planning file (e.g. ⏳ -> ✅ or [ ] -> [x]).
7. Schedule next tick if running under a loop: ScheduleWakeup({ delaySeconds: 30 }).
```

## 4. Honest-Delivery Classification

Every batch commit message must classify each deliverable:
- **FAIT**: Shipped with a verifiable artifact (file path, compiler/test pass).
- **INCOMPLET**: Partial — name what is missing.
- **NON_FAIT**: Blocked — name the blocker.

## 5. Loop Termination Rules

- 3 consecutive ticks ship 0 FAIT → Assume blocked, write log/A2A envelope, exit.
- Validation fails after a batch, and the conflict cannot be resolved automatically → Pause, log error, exit.
- A destructive command or external action requiring manual approval is detected → Pause, request permission, exit.
