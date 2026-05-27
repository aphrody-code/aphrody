<!-- SPDX-License-Identifier: Apache-2.0 -->
# Publishing aphrody packages to GitHub Packages

aphrody's npm packages publish to **GitHub Packages** (registry
`npm.pkg.github.com`, org `aphrody-code`). Scope is wired in the root
[`.npmrc`](.npmrc); a single script handles build + scope-rewrite + publish.

## Publishable packages

| In-tree | Published as | Status |
|---|---|---|
| `packages/material-web` (`@material/web` fork + aphrody M3 extensions) | **`@aphrody-code/material-web@2.4.1`** ✅ live | renamed at publish time so the working tree stays a `@material/web` drop-in |
| `apps/m3-react` | **`@aphrody-code/m3-react@1.0.0`** ✅ live | ships `src` (TS) + README + LICENSE |

Both live on GitHub Packages (`github.com/aphrody-code` · npm.pkg.github.com).

> **material-web build note:** upstream's `npm run build` (wireit) is bash-only
> and breaks under Windows cmd (`$(ls -d */ | grep …)`). The publish script runs
> the publishable steps via bash directly — `sass → css-to-ts → tsc` — which
> emits the `.js`/`.d.ts` even with the env-only bun-types/`Timeout` type
> warnings. nested-repo packages also get a temp authed `.npmrc` (removed after).

The other forks — `packages/{lit, ui, tailwindcss, gts}` — are **multi-package
upstream monorepos** consumed in-tree (via `just sync-packages`). Publishing
them whole isn't a single safe operation: each would need every internal
sub-package rescoped to `@aphrody-code/*`. They are intentionally **not**
published individually here; treat them as vendored build inputs.

## Prerequisites — one-time auth

GitHub Packages publish needs a token with **`write:packages`**. The default
`gh` token (`gist read:org repo workflow`) lacks it. Add it:

```sh
gh auth refresh -s write:packages,read:packages
export GITHUB_TOKEN="$(gh auth token)"
```

(or create a classic PAT with `write:packages` and `export GITHUB_TOKEN=…`).

## Publish

```sh
# 1. Validate (no auth needed) — packs each tarball and prints contents:
bun scripts/publish-github-packages.ts

# 2. Publish for real (needs $GITHUB_TOKEN with write:packages):
bun scripts/publish-github-packages.ts --publish

# Publish a single package:
bun scripts/publish-github-packages.ts --publish --only material-web
```

`scripts/publish-github-packages.ts` builds (`npm run build` for material-web),
rewrites the package name to its `@aphrody-code/*` published form in a temp copy
of `package.json`, publishes, then restores the original manifest — so a publish
never leaves the tree renamed. Defaults to **dry-run**; `--publish` is required
to publish.

## Versioning

Bump the `version` in the package's `package.json` before publishing (GitHub
Packages, like npm, rejects re-publishing an existing version). Per
`CLAUDE.md` §0.1 the **first** `v*` tag/publish is human-gated — a maintainer
runs the `--publish` step.

## Consuming

```sh
# consumer .npmrc
@aphrody-code:registry=https://npm.pkg.github.com
//npm.pkg.github.com/:_authToken=${GITHUB_TOKEN}   # read:packages token
```
```sh
npm i @aphrody-code/material-web @aphrody-code/m3-react
```
