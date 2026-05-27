<!-- SPDX-License-Identifier: Apache-2.0 -->
# The aphrody UI stack (unified)

One stack, one token source, one component foundation. This is the standard for
every first-party aphrody UI; new apps follow it rather than introducing a
parallel approach.

## The three layers

| Layer | What | Where |
|---|---|---|
| **Tokens** | The Material 3 fusion (M3 `--md-sys-color-*` + shadcn/ui aliases + Tailwind v4 `@theme`), light + dark. | `@aphrody-code/theme` (`apps/theme/tokens.css`) |
| **Components** | Material Web (Lit web components) as the cross-framework base; `@aphrody-code/m3-react` for React surfaces. | `packages/material-web` (fork), `apps/m3-react` |
| **Engine access** | The full aphrody CLI in-process via `bun:ffi`. | `@aphrody-code/native` (wraps the Rust `aphrody-ffi` cdylib) |

## Rules

1. **Tokens are generated, never hand-rolled.** The single source of truth is
   the Rust CLI: `aphrody design tokens --fusion` (+ `--dark`). It is
   materialised once in `@aphrody-code/theme`; every app imports
   `@aphrody-code/theme/tokens.css` and styles with `var(--md-sys-color-*)` (or
   the shadcn aliases). No app defines its own palette.
2. **Material Design 3 is the design language.** Use Material Web components
   (or `m3-react` in React) over bespoke widgets; reach for raw elements only
   for layout, themed with the tokens.
3. **Forks under `packages/*` are building blocks, not first-party code.** They
   keep their own `.git` and conventions; do not reformat them. First-party UI
   lives in `apps/*`.
4. **The UI talks to the engine in-process, not by subprocess.** `bun:ffi` runs
   in Bun, not the browser, so a web UI is a Bun server backed by
   `@aphrody-code/native` plus a Material Web frontend (see `apps/console`).

## First-party apps

| App | Package | Role |
|---|---|---|
| `apps/theme` | `@aphrody-code/theme` | canonical M3 fusion tokens (this stack's foundation). |
| `packages/native` | `@aphrody-code/native` | in-process bridge to the aphrody cdylib (`bun:ffi`). |
| `apps/console` | `@aphrody-code/console` | live M3 web console driving the real CLI (the reference UI). |
| `apps/m3-react` | `@aphrody-code/m3-react` | React 19 wrappers for Material Web. |
| `apps/design` | `@aphrody/design` | M3 / Google Design generation engine (HCT, token authoring). |

## Beyond the browser (roadmap)

`apps/console` (Bun server + system browser) is the shipped reference UI and the
zero-overhead default — the best fit for the headless / LLM-first ethos. When a
packaged desktop/mobile app is wanted, the validated next step (see the Rust
repo's `docs/research/gui-options-2026.md`) is **Tauri v2 with a Rust backend
that links `aphrody-ffi` in-process** (system webview, reuses this exact M3
fusion frontend, MIT/Apache — no redundant sidecar). A webview-free pure-Rust
path (Xilem + Masonry + Vello/wgpu, reusing `m3-tokens`) is the longer-term
option once it stabilises. Avoid GPUI (GPL-3.0 + no Windows) and Slint's GPL.

## Notes

- A non-Tailwind consumer of `tokens.css` (e.g. the console) sees a harmless
  bundler warning on the Tailwind `@theme` block; the M3 + shadcn variables it
  actually uses bundle normally. Tailwind apps consume the same block as
  intended.
- Force the dark theme with `class="dark"` on the root element; otherwise the
  sheet follows `prefers-color-scheme`.
