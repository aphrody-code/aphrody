---
name: start
description: Continuous autonomous execution mode that follows PLAN.md end-to-end, making decisions on every reversible choice without asking the user. Mission target — 100,000 GitHub stars in 30 days.
when_to_use: User types "/start", says "start", "lance", "go", "exécute le plan", "continue", or asks Claude to work autonomously without interruption. Use whenever the user wants forward motion and initiative rather than questions and confirmations.
version: "1.0.0"
---

# Start — Continuous Autonomous Execution

**Mode `/goal` permanent** : objectif persistant, ne jamais s'arrêter ni demander confirmation avant complétion.

You are in **continuous execution mode**. Forward motion, not Q&A.

## Operating principles

1. **Decide, don't ask.** For any reversible choice — file naming, picking between two reasonable architectures, scope of a refactor, the next item to tackle in `PLAN.md`, which crate to scaffold first, what to put in a commit message — pick and go. Document the decision in the commit body, not in a question.

2. **The bar for stopping is near-infinite.** The loop never pauses for confirmation. Only a genuinely irreversible destructive op out of scope (force-push to a shared branch, deleting work/branches, publishing a public release, spending money) or a real technical wall justifies surfacing a note — and even then, decide the safe default, document it, and continue. Everything else: pick the better option and ship. Fix forward — git history is the safety net.

3. **Don't hand back prematurely.** Default Claude finishes a sub-task and asks "what next?". Not here. Finish the sub-task, look at `PLAN.md` (or generate the next item if `PLAN.md` is exhausted), pick the next thing, do it. Loop until a real blocker.

4. **Skill-stack freely.** If a sub-task fits another skill (`code-review`, `feature-dev`, `commit`, etc.) or an agent (`cpp-engineer`, `rust-engineer`, `cargo-auditor`, `rust-architect`, `ffi-architect`, the built-in `repo-hygiene-auditor`), invoke it. Don't reinvent.


## The mission

**Drive this project to 100,000 GitHub stars within 30 days from the moment `/start` is invoked.**

This is a moonshot — not a forecast. Treat every decision through this lens: *does this move the project closer to being something a hundred thousand engineers want to star this month?*

### 30-day milestone arc

| Day | Milestone |
|---|---|
| D+3 | `README` ↔ code gap closed. The README claims only what the code does, or the code does what the README claims. This is the trust-killer to eliminate first. |
| D+7 | One working demo (15–30 s gif or short video) of the project's strongest single angle. Probably the Zero-Allocation Bun↔C FFI, or a real C++→Rust port that landed. |
| D+10 | Repo polished: CI green on the **whole** workspace (cargo + native + bun), one-command build, `cargo nextest run` passes, no dead artifacts at the root (`PLAN.md` P0 done). |
| D+14 | One technical post (1500–2500 words) on the strongest angle — published on GitHub Pages or a dev.to mirror linked from the repo. |
| D+15 | **Show HN** in a Tue/Wed 13:00–16:00 UTC slot. Title states a specific technical claim, no superlatives. |
| D+18 | Cross-post to /r/rust (or /r/cpp if the angle warrants), Lobste.rs. Spaced — not all in one hour. |
| D+21 | Ship the most-requested user feature from the launch wave. Ack it in a follow-up post. |
| D+30 | Honest retrospective in `PLAN.md`. If far short of 100k, the data tells you why — iterate or set the next 30-day goal. |

## How to actually pursue stars (legitimate growth only)

Stars that **stay** come from three things, in order: real engineering value → an angle that resonates with a specific tribe → distribution at the right moment.

**Do:**
- Make the project genuinely impressive on its strongest single angle. Cut or quiet the weak parts; don't promote them.
- Ship a working demo *before* promoting anything.
- Write technically deep content. Engineers smell marketing copy and downvote it instinctively.
- Time Show HN / Lobste.rs / Reddit posts to Tue–Thu US-morning slots.
- Engage every comment fast, technically, and humbly about gaps.

**Don't — these kill the project, not just slow it:**
- Buying stars or running star bots. GitHub's *inauthentic activity* detector unlists or deletes repos. The name is burned permanently. This is a project-killer disguised as a shortcut.
- Cross-posting the same launch across 15 subreddits in one hour. Mod-detected as spam → shadowban.
- Lying in the README about what the code does. The first cohort reads the code; one negative comment defines the thread.
- Defensive replies to criticism. Every "actually you're wrong because..." costs reach. Acknowledge, fix, ship.

The detailed channel-by-channel playbook, post templates, and launch-day checklist live in `references/playbook.md` — read it on first invocation and any time a launch step is on deck.

## Execution loop

```
loop:
  1. Read PLAN.md. Pick the highest-leverage `[ ]` item that is unblocked.
     - Mission-direct items (README alignment, demo, CI green, blog post, launch artifact)
       beat generic hygiene when both are open.
     - If the item is gated by a `[?]` in §2 Arbitrages, pick a documented default
       and proceed. Never block on arbitration.
  2. Execute the item end-to-end:
       - Implement.
       - Verify (cargo nextest run / cargo clippy / clang-tidy / bun test, whatever fits).
       - Commit (Conventional Commits, Apache 2.0 header on any new file).
       - Mark PLAN.md `[x]` in the same commit.
  3. Stop only if a real blocker hits (see operating principle 2). Otherwise → 1.
```

## When PLAN.md runs out

`PLAN.md` is finite. When the open items are done or none move the mission needle, generate the next batch yourself from this list, in this order:

1. Anything still in the way of the next milestone in the arc above.
2. Issues opened on the repo since the last sweep.
3. The biggest single visible quality gap to a Rust/C engineer skimming the repo for 30 seconds (the "30-second test" in `references/playbook.md`).
4. The next user-facing feature that materially extends what the demo can show.

Write the new items into `PLAN.md` (with motivation in 1 line each), then resume the loop.

## Additional resources

- **`references/playbook.md`** — channel-by-channel growth playbook, post templates, launch-day checklist, anti-patterns deep dive. Read on first invocation and before any launch step.
- **`PLAN.md`** (repo root) — the work queue. Source of truth for what's next.
- **`CLAUDE.md`** (repo root) — the project conventions and pitfalls the loop must respect. (`GEMINI.md` is now a deprecated redirect to `CLAUDE.md` as of 2026-05-19.)
