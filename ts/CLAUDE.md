# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this repo is

`aphrody-ts` is the **Bun workspace** for aphrody's first-party TypeScript/JS, extracted 2026-05-23 from the Rust monorepo at `C:\src\aphrody` (which remains the core). It is the JS/TS side of the aphrody **Material Design 3 fusion** (lit + material-web + shadcn + Tailwind). The runtime is **Bun**, not Node; the package manager is `bun`.

## Repository layout — two tiers that are managed differently

The single most important structural fact: `apps/*` and `packages/*` are governed by different rules.

- **`apps/*` — Bun workspace members, tracked by this repo.** This is where first-party code lives. Currently just `apps/m3-react`. These are the only `workspaces.packages` entries (see root `package.json`), and the only paths linted/formatted by default.
- **`packages/*` — vendored UI forks, NOT tracked and NOT workspace members.** `lit`, `material-web`, `tailwindcss`, `ui`, `gts`, `bxc`, `lightpanda`. Each keeps its **own `.git`** and its own native package manager (npm/pnpm), and is pushed to its own `aphrody-code/*` remote. `.gitignore` excludes all of `packages/` to avoid accidental gitlink embedding. They are consumed in-tree as build inputs (synced via `just sync-packages` in the upstream Rust monorepo — there is no justfile here). oxlint/oxfmt explicitly ignore `packages/**`. Treat these as third-party: don't reformat them, don't expect them to follow this repo's conventions.

`docs/` is reference/design material (M3 spec extractions, Google design research, `docs/ts/tsguide.md` = Google TS style guide). `docs/design/M3-FRAMEWORK.md` is the design-system anchor.

## Commands

Run from repo root unless noted. Runtime is Bun.

```sh
bun install              # install workspace deps (exact lockfile, hoisted linker — see bunfig.toml)
bun test                 # run tests (bun's built-in runner; --coverage to opt in)
bun test path/to/x.test.ts   # run a single test file
bun test -t "name"       # run tests matching a name filter
oxlint                   # lint (config: .oxlintrc.json) — `bun run lint`
oxfmt apps/              # format apps/ — `bun run fmt`; `oxfmt --check apps/` to verify
```

Per-package (e.g. in `apps/m3-react`):

```sh
bun run typecheck        # tsc --noEmit (this is the build check — tsconfig is noEmit, no bundle step)
bun run lint             # oxlint
bun run fmt              # oxfmt src
```

There is currently no test file in the tracked tree; `m3-react` is type-checked via `tsc --noEmit`, not built.

## Toolchain conventions

- **Linter/formatter is oxc, not ESLint/Prettier.** `oxlint` (`.oxlintrc.json`: correctness=error, suspicious/perf=warn, style=off; plugins typescript/unicorn/oxc/promise/import) and `oxfmt` (`.oxfmtrc.json`). Both ignore `packages/**` and `apps/photoshop-*`.
- **Code style follows the Google TypeScript Style Guide** (`docs/ts/tsguide.md`).
- **Every source file starts with `// SPDX-License-Identifier: Apache-2.0`.** Match the existing header style (the `//!` module-doc convention seen in `m3-react/src/*` is borrowed from Rust and used here deliberately).
- TS config is strict + `verbatimModuleSyntax` + `allowImportingTsExtensions` + `moduleResolution: bundler`. Imports of `@material/web/**` use explicit `.js` extensions (this is how Material Web ships).
- Comments and prose in this repo are frequently in **French** (matching the maintainer); keep that register when editing existing files.

## m3-react architecture

`apps/m3-react` (published as `@aphrody-code/m3-react`) is the React 19 bridge to Material Web:

- `src/index.ts` — wraps each Material Web `md-*` custom element as a typed React component via `@lit/react`'s `createComponent`, mapping React props → element properties and React handlers → DOM events (React doesn't do this for custom elements natively). Grouped by M3 taxonomy. ~32 components; gaps tracked in `docs/design/m3-components-spec.md`.
- `src/interactions.tsx` — React hooks/components reproducing design.google + gemini.google.com interaction patterns (View Transitions, scroll reveal, thinking/streaming shimmer) on standard platform APIs, all reduced-motion aware.
- `src/theme.css` — M3 system tokens (`--md-sys-color-*`) + shadcn/Tailwind alias sheets. **Generated**, not hand-edited: `aphrody design tokens --fusion -o apps/m3-react/src/theme.css` (the `aphrody` CLI lives in the Rust monorepo).

## Publishing

Packages publish to **GitHub Packages** (`npm.pkg.github.com`, org `aphrody-code`), not npmjs.com. See `PUBLISHING.md`. Only two targets are publishable: `apps/m3-react` and `packages/material-web` (the latter is renamed `@material/web` → `@aphrody-code/material-web` at publish time only).

```sh
bun scripts/publish-github-packages.ts             # dry-run (default, safe — packs + prints tarball contents)
bun scripts/publish-github-packages.ts --publish   # real publish (needs $GITHUB_TOKEN with write:packages)
bun scripts/publish-github-packages.ts --publish --only m3-react
```

The script rewrites the package name in a temp `package.json` copy and restores it after, so the working tree is never left renamed. The other forks (`lit`/`ui`/`tailwindcss`/`gts`) are multi-package monorepos and are intentionally **not** published from here. Auth needs `write:packages` (the default `gh` token lacks it): `gh auth refresh -s write:packages,read:packages && export GITHUB_TOKEN="$(gh auth token)"`. The first `v*` publish is human-gated.
