---
name: bun-test-runner-pattern
description: "Custom scripts/test-all.ts bun test runner in rpbey + shenron that covers EVERY workspace scope; the non-obvious gotchas it solves (turbo skips script-less scopes, mock.module per-file isolation, .next double-run, vendored fork classification)."
metadata:
  node_type: memory
  type: reference
  originSessionId: e87d3ad8-df91-4692-835f-a6350089539d
---

**`scripts/test-all.ts`** is a bespoke Bun test runner added to **rpbey**
(`/home/ubuntu/rpbey`, commit `d798f03`) and **shenron**
(`/home/ubuntu/shenron`, commit `6dbbc51`) on 2026-06-04. It covers **every**
workspace scope; run via `bun run test:all` / `test:ci` (--strict) /
`test:cov` (lcov+junit) / `test:flake` (--randomize --rerun-each=3) /
`test:vendored` (rpbey) / `test:live` (shenron). CI in both now runs `test:ci`.

**Why it exists — `turbo run test` silently skips scopes without a `test`
script.** In rpbey that hid `gacha-server` + `dashboard` (they *have* test
files, no script). The runner enumerates members from the `workspaces` globs,
git-discovers test files (tracked + untracked-non-ignored), assigns each to its
deepest owning member, runs `bun test` per scope **in its own cwd** (so each
scope's `bunfig [test].preload` applies), classifies special scopes explicitly
(no silent skips), prints a scope matrix, and fails on any unexplained gap.

**Non-obvious gotchas it encodes (verified this session):**
- **Pass explicit absolute file paths to `bun test`, not a dir.** Bun's own
  recursion does NOT prune `.next`/`dist`, so it double-runs build-output copies
  (rpbey's `.next/standalone/.../utils.test.ts`). Git discovery prunes via
  `.gitignore`. (Bun treats args starting with `/` as paths, not name-filters.)
- **Per-file process isolation for fixture-coupled scopes.** rpbey
  `@rose-griffon/bot` uses process-global `mock.module` + top-level-await dynamic
  imports → all 6 files in one `bun test` process leak mocks across files
  (order-dependent FAIL; passes file-by-file). shenron `@rpbey/discordy`
  decorator state is the same class. Policy `perFile: true` runs each file in its
  own process. `--randomize`/`--rerun-each` surface this (they made discordy fail).
- **Per-package bunfig preload does NOT merge with root** — must run from the
  scope's cwd (rpbey: bot=reflect-metadata, dashboard=happy-dom; shenron bot=
  reflect-metadata + canvas shim + fresh `./data/test.db`).
- **Vendored discordx fork** (re-scoped `@rpbey/*`) is classified `vendored`
  (skip unless `--vendored`); the broken leaf `@rpbey/discordx` (missing self-dep
  `discordx` → `Cannot find package`) is `skip`-with-reason. rpbey's nested
  `packages/discordx/packages/*` = 13 such members. `@rpbey/api-client` (generated
  SDK) is `skip` (covered by `@rpbey/api-contract`).
- **Live tier is opt-in.** shenron `apps/site` no-404 is a live prod crawler
  (dragonballfr.com) — flaky; the runner keeps it out of default/CI (`--live`
  only). This also fixed shenron CI which previously ran it via turbo.

**Scope coverage after this session (all first-party gaps filled with REAL
tests, zero stubs):** rpbey 25 scopes → 11 pass / 0 fail / 0 gaps / 14 classified
skips (+141 tests: api-contract 28, gacha-client 45, cdn 21, challonge-core 25,
db 10, embed-sidecar 10, types 2). shenron 7 scopes → 6 pass / 1 live-skip / 0
gaps (+45 tests: di/importer/internal/pagination). Bun on the VPS is
**1.4.0-canary.1** (not the 1.3.14 the manifests pin). See
[[sibling-repo-build-and-versions]].
