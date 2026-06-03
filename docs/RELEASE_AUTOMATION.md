<!-- SPDX-License-Identifier: Apache-2.0 -->
# Release Automation and Workflow Guide

This document describes the automated release process for aphrody's Material
Design 3 packages (the `@aphrody/*` family) published to the public npm
registry.

> Superseded the obsolete `aphrody-ts` extracted-repo + GitHub-Packages flow
> (`scripts/release.ts` / `scripts/publish-github-packages.ts`, both removed) on
> 2026-06-04. npm (`registry.npmjs.org`, scope `@aphrody`) is now canonical.

## Automated Release Workflow

[`.github/workflows/release-m3-packages.yml`](../.github/workflows/release-m3-packages.yml)
automates the end-to-end publish pipeline. It fires on an **`m3-v*`** tag
(decoupled from the core `release.yml`, which fires on `v*`) and can also be run
manually via `workflow_dispatch`.

### Pipeline Steps

1. **Checkout + Bun**: `actions/checkout` then `oven-sh/setup-bun` (canary).
2. **Install**: `bun install --frozen-lockfile` (`GH_PACKAGES_TOKEN` lets any
   remaining private GitHub-Packages transitive dep — e.g. the showcase's
   optional `@aphrody/bxc` — resolve at install time).
3. **Build**: `bunx turbo run build --filter='./packages/*'` — builds only the
   publishable packages (turbo pulls in their workspace deps; `examples/*` are
   excluded). `build:sass` falls back to `sass-embedded`, so no Rust toolchain
   is required in CI.
4. **npm auth**: writes `//registry.npmjs.org/:_authToken=${NPM_TOKEN}` to
   `~/.npmrc` (`NPM_TOKEN` = automation token owning the `@aphrody` org).
5. **Publish**: for each package in dependency order
   (`m3-tokens → material-web → react → m3-motion → m3-theme → m3-design →
   eslint-plugin-m3 → doc-ai → bun-rs`), runs
   `bun publish --access public --registry https://registry.npmjs.org`.
   `bun publish` inlines every `workspace:*` dependency with its resolved
   version; an already-published version is skipped, not failed.

## How to Execute a Release

```sh
# 1. Bump versions in the relevant packages/*/package.json.
# 2. Commit, then tag and push the m3-v* tag:
git tag m3-v<version>
git push github m3-v<version>
```

The tag push triggers the workflow. There is no local publish script — the
GitHub Actions workflow is the single source of truth.

## CI secrets

- **`NPM_TOKEN`** — automation token, owner of the `@aphrody` npm org.
- **`GH_PACKAGES_TOKEN`** — install-time only, for private GitHub-Packages
  transitive deps.
