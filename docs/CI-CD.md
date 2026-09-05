<!-- SPDX-License-Identifier: Apache-2.0 -->

# CI / CD overview

Map of every workflow under `.github/workflows/`.

## 1. Workflow inventory

### `bench.yml` — `bench`
- Triggers: push, PR, workflow_dispatch (path-filtered to
  `crates/**/*.rs`, `crates/**/Cargo.toml`, `crates/**/benches/**`,
  `Cargo.lock`). Branches: `main`, `stable`. Jobs: 1.
- Purpose: `cargo bench --workspace` on Linux, criterion summary as PR
  artifact.
- Failure impact: warning only.

### `build.yml` — `Build Google-OS`
- Triggers: push, PR. Branches: push on `master`, `stable`, `dev`,
  `windows`, `linux`; PR on `master`, `stable`. Jobs: 2 (`build-rust`,
  `lint-bun`).
- Purpose: legacy Windows-only gate (clippy + fmt + nextest +
  cargo-deny) plus bun lint. Predates `cross-platform`.
- Failure impact: blocks merge on listed branches.

### `codeql.yml` — `codeql`
- Triggers: push, PR, schedule (`0 6 * * 1`). Branches: `main`,
  `stable`. Jobs: 1 (matrix `rust` + `javascript-typescript`).
- Purpose: GitHub CodeQL static analysis.
- Failure impact: blocks merge.

### `coverage.yml` — `coverage`
- Triggers: push, PR. Branches: `master`, `stable`. Jobs: 1
  (`llvm-cov`).
- Purpose: `cargo llvm-cov` on Linux, uploads `lcov.info` to Codecov
  (`fail_ci_if_error: false`).
- Failure impact: blocks merge.

### `cross-platform.yml` — `cross-platform`
- Triggers: push, PR. Branches: push on `main`, `stable`, `dev`; PR on
  `main`, `stable`. Jobs: 10 (`lint`, `linux-priority`, `linux-native`,
  `macos-native`, `windows-priority`, `wasm-priority`,
  `cross-extended`, `supply-chain`, `android`, `docs`).
