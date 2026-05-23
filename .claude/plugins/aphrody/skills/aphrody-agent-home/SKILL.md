---
name: aphrody-agent-home
version: "1.0.0"
description: One-session autonomous build of the `aphrody-agent-home` crate (soul / identity / workspace). Dispatches the dedicated agent-home-builder subagent (Opus 4.7) to execute the ENTIRE docs/plans/agent-home.md (AH-0..AH-19) with zero stub and no human in the loop, then verifies, commits in batches, and flips the plan items to done.
when_to_use: User types "/aphrody-agent-home", says "build the agent-home crate", "exécute le plan agent-home", "soul/identity/workspace crate", or asks to run docs/plans/agent-home.md end-to-end autonomously in one session. Use whenever the goal is to land the whole aphrody-agent-home crate without handhold.
---

# aphrody-agent-home — one-session autonomous crate build

Mode `/goal` permanent : objectif persistant, zéro confirmation, ne pas
s'arrêter avant que `crates/aphrody-agent-home` soit livré, vérifié et committé.
Aucune intervention humaine (CLAUDE.md §0.1).

This skill drives the dedicated **`agent-home-builder`** subagent (model: Opus
4.7) to execute the entire plan `docs/plans/agent-home.md` (items AH-0..AH-19)
in a single session, then handles verification, batched commits, and plan
bookkeeping — the parts the builder is contractually forbidden from doing.

Paths: `$PLAN` = `docs/plans/agent-home.md`, `$CRATE` =
`crates/aphrody-agent-home`, `$COORD` = A2A mailbox dir (peer `.coord/`).

## Division of labour

| Actor | Owns |
|---|---|
| `agent-home-builder` (subagent) | ALL the code: scaffold, modules, tests, 3-target check, real-behavior smoke. Stages changes. Flips ⏳→✅ in `$PLAN`. **Never commits.** |
| This skill (orchestrator) | Dispatch + monitor the builder, run the full repo validation gate, commit in logical batches, drop A2A heartbeat, final report. |

The crate is **one cohesive unit** (shared `lib.rs` + `Cargo.toml`): dispatch
**ONE** builder, never parallel agents on it (parallel writers race the shared
files — CLAUDE.md §7 worktree gotcha). Sequential coherence wins here.

## The run, one session

```
1. Pre-flight (orchestrator):
   - Confirm $PLAN exists and lists AH-0..AH-19; read it once.
   - git status --short  -> capture clean baseline. If aphrody-agent-home
     already partly exists, note which AH-* are already ✅ and tell the builder
     to resume from the first ⏳.
   - Sync TaskList: one task per phase P0..P6.

2. Dispatch the builder (single Agent call, run_in_background: true):
     subagent_type: "agent-home-builder"  (fallback: "rust-engineer" + paste the
       agent-home-builder body as the prompt if the type is unavailable)
     prompt: "Execute docs/plans/agent-home.md in full (AH-0..AH-19), vertical
       slice first, zero stub, stage-don't-commit. Report honest-delivery per
       item." 
     model: opus   (Opus 4.7 — explicit, non-negotiable for this build)

3. While it runs, orchestrator work that does NOT touch crate files:
     - bump $COORD/heartbeat-aphrody.txt
     - drop a `fact` envelope in $COORD/inbox-from-aphrody.jsonl naming the build
     - mark P0 task in_progress

4. On builder completion, run the FULL gate from the repo root (CLAUDE.md §3):
     cargo clippy -p aphrody-agent-home --all-targets --locked -- -D warnings
     cargo nextest run -p aphrody-agent-home --locked
     cargo check -p aphrody-agent-home --target x86_64-unknown-linux-gnu --locked
     cargo check -p aphrody-agent-home --target x86_64-pc-windows-msvc   --locked
     cargo check -p aphrody-agent-home --target wasm32-unknown-unknown   --locked
     cargo check -p aphrody --locked   # CLI integration (AH-14/15/16) still builds
   ALL must pass. If any fails, re-dispatch the builder with the captured error
   (continue the same agent / fresh dispatch naming the failing target + log).

5. Commit in logical batches (only when the gate is green):
     - "feat(agent-home): scaffold crate + soul/identity/user/tools models"
       (AH-0..AH-3, AH-8)
     - "feat(agent-home): mmap cache + content-addressed store + budget assembler"
       (AH-11/12/6/5/7)
     - "feat(agent-home): onboard seeding + hot-reload + git backup + profiles"
       (AH-4/13/9/10)
     - "feat(agent-home): wire oc-onboard + chat system-prompt + doctor"
       (AH-14/15/16)
     - "test(agent-home): 3-target check + criterion bench; close plan"
       (AH-17/18/19 + $PLAN ✅ flips)
   Each commit message ends with the Co-Authored-By trailer. Group only files
   that belong together; never commit the peer's uncommitted files.

6. Final report (orchestrator): per-item FAIT/INCOMPLET/NON_FAIT table, the
   commit SHAs, the verify matrix, and the oc-onboard smoke output.
```

