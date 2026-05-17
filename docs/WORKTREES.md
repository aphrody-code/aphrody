<!-- SPDX-License-Identifier: Apache-2.0 -->

# External worktrees — what aphrody references but does NOT vendor

Aphrody is a deliberately light repo. Several features depend on upstream
projects (browser engines, design corpora, voice models, sibling agent
runtimes) that we read from disk at runtime via absolute paths under a
single root: `C:/worktree/` on Windows, `~/worktree/` elsewhere. We do
NOT vendor them — a single `bun run scripts/setup-worktrees.ts` clones
the lot.

## Why this layout

- **Repo stays light** — vendoring all 13 upstreams would push the repo
  past 1 GB. The aphrody-side of the workspace caps at ~ 60 MB.
- **Upstreams stay fresh** — `git pull` per worktree is a one-liner;
  vendored snapshots rot quickly.
- **Single source of truth** — both this doc and
  `scripts/setup-worktrees.ts` enumerate the same list, so adding a new
  worktree is a single PR.
- **CI verifiable** — `bun run scripts/check-worktrees.ts` walks every
  expected path and exits non-zero with a printable fix command if any
  is missing.

## Bootstrap (one-liner)

```bash
bun run scripts/setup-worktrees.ts
```

Flags:

| Flag | Purpose |
|---|---|
| `--root=<path>` | Override worktree root (default `C:/worktree`, fallback `~/worktree`) |
| `--only=<csv>` | Restrict to a slug allowlist (e.g. `--only=bxc,open-design`) |
| `--update`     | `git pull --ff-only` every existing clone |
| `--dry-run`    | Print plan without cloning |
| `--json`       | Machine-readable plan + result |

Manifest cache lands in `var/data/worktrees-manifest.json` (gitignored).

## Catalogue (15 worktrees)

| Slug                  | Upstream                                              | Shallow | Approx | Consumers                                       |
|---|---|---|---|---|
| `bxc`                 | aphrody-code/bxc @ aphrody                            | no      | 60 MB  | `scripts/bxc-mass-scrape.ts`, `scripts/scrape-m3-tokens.ts` |
| `n2b`                 | aphrody-code/n2b @ aphrody                            | no      | 40 MB  | `Cargo.toml` workspace.dependencies              |
| `open-design`         | nexu-io/open-design                                   | no      | 310 MB | `packages/aphrody-skills/src/sources.ts`, `scripts/design-{systems,templates}-import.ts`, `scripts/skills-harvest-open-design.ts`, `scripts/skill-schema-align.ts` |
| `openclaw`            | openclaw/openclaw                                     | no      | 240 MB | `packages/aphrody-skills/src/sources.ts`, `packages/plugin-package-contract/`, `scripts/openclaw-extensions-audit.ts` |
| `gemini-cli`          | google-gemini/gemini-cli                              | yes     | 80 MB  | `packages/aphrody-skills/src/sources.ts`, `packages/gemini-live-aphrody/src/auth.ts`, `crates/gemini-runtime/` (reference) |
| `components`          | angular/components                                    | yes     | 40 MB  | `crates/m3-tokens/src/*` (M3 SCSS reference), `docs/audits/2026-05-17-angular-material-scrape.md` |
| `whisper`             | openai/whisper                                        | yes     | 30 MB  | `crates/aphrody-voice-stt/src/local_whisper.rs` (reference) |
| `live-api-web-console`| google-gemini/live-api-web-console                    | yes     | 20 MB  | `packages/gemini-live-aphrody/` (forked from upstream) |
| `design.md`           | google-labs-code/design.md                            | yes     | 15 MB  | `DESIGN.md` spec gate (`bun x @google/design.md lint`) |
| `agent-browser`       | vercel-labs/agent-browser                             | yes     | 30 MB  | `docs/audits/2026-05-17-vercel-agent-browser-vs-bxc.md` |
| `vercel-agent-skills` | vercel-labs/agent-skills                              | yes     | 10 MB  | `packages/aphrody-skills/src/sources.ts` |
| `vercel-skills`       | vercel-labs/skills                                    | yes     | 10 MB  | `packages/aphrody-skills/src/sources.ts` |
| `open-agents`         | vercel-labs/open-agents                               | yes     | 50 MB  | `packages/aphrody-skills/src/sources.ts` |
| `wterm`               | vercel-labs/wterm                                     | yes     | 40 MB  | `crates/aphrody-terminal-vt/` (architecture ref), `crates/aphrody-terminal-wasm/` (API mirror), `crates/aphrody-terminal-backend/` (WS transport ref) |
| `terminal`            | microsoft/terminal                                    | yes     | 120 MB | `crates/aphrody-terminal-*/` (Buffer/Renderer/AtlasEngine/ConPTY/profiles.schema.json algorithmic reference — Windows-Terminal-class UX in WASM+M3) |

