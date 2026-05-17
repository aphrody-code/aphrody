<!-- SPDX-License-Identifier: Apache-2.0 -->

# CI / CD overview

Single-page map of every GitHub Actions workflow shipped in
`.github/workflows/`. Read this before opening individual `.yml` files. For
deep dives on supply-chain or release plumbing, follow the cross-links in
section 8.

## 1. Workflow inventory

Each entry is one `.yml` under `.github/workflows/`.

### `bench.yml` — `bench`
- Triggers: push, pull_request, workflow_dispatch.
- Branches: `main`, `stable` (path-filtered to `crates/**/*.rs`,
  `crates/**/Cargo.toml`, `crates/**/benches/**`, `Cargo.lock`).
- Jobs: 1 (`bench`).
- Purpose: runs `cargo bench --workspace` on Linux and uploads a criterion
  summary as a PR comment artifact.
- Failure impact: warning only — no branch-protection gate.

### `build.yml` — `Build Google-OS`
- Triggers: push, pull_request.
- Branches: push on `master`, `stable`, `dev`, `windows`, `linux`;
  pull_request on `master`, `stable`.
- Jobs: 2 (`build-rust`, `lint-bun`).
- Purpose: legacy Windows-only workspace gate (clippy + fmt + nextest +
  cargo-deny) plus bun lint. Predates the `cross-platform` workflow; kept
  active for the historical branch set.
- Failure impact: blocks merge on the listed branches.

### `codeql.yml` — `codeql`
- Triggers: push, pull_request, schedule (`0 6 * * 1`, Monday 06:00 UTC).
- Branches: `main`, `stable`.
- Jobs: 1 (`analyze`, matrix `rust` + `javascript-typescript`).
- Purpose: GitHub CodeQL static analysis for security advisories.
- Failure impact: blocks merge (required gate).

### `coverage.yml` — `coverage`
- Triggers: push, pull_request.
- Branches: `master`, `stable`.
- Jobs: 1 (`llvm-cov`).
- Purpose: `cargo llvm-cov` on Linux, uploads `lcov.info` to Codecov.
- Failure impact: blocks merge (Codecov upload itself is
  `fail_ci_if_error: false`; the `cargo llvm-cov` run is the hard gate).

### `cross-platform.yml` — `cross-platform`
- Triggers: push, pull_request.
- Branches: push on `main`, `stable`, `dev`; pull_request on `main`,
  `stable`.
- Jobs: 10 (`lint`, `linux-priority`, `linux-native`, `macos-native`,
  `windows-priority`, `wasm-priority`, `cross-extended`, `supply-chain`,
  `android`, `docs`).
