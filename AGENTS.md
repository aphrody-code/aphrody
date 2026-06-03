<!-- SPDX-License-Identifier: Apache-2.0 -->
# AGENTS.md — Agent-facing onboarding for AI assistants

Aphrody is built by agents, for agents. This is the agent-facing counterpart
to `CLAUDE.md`. Both are required reading; neither supersedes the other.

## 1. For AI agents working in this repo

Read in order:

1. `CLAUDE.md` — operator-facing spec: platform targets, language policy,
   validation commands, architecture, supply-chain rules, known pitfalls.
2. `DEPLOY.md` — VPS/Linux deploy: Rust CLI, `aphrody-mcp`, A2A, scripts, systemd
   (Python `:8082` is separate). Pair with `../bxc/DEPLOY.md`.
3. `docs/agent-stack/DEPLOY.md` — fast stop/clean/smoke for the shared agent host.
4. `AGENTS.md` (this file) — agent conventions, coordination protocol,
   grind loop patterns, and stop conditions.

`CLAUDE.md` answers *what* the project requires. `AGENTS.md` answers *how*
an AI assistant should behave inside it.

## 2. Conventions agents must follow

- **No AI co-author trailers in commits.** Never add `Co-Authored-By: Claude`
  or any AI fingerprint. Use `aphrody-code` + `noreply@aphrody-code.dev`.
- **No personal name leaks.** The strings `yohan`, `pierre`, and `yoyo` must
  never appear in source, docs, commits, or issues.
- **Apache-2.0 SPDX header** on every new source file where the format
  supports comments. Root-level markdown files are exempt.
- **Conventional Commits**: `feat:`, `fix:`, `refactor:`, `docs:`, `chore:`,
  `build:`, `test:`. No free-form messages.
- **No emoji** in source, commits, or docs unless the user explicitly requests.
- **`publish = false`** on every new crate. Only flip after a publish ladder
  review; see `docs/cargo/PUBLISH-LADDER.md`.
- **Zero human-in-the-loop, repo-wide.** aphrody is autonomous A→Z on every
  surface: sessions, skills, CLI, sub-agents, code, GitHub, build, install.
  Decide alone with documented rationale. Ask the operator only for truly
  destructive irreversible actions (force-push main, drop DB, deploy prod).
  See `CLAUDE.md` §0.1 for the full matrix.

## 3. The honest-delivery extension

Every commit produced by an autonomous loop must classify each deliverable per
`docs/extensions/honest-delivery-v1.md`:

- **FAIT** — shipped with a verifiable artifact.
- **INCOMPLET** — partial; name what is missing.
- **NON_FAIT** — blocked; name the blocker.

"Shipped 4 features" without a per-item breakdown is not acceptable.

## 4. Production-readiness gates

Before classifying any deliverable FAIT, all of the following must exit 0:

- `cargo check --workspace --all-targets --locked --offline`
- `cargo clippy --workspace --all-targets --locked --offline -- -D warnings`
- `cargo deny check`

For new tests: `cargo nextest run` passes for the affected crate.
For new docs: every cross-link resolves to an existing file.

## 5. A2A coordination (if a peer agent exists)

The A2A coordination protocol is specified in `docs/PROTOCOL.md`. When
operating alongside a peer agent (typically in `C:/winclean/`):

- Bump `C:/winclean/.coord/heartbeat-<self>.txt` at least every 10 minutes
  (ISO-8601 timestamp).
- Drop a fact envelope in `C:/winclean/.coord/inbox-from-<self>.jsonl` for
  every significant action (file family claimed, crate added, schema changed).
- Read `C:/winclean/.coord/inbox-from-winclean.jsonl` before any edit that
  touches shared workspace files (`Cargo.toml`, `Cargo.lock`, `ai.json`).
- Each agent writes only its own inbox and heartbeat files. Never write to
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

- **Bash with `run_in_background: true`** for any slow or independent command.
- **Read, Edit, Write, Grep, Glob** for file operations — never shell out to
  `grep`, `find`, `cat`, `head`, `tail`, or `sed`.
- **`mcp__context7`** (`resolve-library-id` then `get-library-docs`) for any
  library API or version lookup. Do not guess from training data.
- **`cargo check`** after every Rust modification. Stale LSP diagnostics
  may report errors that no longer exist; `cargo check` is the source of truth.

## 8. Common pitfalls

See `docs/TROUBLESHOOTING.md` for the full list. High-priority items:

- Rust `edition = "2024"` requires the nightly pin in `rust-toolchain.toml`.
  Verify the pin before adding any edition-2024-only syntax.
- `mrx scan` writes `path.json` and `monorepo-map.json` to cwd (gitignored);
  do not commit them.
- `a2a-pb` codegen only runs with `A2A_PB_REGEN=1`. The checked-in files
  under `src/gen/` are the authoritative source.
- `tokio` full features do not compile for `wasm32-unknown-unknown`. Use
  `tokio-stream`, `wasm-bindgen-futures`, `js-sys` for WASM targets.
- `rustls 0.23` panics with "No provider set" unless
  `rustls::crypto::ring::default_provider().install_default()` is called
  before the first `reqwest::Client::new()`.
- **Extraction `x-client` / `@aphrody-code/x`** : the Bun module `packages/x` has been extracted to a standalone repository `/home/ubuntu/x-client` (package `@aphrody-code/x`, registry `npm.pkg.github.com`). Downstream client projects (`rg`/`rpbey`) must import the package or point to `/home/ubuntu/x-client/ts/` instead of monorepo paths.
- **Imports Python / conflit de namespace** : l'exécution de `pytest` depuis le dossier `py/` résout par défaut le dossier de configuration `py/aphrody` comme un namespace package vide (avec `__file__ = None`), ce qui provoque l'échec de l'import de `__version__`. Toujours exécuter ou préfixer les tests avec la variable d'environnement `PYTHONPATH=aphrody` pour forcer la résolution vers le package source `py/aphrody/aphrody/`.

## 9. Skills and agents to leverage

| Identifier | Purpose |
|---|---|
| `aphrody-yolo-grind` | 4-agent parallel grind tick |
| `start` | Sequential single-agent autonomous mode |
| `rust-engineer` | Deep Rust systems work, FFI, cross-platform |
| `cargo-auditor` | Supply-chain auditing (`deny`, `vet`, `machete`) |
| `general-purpose` | Unclassified tasks, docs, scripting |

Skills are documented in `docs/cargo/SKILLS.md`. Read a skill's `SKILL.md`
before starting work in that mode.

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
visiting engineer skims the repo and judges whether the project is credible,
well-maintained, and solves a real problem. Hygiene work counts only when it
improves that first impression or unblocks a user-facing feature.

Cosmetic changes that do not affect correctness, performance, or documentation
clarity should be deferred unless they are a side-effect of a FAIT deliverable.