**Total disk budget after bootstrap: ~ 1095 MB.**

## Verify (CI gate)

```bash
bun run scripts/check-worktrees.ts
```

Output (example, all present):

```
[check-worktrees] root=C:/worktree  total=13  present=13  missing=0
  ok     bxc                    C:/worktree/bxc
  ok     n2b                    C:/worktree/n2b
  ...
```

Exit code 0 = all present. Exit code 1 = at least one missing, with a
printed `--only=...` fix command:

```
[check-worktrees] root=C:/worktree  total=13  present=11  missing=2
  MISS   whisper                C:/worktree/whisper
  MISS   agent-browser          C:/worktree/agent-browser

Fix: bun run scripts/setup-worktrees.ts --only=whisper,agent-browser
```

## What we explicitly do NOT vendor (and why)

- **`C:/winclean/`** — peer A2A Claude's workspace. Architectural
  separation by design (cf. `CLAUDE.md` § 6.1 + `ai.json`). Aphrody
  reads its `.coord/*.jsonl` mailbox at runtime but never includes
  winclean source in this repo.
- **Heavy game RE corpora** (IEVR Steam install, `var/data/bxc-cache/`,
  `var/data/edge-cache/`) — gitignored caches, regenerated on demand
  by `scripts/{bxc,edge}-mass-scrape.ts`.
- **Per-crate `target/` directories** — workspace owns the root
  `target/`. Stripped by the uniformity sweep.

## Adding a new worktree

1. Add a `WorktreeSpec` entry to `WORKTREES` in
   `scripts/setup-worktrees.ts`.
2. Add a corresponding `ExpectedWorktree` entry to `EXPECTED` in
   `scripts/check-worktrees.ts`.
3. Add a row to the catalogue table above.
4. (Optional) Wire the new clone path into a consumer (`crates/*`,
   `scripts/*`, `packages/*`).
5. Run `bun run scripts/setup-worktrees.ts --only=<new-slug>` to
   verify the clone command works.

## Graceful degradation

Every aphrody script that reads a worktree path is required to:

- Either succeed (path exists), or
- Print a clear "missing worktree" error pointing at the bootstrap
  one-liner above.

The `packages/aphrody-skills` aggregator already does this — it skips
missing sources silently and reports them via `skills sources`. Other
scripts (the scrape orchestrators, the design-systems importer) error
out loudly with the bootstrap hint.

## Related docs

- `CLAUDE.md` § 6.1 (A2A coordination, `C:/winclean`)
- `DESIGN.md` (spec source: `C:/worktree/design.md`)
- `packages/aphrody-skills/README.md` (consumes 8 worktrees)
- `docs/audits/2026-05-17-open-design-openclaw-harvest.md`
- `docs/audits/2026-05-17-voice-gemini-whisper-explore.md`
- `docs/audits/2026-05-17-vercel-agent-browser-vs-bxc.md`
- `docs/audits/2026-05-17-angular-material-scrape.md`