## Guardrails (mandatory before every commit — CLAUDE.md §0.1 / standing constraint)

1. **Build green**: the §3 gate above passed. Never commit broken code.
2. **Anti-leak**: grep the staged diff for `AIza`, `ctx7sk`, `sk-ant`,
   `private_key`, `BEGIN .* PRIVATE KEY`. Abort the commit if any match a
   tracked file.
3. **No branch switch**: commit on the current branch (`main`). Never
   `git checkout -b`. The peer agy commits on `main` with the same
   `aphrody-code` identity — re-run `git log --oneline -5` before committing to
   confirm HEAD hasn't moved under you.
4. **No emoji** in commit messages or source (plan markers ⏳/✅ stay in `$PLAN`).
5. **secrets/ and .env are gitignored** — never stage them.

## When context runs out mid-build (one session, gracefully)

The builder may approach its context limit before AH-19. If so:
- It will have flipped the shipped items to ✅ in `$PLAN` and staged them.
- The orchestrator commits the green batch, then re-dispatches a fresh
  `agent-home-builder` with: "Resume docs/plans/agent-home.md from the first ⏳
  item; the prior items are already shipped + committed."
- Repeat until `grep -c "⏳" $PLAN` for the agent-home items reaches 0.
This keeps it autonomous across the context boundary without a human.

## When to stop

- All AH-0..AH-19 are ✅ and the full gate is green -> done, final report.
- Two consecutive builder dispatches ship 0 new FAIT -> assume a hard blocker
  (offline dep, target failure); record it in `$PLAN`, drop an A2A fact
  envelope, end with the concrete blocker. Do not loop forever.
- A destructive remote op would be required -> never auto-run; report and stop.

## Files this skill touches

| Path | Direction | Role |
|---|---|---|
| `$PLAN` (`docs/plans/agent-home.md`) | read + edit | AH-* queue + ✅ flips |
| `$CRATE/**` + root `Cargo.toml` | builder writes | the crate itself |
| `crates/cli/src/oc_cmd.rs`, `crates/aphrody-chat/**`, `crates/aphrody-prompts/**` | builder writes | AH-14/15/16 wiring |
| `$COORD/heartbeat-aphrody.txt` / `inbox-from-aphrody.jsonl` | write / append | A2A proof-of-life |

## Related

- **`agent-home-builder`** agent (`.claude/plugins/aphrody/agents/`) — the
  dedicated Opus 4.7 builder this skill dispatches.
- **`docs/plans/agent-home.md`** — the executed plan.
- **`docs/research/openclaw-vs-aphrody.md`** — the gap audit that motivated it.
- **`/aphrody-yolo-grind`** — the *parallel* multi-item variant; this skill is
  *single-crate sequential* (the crate is one unit, parallelism would race it).
- **`/loop`** — optional pacing if you want periodic resume ticks; not required,
  the resume logic above already spans the context boundary.
