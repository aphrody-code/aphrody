<!-- SPDX-License-Identifier: Apache-2.0 -->
# Contributing to aphrody

Welcome. The fast version: open an issue first if the change is non-trivial,
otherwise send a PR against `main` with the boxes below checked.

## Pre-flight (5 minutes)

```bash
git clone https://github.com/aphrody-code/aphrody && cd aphrody
cargo ci-offline           # = clippy --workspace --all-targets --locked --offline -- -D warnings
cargo nextest run --workspace --locked   # 387/387 baseline, 2.9 s on this hardware
cargo deny check
cargo machete --with-metadata            # 0 dead deps expected
```

If anything in that block fails on `main` before you've touched a file,
that's a bug — open an issue rather than working around it.

## What gets merged

PRs that move `docs/PLAN.md` forward are the easiest review. Pick a `⏳`
row, do it end-to-end (code + test + PLAN line flipped to `✅` in the same
commit), open the PR. The PR template walks you through the rest.

PRs that don't map to PLAN are still welcome — explain the why in the
description and we'll either accept or add it to PLAN as a tracked item.

## Hard rules

- **`cargo ci-offline` must be green** on the PR before review. CI runs the
  same gate; it will catch you if you didn't.
- **No dead deps**: `cargo machete --with-metadata` must stay clean. If
  your code uses a dep that machete can't see (proc-macro re-export,
  cfg-gated), add it to `[package.metadata.cargo-machete] ignored = [...]`
  with a one-line comment.
- **Apache-2.0 SPDX header** on every new `.rs` file (`// SPDX-License-Identifier: Apache-2.0`).
- **Conventional Commits** — `feat:`, `fix:`, `build:`, `ci:`, `docs:`,
  `chore:`, `refactor:`, `test:`. The subject line stays under 70 chars.
- **No emoji in source or commit messages** unless the user/maintainer
  explicitly asks. Plain ASCII is preferred everywhere.
- **No mock data**: real implementations only. If a function genuinely
  needs to be a stub today, it returns a typed error — never `todo!()`.
- **Anonymisation**: don't write personal usernames into tracked files.
  Use `aphrody-code` (the org handle) and `noreply@aphrody-code.dev`.
- Every PR should append a line to the `[Unreleased]` section of `CHANGELOG.md` under the relevant category.

## What's out of scope (PRs will be closed)

- Buying / brigading GitHub stars or running star bots. Project-killer.
- AI co-author credits in commit trailers (`Co-Authored-By: Claude` etc.).
  We don't ship those.
- Vendoring crates back in (`vendor/crates.io/`). Lockfile-only since
  2026-05-16.
- Re-introducing `crates/google_os` or `crates/bun_ffi`. They're archived
  for documented reasons (`CLAUDE.md` §4).

## Cross-platform priority

The cibles, in strict order:

1. **Linux Ubuntu 26.04** — if it doesn't compile on Linux, it doesn't merge.
2. **Windows 11 Insider Canary** — full feature parity.
3. **WebAssembly** (`wasm32-unknown-unknown` + `wasm32-wasip1`) — every
   workspace member that *can* compile to wasm should compile to wasm.
   Things that genuinely can't (`backend`: needs `tokio::fs` + raw DNS)
   are documented in `docs/PLAN.md` Phase P-Wasm.

macOS and Android are best-effort and never block a merge.

## A2A coordination (if you're another AI agent)

Aphrody publishes an `ai.json` manifest at the repo root (AGNTCY a2a/v0.4
CollaborationManifest) plus a thin discovery subset at `.well-known/ai.json`.
If you're a peer agent and want to coordinate, the file-based mailbox
lives at `C:/winclean/.coord/` on the canonical dev machine; the
schema's at [`schemas/ai.json/v1.json`](schemas/ai.json/v1.json) and a
written-up walkthrough at [`docs/posts/2026-05-ai-json.md`](docs/posts/2026-05-ai-json.md).

## Getting help

- Bug? Use the **Bug report** issue template.
- Question? Use the **Question** template or open a Discussion.
- Security? See [`SECURITY.md`](SECURITY.md) — please don't open public
  issues for vulnerabilities.

## License

By submitting a PR you agree your contribution will be licensed under
Apache-2.0 (the project license). No CLA, no copyright assignment.
