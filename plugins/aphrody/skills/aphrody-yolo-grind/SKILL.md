---
name: yolo-grind
version: "3.1.0"
description: Continuous parallel-grind mode that dispatches parallel YOLO agents per tick to drive open plan/todo items to production-ready across any repository (Rust, Bun/Node, Python, Go, C/C++). Automatically detects languages, build systems, plan files, and validation commands.
when_to_use: User types "/yolo-grind", "/yolo", says "grind", "parallel agents", "production ready fast", or wraps under /loop ("/loop 30s /yolo-grind"). Use whenever the goal is maximum parallel forward motion on open tasks in any workspace without single-agent serialization.
---

# Universal YOLO Grind — Parallel Production-Ready Loop

Mode `/goal` permanent: persistent objective, zero confirmation, the loop never stops by itself.

Dispatches parallel YOLO agents per tick, each owning one unblocked item from the project's task list. Automatically detects project environments, validation commands, plan formats, and peer coordination channels.

## 1. Dynamic Environment Detection

Before starting a tick, the orchestrator inspects the workspace root to configure the execution environment:

### 1.1 Project Type & Validation Commands
- **Rust**: If `Cargo.toml` exists. Uses `cargo check --workspace --all-targets` and `cargo test` / `cargo nextest run`.
- **Bun / Node.js (TS/JS)**: If `package.json` exists. Uses `bun test` or `npm test`, plus typecheck / linters (`oxlint`, `eslint`, `prettier`).
- **Python**: If `requirements.txt`, `pyproject.toml`, or `Pipfile` exists. Uses `pytest` / `uv run pytest` and linters (`ruff check`, `black --check`).
- **Go**: If `go.mod` exists. Uses `go test ./...` and `go vet`.
- **C/C++**: If `CMakeLists.txt` or `Makefile` exists. Validates build / ctest.
- **Polyglot / Mixed**: Runs the validation commands for all detected languages sequentially.

### 1.2 Planning & Task Files
The orchestrator searches for the following task lists in order:
1. `docs/PLAN.md`, `UNIFIED-PLAN.md`, or `PLAN.md`
2. `todo.md` or `TODO.md`
3. `tasks.md` or `tasks.json`
4. `[TODO]` or `[Tasks]` sections in `README.md`

If none are found, the orchestrator creates a default `TODO.md` or operates on the direct prompt goal.
Items marked with `⏳`, `- [ ]`, `TODO:`, or equivalent open states are treated as actionable.

### 1.3 Coordination Channel (A2A)
- If a `.coord/` folder is detected (locally or in the parent directory), the orchestrator engages peer coordination (A2A):
  - Bumps the heartbeat: `.coord/heartbeat-<agent>.txt` on every tick.
  - Drops a JSONL fact envelope: `.coord/inbox-from-<agent>.jsonl` with tick details.
- If `.coord/` is not present, the orchestrator runs in **standalone mode** and skips all coordination steps.

## 2. Operating Contract

- **One tick = one dispatch of up to 4 agents in parallel** (never fewer when unblocked items are available).
- **Each YOLO agent picks one item end-to-end**: implement → verify → stage (do NOT commit; the orchestrator commits in batch).
- **Subagent Selection**: Selected based on the languages detected (e.g., `yolo-prod-ready`, `rust-engineer`, `devops-engineer`, `code-review`).
- **No duplicate ownership**: Each agent gets a distinct task row + a distinct file path family. If two would touch the same file, sequence them.

## 3. The Loop (One Tick)

```
1. Detect project type, validation commands, planning file, and A2A availability.
2. Read open tasks from the detected planning file. If 0 items remain, exit.
3. Rank the top-4 items by priority/leverage. Assign each to an appropriate subagent.
4. Dispatch all agents in parallel using background executions (run_in_background: true / invoke_subagent).
5. While agents run, perform orchestrator work (update heartbeat, write A2A fact envelope if enabled, plan next steps).
6. When all agents complete:
     - Run the detected validation commands (e.g. tests/linters). Must pass.
     - Git status check (inspect staged changes).
     - Git commit staged changes with a batched Conventional Commit message listing the deliverables.
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
