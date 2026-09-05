<!-- SPDX-License-Identifier: Apache-2.0 -->
# Publishing aphrody's Material Design 3 packages to npm

aphrody's first-party Material Design 3 packages publish to the **public npm
registry** (`registry.npmjs.org`) under the **`@aphrody`** scope. Publication is
fully automated by the GitHub Actions workflow
[`.github/workflows/release-m3-packages.yml`](.github/workflows/release-m3-packages.yml),
which fires on an **`m3-v*`** tag (decoupled from the core `release.yml`, which
fires on `v*`).

> npm became the canonical registry (was GitHub Packages, `@aphrody-code`) on
> 2026-06-04. The old `scripts/publish-github-packages.ts` /
> `scripts/release.ts` GitHub-Packages flow is gone — the workflow is the only
> publish path.

## Published packages (`@aphrody/*`)

The workflow builds the publishable packages with
`bunx turbo run build --filter='./packages/*'` (turbo pulls in their workspace
deps automatically; `examples/*` are excluded) and publishes them in dependency
order with `bun publish --access public --registry https://registry.npmjs.org`.
`bun publish` inlines every `workspace:*` dependency with its resolved version.

| In-tree | Published as |
|---|---|
| `packages/m3-tokens` | `@aphrody/m3-tokens` |
| `packages/material-web` | `@aphrody/material-web` |
| `packages/react` | `@aphrody/m3-react` |
| `packages/m3-motion` | `@aphrody/m3-motion` |
| `packages/m3-theme` | `@aphrody/m3-theme` |
| `packages/m3-design` | `@aphrody/m3-design` |
| `packages/eslint-plugin-m3` | `@aphrody/eslint-plugin-m3` |
| `packages/doc-ai` | `@aphrody/doc-ai` |
| `packages/bun-rs` | `@aphrody/bun-rs` |

`build:sass` falls back to `sass-embedded` when the `bun-rs` FFI lib is absent,
so CI needs no Rust toolchain.

## Release

```sh
# 1. Bump versions in the relevant packages/*/package.json (npm rejects
#    re-publishing an existing version).
# 2. Commit, then tag and push:
git tag m3-v<version>
git push github m3-v<version>
```

The push of an `m3-v*` tag triggers `release-m3-packages.yml`. The workflow can
also be run manually via `workflow_dispatch`.

### CI secrets

- **`NPM_TOKEN`** — automation token owning the `@aphrody` npm org; used for
  `//registry.npmjs.org/:_authToken=…`.
- **`GH_PACKAGES_TOKEN`** — only needed at *install* time, so any remaining
  private GitHub-Packages transitive dep (e.g. the showcase's optional
  `@aphrody/bxc`) resolves during `bun install`.

## Consuming

```sh
npm i @aphrody/material-web @aphrody/m3-react @aphrody/m3-tokens
```

No registry override or auth is needed — the `@aphrody` scope is public on
`registry.npmjs.org`.
