# AGENTS.md — Agent-facing onboarding for AI assistants

Aphrody is built by agents, for agents. This document is the agent-facing
counterpart to `CLAUDE.md`. Both are required reading; neither supersedes the
other.

## 1. For AI agents working in this repo

Read in order:

1. `CLAUDE.md` — operator-facing spec: platform targets, language policy,
   validation commands, architecture, supply-chain rules, known pitfalls.
2. `AGENTS.md` (this file) — agent conventions, coordination protocol,
   grind loop patterns, and stop conditions.

`CLAUDE.md` answers *what* the project requires. `AGENTS.md` answers *how*
an AI assistant should behave inside it.

## 2. Conventions agents must follow

- **No AI co-author trailers in commits.** Never append `Co-Authored-By:
  Claude` or any AI fingerprint. Use `aphrody-code` + `noreply@aphrody-code.dev`
  when a git identity is needed.
- **No personal name leaks.** The strings `yohan`, `pierre`, and `yoyo` must
  never appear anywhere in source, docs, commits, or issues.
- **Apache-2.0 SPDX header on every new source file** (Rust, TypeScript, shell)
  where the format supports comments. Root-level markdown files (like this one)
  are exempt.
- **Conventional Commits** for all commit messages:
  `feat:`, `fix:`, `refactor:`, `docs:`, `chore:`, `build:`, `test:`, etc.
  No free-form messages.
- **No emoji** in source code, commit messages, or docs unless the user
  explicitly requests one for a specific output.
- **`publish = false`** on every new crate by default. Only flip it after a
  publish ladder review; see `docs/cargo/PUBLISH-LADDER.md`.

## 3. The honest-delivery extension

Every commit produced by an autonomous loop must classify each deliverable per
`docs/extensions/honest-delivery-v1.md`:

- **FAIT** — shipped with a verifiable artifact (file path, test name, etc.).
- **INCOMPLET** — partial; name exactly what is missing and why.
- **NON_FAIT** — blocked; name the blocker.

"Shipped 4 features" without a per-item breakdown is not acceptable. Reviewers
and peer agents rely on this classification to route follow-up work.

## 4. Production-readiness gates

Before classifying any deliverable FAIT, all of the following must exit 0:

- `cargo check --workspace --all-targets --locked --offline`
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`
- `cargo deny check`

For new test coverage: `cargo nextest run` must pass for the affected crate.
For new documentation: every cross-link in the added text must resolve to an
existing file in the repository.

## 5. A2A coordination (if a peer agent exists)

The A2A coordination protocol is specified in `docs/PROTOCOL.md`. When
operating alongside a peer agent (typically in `C:/winclean/`):

- Bump `C:/winclean/.coord/heartbeat-<self>.txt` at least every 10 minutes
  during an active session using an ISO-8601 timestamp.
- Drop a fact envelope in `C:/winclean/.coord/inbox-from-<self>.jsonl` for
  every significant action (file family claimed, crate added, schema changed).
- Read `C:/winclean/.coord/inbox-from-winclean.jsonl` before any edit that
  touches shared workspace files (`Cargo.toml`, `Cargo.lock`, `ai.json`).
- Each agent writes only its own inbox file and heartbeat file. Never write to
  files allocated to a peer agent.

## 6. Parallel grind (YOLO loop)

The canonical parallel execution pattern is documented in
`.claude/skills/aphrody-yolo-grind/SKILL.md`. Key rules:

- File-path-family ownership: no two agents touch the same file family (same
  crate directory, same doc section) within a single tick.
- Sub-agents stage changes only; the orchestrator agent runs `git commit` in
  batch after reviewing all staged diffs.
- One tick = one `cargo check` cycle. Do not commit if check fails.

For sequential single-agent autonomous mode, see `.claude/skills/start/SKILL.md`.

## 7. Tools — what to use and when

- **Bash with `run_in_background: true`** for any command that may take more
  than a few seconds, or when running multiple independent commands.
- **Read, Edit, Write, Grep, Glob** directly for file operations. Do not shell
  out to `grep`, `find`, `cat`, `head`, `tail`, or `sed` — the dedicated tools
  have correct permissions and produce better output.
- **`mcp__context7`** (`resolve-library-id` then `get-library-docs`) for any
  library API lookup, version check, or migration guide. Do not guess library
  interfaces from training data.
- **`cargo check`** after any Rust modification to refresh rust-analyzer cache.
  Stale LSP diagnostics report errors that no longer exist; `cargo check` is
  the source of truth (per `feedback_refresh_rust_lsp`).

## 8. Common pitfalls

See `docs/TROUBLESHOOTING.md` for the full list. High-priority items:

- Rust `edition = "2024"` requires the nightly pin in `rust-toolchain.toml`.
  Verify the pin before adding any edition-2024-only syntax.
- `mrx scan` writes `path.json` and `monorepo-map.json` to the current working
  directory. Both are gitignored at root; do not commit them accidentally.
- `a2a-pb` protobuf codegen only runs when `A2A_PB_REGEN=1` is set.
  Crates.io rejects build scripts that write outside `$OUT_DIR`; the generated
  files under `src/gen/` are the authoritative committed source.
- The `tokio` full feature set does not compile for `wasm32-unknown-unknown`.
  Use feature-selected imports (`tokio-stream`, `wasm-bindgen-futures`,
  `js-sys`) for any code that must reach the WASM target.
- `rustls 0.23` panics at startup with "No provider set" unless
  `rustls::crypto::ring::default_provider().install_default()` is called before
  the first `reqwest::Client::new()`.

## 9. Skills and agents to leverage

| Identifier | Purpose |
|---|---|
| `aphrody-yolo-grind` | 4-agent parallel grind tick |
| `start` | Sequential single-agent autonomous mode |
| `rust-engineer` | Deep Rust systems work, FFI, cross-platform |
| `cargo-auditor` | Supply-chain auditing (`deny`, `vet`, `machete`) |
| `general-purpose` | Unclassified tasks, docs, scripting |

Skills are documented in `docs/cargo/SKILLS.md`. Invoke a skill by reading
its `SKILL.md` before starting work in that mode.

## 10. When to stop

An agent must surface to the user and stop execution when:

- The next required action is destructive or requires external authorization:
  force-push to a protected branch, `cargo publish`, posting to a public
  channel, or any action touching credentials.
- Three consecutive ticks produce zero FAIT deliverables. Assume a systemic
  blocker, post a fact envelope to the peer inbox, and end the loop.
- Two or more agents have staged conflicting edits to the same file. Do not
  attempt an automated merge. Open a tracking issue and surface the conflict.

Stopping cleanly is a production outcome, not a failure.

## 11. Mission alignment

Project goal: 100k GitHub stars within 30 days (see `docs/ROADMAP.md`).

Every agent action must move that needle. The measure is the 30-second test: a
visiting engineer skims the repository and judges whether the project is
credible, well-maintained, and solves a real problem. Hygiene work (lint fixes,
doc corrections, dependency audits) counts only when it visibly improves that
first impression or unblocks a user-facing feature.

Cosmetic changes that do not affect correctness, performance, or documentation
clarity should be deferred unless they are a side-effect of a FAIT deliverable.