- Purpose: main matrix — Linux (cible #1), Windows (cible #2), WASM
  (cible #3), best-effort macOS / Android, plus supply-chain gate.
- Failure impact: blocks merge except `macos-native`, `cross-extended`,
  `android` (all `continue-on-error`).

### `dependabot-auto-merge.yml` — `dependabot-auto-merge`
- Triggers: `pull_request_target`, gated to
  `github.actor == 'dependabot[bot]'`. Jobs: 1.
- Purpose: auto-merges semver-patch + minor dev-dep updates not on the
  deny-list (`.github/dependabot.yml`, CLAUDE.md §7).
- Failure impact: none — PR stays open for manual review.

### `docs.yml` — `docs`
- Triggers: push, PR, workflow_dispatch. Branches: `main`, `stable`.
  Jobs: 2 (`build-docs`, `deploy`).
- Purpose: builds rustdoc + mdBook, deploys to GitHub Pages.
- Failure impact: `build-docs` blocks merge; `deploy` is push-to-main
  only.

### `release-please.yml` — `release-please`
- Triggers: push. Branches: `main`. Jobs: 1.
- Purpose: parses Conventional Commits, opens/updates a release PR with
  the next semver bump + changelog.
- Failure impact: no merge gate.

### `release.yml` — `release`
- Triggers: push of tags `v*.*.*`, workflow_dispatch (tag input). Jobs:
  3 (`build-gemini`, `build` matrix, `publish`).
- Purpose: cross-compiles release binaries, uploads as GitHub Release
  artifacts.
- Failure impact: not a PR gate.

### `security.yml` — `security`
- Triggers: push, PR, schedule (`0 7 * * 1`), workflow_dispatch.
  Branches: `main`, `stable`. Jobs: 4 (`gitleaks`, `trufflehog`,
  `cargo-audit`, `trivy`).
- Purpose: third security layer (after CodeQL + `cargo-deny`) — secret +
  vulnerability scanning.
- Failure impact: `gitleaks` + `trufflehog` block merge; `cargo-audit` +
  `trivy` are `continue-on-error`.

## 2. Trigger matrix

| Workflow | push main | push stable | PR main | PR stable | schedule | manual |
|---|---|---|---|---|---|---|
| bench | path | path | path | path | no | yes |
| build | no (master) | yes | no (master) | yes | no | no |
| codeql | yes | yes | yes | yes | Mon 06:00 | no |
| coverage | no (master) | yes | no (master) | yes | no | no |
| cross-platform | yes | yes | yes | yes | no | no |
| dependabot-auto-merge | no | no | yes (bot) | yes (bot) | no | no |
| docs | yes | yes | yes | yes | no | yes |
| release | tag-only | tag-only | no | no | no | yes |
| release-please | yes | no | no | no | no | no |
| security | yes | yes | yes | yes | Mon 07:00 | yes |

`build.yml` + `coverage.yml` target the legacy `master` branch.
`cross-platform.yml` is the authoritative gate on `main` / `stable`.

## 3. Pinned action SHAs

`dtolnay/rust-toolchain` is pinned to
`5b842231ba77f5c045dba54ac5560fed2db780e2` across `codeql.yml`,
`coverage.yml`, `cross-platform.yml`, `docs.yml`, `security.yml`. Re-pin
via `gh api repos/dtolnay/rust-toolchain/branches/nightly --jq
.commit.sha`. `build.yml` still uses `@nightly` (deprecation path).
Other actions pin to majors (`@v3`, `@v4`, `@v2`); one exception:
`aquasecurity/trivy-action@master` in `security.yml`. Rationale:
`docs/cargo/SECURITY-DEEP.md` §6.

## 4. Required CI gates

Must pass before main-branch merge:
- `lint` (cross-platform.yml).
- `linux-priority` + `linux-native` (cible #1).
- `windows-priority` (cible #2). `wasm-priority` (cible #3).
- `supply-chain` (`cargo-deny` + `cargo-vet` + `cargo-machete`).
- `coverage` (`llvm-cov`).
- `docs` (rustdoc + in-matrix SUMMARY-drift job).
- `codeql` (both matrix legs).
- `security` — only `gitleaks` + `trufflehog` block; `cargo-audit` +
  `trivy` are `continue-on-error`.

Non-blocking lanes (`continue-on-error: true`): `macos-native`,
`cross-extended`, `android`. macOS is best-effort per CLAUDE.md.

## 5. Concurrency + caching

All workflows declare `concurrency: { group: <workflow>-<ref>,
cancel-in-progress: true }` except `release.yml` (artifacts must not
cancel mid-upload) and `release-please.yml` (singleton group,
`cancel-in-progress: false`).

Rust jobs share `Swatinem/rust-cache@v2`; each sets a `shared-key`
(`linux-native`, `windows-priority`, `macos-native`, `bench`, …) so
caches are platform-namespaced. Bun-aware jobs layer `actions/cache@v4`
on `~/.bun/install/cache`.

## 6. Failure debugging

- Re-run via GitHub UI: Actions tab, pick the run, "Re-run failed jobs".
- Local repro: `docs/TROUBLESHOOTING.md`.
- Dependency failures: open Dependabot PRs may hit pins in
  `.github/dependabot.yml` + CLAUDE.md §7 (tracing-subscriber, rand,
  flume, notify-debouncer-full, rusqlite, schemars, toml, foldhash,
  zod).

## 7. Adding a new workflow

- Path: `.github/workflows/<name>.yml`.
- Header: `# SPDX-License-Identifier: Apache-2.0`.
- Pin every action by SHA (or by major for `@v*` per policy); document
  exceptions inline.
- Declare a concurrency group.
- Update `.github/dependabot.yml` if introducing new action ecosystems.
- Update THIS doc (section 1 + section 2 matrix).

## 8. Related docs

- `docs/cargo/SECURITY-DEEP.md` — supply-chain CI deep dive.
- `docs/cargo/PUBLISH-LADDER.md` — publish workflow.
- `docs/TROUBLESHOOTING.md` — CI failure recipes.