- Purpose: the main matrix — Linux (cible #1), Windows (cible #2),
  WebAssembly (cible #3) plus macOS / Android best-effort lanes and a
  supply-chain (`cargo-deny` + `cargo-vet` + `cargo-machete`) gate.
- Failure impact: blocks merge except `macos-native`, `cross-extended`,
  and `android` which are `continue-on-error: true`.

### `dependabot-auto-merge.yml` — `dependabot-auto-merge`
- Triggers: `pull_request_target` (opened, synchronize, reopened); gated
  to `github.actor == 'dependabot[bot]'`.
- Branches: all PR targets.
- Jobs: 1 (`dependabot-merge`).
- Purpose: auto-merges semver-patch updates and minor dev-dep updates that
  are not on the deny-list (see `.github/dependabot.yml` and CLAUDE.md §7).
- Failure impact: no merge gate — failures simply mean the PR stays open
  for manual review.

### `docs.yml` — `docs`
- Triggers: push, pull_request, workflow_dispatch.
- Branches: `main`, `stable`.
- Jobs: 2 (`build-docs`, `deploy`).
- Purpose: builds rustdoc + mdBook and deploys to GitHub Pages.
- Failure impact: `build-docs` blocks merge (rustdoc must compile);
  `deploy` runs only on push to main and does not gate PRs.

### `release-please.yml` — `release-please`
- Triggers: push.
- Branches: `main` only.
- Jobs: 1 (`release-please`).
- Purpose: parses Conventional Commits and opens or updates a release PR
  with the next semver bump plus changelog.
- Failure impact: no merge gate; releases simply stall until fixed.

### `release.yml` — `release`
- Triggers: push of tags matching `v*.*.*`, workflow_dispatch (tag input).
- Branches: tag-only (any ref that produces a matching tag).
- Jobs: 3 (`build-gemini`, `build` (matrix), `publish`).
- Purpose: cross-compiles release binaries and uploads them as GitHub
  Release artifacts.
- Failure impact: not a PR gate; a failed release blocks the tag from
  shipping artifacts and must be re-run.

### `security.yml` — `security`
- Triggers: push, pull_request, schedule (`0 7 * * 1`, Monday 07:00 UTC),
  workflow_dispatch.
- Branches: `main`, `stable`.
- Jobs: 4 (`gitleaks`, `trufflehog`, `cargo-audit`, `trivy`).
- Purpose: third security layer (after CodeQL and `cargo-deny`) — secret
  scanning plus vulnerability scanning.
- Failure impact: `gitleaks` and `trufflehog` block merge;
  `cargo-audit` is `continue-on-error: true`; `trivy` results are uploaded
  to GitHub Security but the job does not fail the run.

## 2. Trigger matrix

| Workflow | push main | push stable | PR main | PR stable | schedule | manual |
|---|---|---|---|---|---|---|
| bench | path-filtered | path-filtered | path-filtered | path-filtered | no | yes |
| build | no (master only) | yes | no (master only) | yes | no | no |
| codeql | yes | yes | yes | yes | weekly Mon 06:00 UTC | no |
| coverage | no (master only) | yes | no (master only) | yes | no | no |
| cross-platform | yes | yes | yes | yes | no | no |
| dependabot-auto-merge | no | no | yes (Dependabot only) | yes (Dependabot only) | no | no |
| docs | yes | yes | yes | yes | no | yes |
| release | tag-only | tag-only | no | no | no | yes |
| release-please | yes | no | no | no | no | no |
| security | yes | yes | yes | yes | weekly Mon 07:00 UTC | yes |

Notes:
- `build.yml` and `coverage.yml` target the legacy `master` branch, not
  `main`. They remain useful for older long-lived branches.
- `cross-platform.yml` is the authoritative gate on `main` and `stable`.

## 3. Pinned action SHAs

`dtolnay/rust-toolchain` is pinned to
`5b842231ba77f5c045dba54ac5560fed2db780e2` across `codeql.yml`,
`coverage.yml`, `cross-platform.yml`, `docs.yml`, and `security.yml`
(re-pin via `gh api repos/dtolnay/rust-toolchain/branches/nightly --jq
.commit.sha`). `build.yml` still uses `dtolnay/rust-toolchain@nightly`
because it predates the SHA-pin policy and is on a deprecation path.
Other third-party actions are pinned to majors (`@v3`, `@v4`, `@v2`),
with one documented exception in `security.yml` for
`aquasecurity/trivy-action@master`. Rationale and policy live in
`docs/cargo/SECURITY-DEEP.md` §6.

## 4. Required CI gates (branch protection)

Workflows that MUST pass before a main-branch merge:
- `lint` (in `cross-platform.yml`).
- `linux-priority` + `linux-native` (Linux is cible #1).
- `windows-priority` (cible #2).
- `wasm-priority` (cible #3).
- `supply-chain` (`cargo-deny` + `cargo-vet` + `cargo-machete`).
- `coverage` (`llvm-cov` job).
- `docs` (rustdoc must build, plus the in-matrix `docs` job for SUMMARY
  drift).
- `codeql` (both matrix legs).
- `security` — only `gitleaks` and `trufflehog` block; `cargo-audit` and
  `trivy` are `continue-on-error`.

Non-blocking lanes (`continue-on-error: true`): `macos-native`,
`cross-extended`, `android` in `cross-platform.yml`; `cargo-audit` and
`trivy` in `security.yml`. macOS is best-effort per CLAUDE.md.

## 5. Concurrency + caching

All workflows declare `concurrency: { group: <workflow>-<ref>,
cancel-in-progress: true }` except `release.yml` (artifacts must not be
cancelled mid-upload) and `release-please.yml` (it uses a singleton
`release-please` group with `cancel-in-progress: false` to serialise
release PR updates).

Rust jobs share `Swatinem/rust-cache@v2`. Each job sets a `shared-key`
(`linux-native`, `windows-priority`, `macos-native`, `bench`, etc.) so
caches are namespaced per platform and do not poison cross-target builds.
Bun-aware jobs (`docs.yml`, `build.yml` `lint-bun`, `release.yml`
`build-gemini`) layer `actions/cache@v4` on `~/.bun/install/cache`.

## 6. Failure debugging

- Re-run a failed workflow from the GitHub UI: Actions tab, click the
  failed run, choose "Re-run failed jobs".
- Local repro for the cross-platform matrix: follow
  `docs/TROUBLESHOOTING.md`.
- For dependency-related CI failures: check the open Dependabot PRs;
  several deps are pinned in `.github/dependabot.yml` and CLAUDE.md §7
  (tracing-subscriber, rand, flume, notify-debouncer-full, rusqlite,
  schemars, toml, foldhash, zod). An auto-merge denial usually means the
  update hit one of those pins.

## 7. Adding a new workflow

- File path: `.github/workflows/<name>.yml`.
- Header: `# SPDX-License-Identifier: Apache-2.0`.
- Pin every action by SHA (or by major for `@v*` ecosystem actions, per
  policy); document any exception inline.
- Declare a concurrency group.
- Update `.github/dependabot.yml` if the workflow introduces new action
  ecosystems.
- Update THIS doc (section 1 plus the trigger matrix in section 2).

## 8. Related docs

- `docs/cargo/SECURITY-DEEP.md` — supply-chain CI deep dive.
- `docs/cargo/PUBLISH-LADDER.md` — publish workflow.
- `docs/TROUBLESHOOTING.md` — CI failure recipes.
