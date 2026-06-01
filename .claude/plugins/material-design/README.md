<!-- SPDX-License-Identifier: Apache-2.0 -->

# material-design — Claude Code plugin

Guides Claude toward **Material Design 3** (Material You), **Google-style**,
**responsive and adaptive** UI. It carries token-accurate M3 knowledge (HCT color
roles, type scale, elevation, shape, motion, state layers, window size classes,
accessibility) and a set of action skills + autonomous agents to **create**,
**spec-check**, **template**, **correct** and **migrate** UI to M3.

It is co-located with the [`material-web`](../../) monorepo
(`@material/web` Lit components + `@aphrody-code/m3-react` wrappers +
`@aphrody-code/m3-tokens` + `@aphrody-code/eslint-plugin-m3`) and is M3-source
faithful: it never invents numbers — values trace to `m3.material.io`,
`material-color-utilities`, `material-tokens` and `material-web.dev`.

## Install

This repo is a Claude Code **marketplace** (see `../.claude-plugin/marketplace.json`).

```bash
# from a Claude Code session
/plugin marketplace add /home/ubuntu/aphrody/.claude/plugins
/plugin install material-design@aphrody-material
```

Or point at the plugin directly while developing:

```bash
claude --plugin-dir /home/ubuntu/aphrody/.claude/plugins/material-design
```

## Skills (15)

One skill per concern, with a skill dedicated to every monorepo package so the
plugin is complete across the whole system.

### Design knowledge & docs

| Skill                 | Kind             | Use it to                                                                                                                                                                                                |
| --------------------- | ---------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `m3-design-guide`     | knowledge (auto) | Apply M3 correctly: color roles, tokens, type/shape/elevation/motion, state layers, the Google-style mindset. Loaded whenever you design or style UI.                                                    |
| `responsive-adaptive` | knowledge (auto) | Pick the window size class, breakpoints, canonical layout (list-detail / supporting-pane / feed) and adaptive navigation (bar / rail / drawer).                                                          |
| `m3-docs`             | knowledge/action | Answer M3 spec questions authoritatively, attributed, **Google sources only** — backed by the `docs/design/m3-material-io-llms.txt` link map (167 m3.material.io pages).                                 |
| `google-design`       | knowledge        | Apply the Google design language: Google Sans / Product Sans, the Gemini sparkle visual language, Material You expressiveness — backed by `docs/design/google-design-llms.txt` (93 design.google pages). |

### Build, check, fix, migrate

| Skill           | Kind   | Use it to                                                                                                                                                                      |
| --------------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `m3-component`  | action | Scaffold a Material component — the `md-*` Lit + React 3-point wiring in the monorepo, or a self-contained token-driven component in any project.                              |
| `m3-spec-check` | action | Audit a file/dir for M3 compliance (color roles, type, shape, spacing, state layers, icons, a11y, migration residue). Read-only report.                                        |
| `m3-template`   | action | Generate a ready M3 page/screen: adaptive shell + canonical layout, M3 components, tokens, Tailwind for host layout.                                                           |
| `m3-correct`    | action | Fix M3 violations: hardcoded colors -> roles, `sx` -> Tailwind+tokens, MUI prop names, icon names, `fontVariationSettings` -> `--md-icon-*`.                                   |
| `migrate-mui`   | action | Migrate a React + MUI / MUI X codebase to material-web (M3) — codemods when available, else guided manual port. Carries the upstream MUI docs index + skills as `references/`. |

### Package-specific (one per `@aphrody-code/*`)

| Skill                | Package            | Use it to                                                                                                                |
| -------------------- | ------------------ | ------------------------------------------------------------------------------------------------------------------------ |
| `m3-dynamic-color`   | `m3-tokens`        | Material You runtime: derive/apply `--md-sys-color-*` from a seed, MUI theme -> tokens, breakpoints.                     |
| `m3-tailwind`        | `m3-theme`         | Wire M3 tokens into Tailwind v4 `@theme` and the shadcn/ui alias fusion sheet.                                           |
| `m3-motion`          | `m3-motion`        | M3 motion in React: transition patterns (fade-through / shared-axis / container transform), easings, durations, springs. |
| `m3-design-compiler` | `m3-design`        | Natural-language brief -> M3 React scaffold (HCT theme + `Md*` layout tree + critique).                                  |
| `m3-lint`            | `eslint-plugin-m3` | Set up and run the 8 M3 lint rules (oxlint / ESLint) on a consuming site.                                                |
| `m3-doc-ai`          | `doc-ai`           | Translate docs and generate Lit component API guides (Gemini backend, offline fallback).                                 |

## Agents (5)

| Agent                  | Role                                                                                                    | Writes?        |
| ---------------------- | ------------------------------------------------------------------------------------------------------- | -------------- |
| `md-component-builder` | Autonomously create an M3 component end-to-end (+ verify build/typecheck + spec-check).                 | yes            |
| `md-spec-checker`      | Autonomously audit a whole codebase for M3 conformance; produces a severity-grouped findings report.    | no (read-only) |
| `md-corrector`         | Autonomously remediate M3 violations across a codebase, re-linting until clean.                         | yes            |
| `md-migrator`          | Autonomously run a MUI -> M3 migration (scope, sandbox, codemods/manual, sx wall, theme seed).          | yes            |
| `md-design-researcher` | Read-only research of a single M3 spec value, **Google/Material sources only**, distilled + attributed. | no (read-only) |

## MCP server — `mui-docs`

`.mcp.json` wires a bundled MCP server (`mcp/mui-mcp/`) that serves the official
**MUI documentation** to the model — useful when migrating a MUI / MUI X app to M3
(knowing the source idioms). It is a bun-native, vendored + completed port of
`@mui/mcp` (MIT; the original `@mastra/*` + private `@mui-chat/tools` deps were
replaced with `@modelcontextprotocol/sdk` and native TypeScript, no build step).
Four tools: `use_mui_docs`, `fetch_docs`, `list_doc_sources`, `fetch_mui_docs`
(covering `@mui/material` v5-9, `@mui/x-*`, system, icons). Run `bun install` once
in `mcp/mui-mcp/` so its deps resolve; no env vars or API key required.

## Hook

`PostToolUse` (Edit/Write/MultiEdit) runs `scripts/color-guard.sh`: a non-blocking
nudge when a just-edited UI file contains a **hardcoded color** in a style/`sx`
context, reminding to use a `var(--md-sys-color-<role>)` role so the UI follows the
theme, dark mode and dynamic color. Token/palette/theme source files are skipped.

## Design principles it enforces

- **Roles, never raw values.** Components reference semantic color roles
  (`primary`, `surface`, `on-surface`, `outline`, `error`, ...), never hex.
- **Tokens.** Web exposes `--md-sys-color-*` at runtime; type/shape/elevation/
  motion are resolved at build time. The plugin respects this split.
- **Adaptive first.** Layouts swap at the 5 M3 window size classes; navigation
  follows (bar -> rail -> drawer); 4 dp grid.
- **Accessible by construction.** Tonal contrast (4.5:1 / 3:1), 48 dp touch
  targets, never color-alone, visible focus.
- **Web reality.** M3 **Expressive** (springs, shape morphing, expressive type,
  new components) is **not** available on the web in 2026 — the plugin uses the
  tokenized foundations and flags the gap.

## Relationship to the monorepo

The action skills and agents prefer the in-repo tooling when present
(`add-md-component`, `migration/codemods/`, `migration/mui-m3-map.json`,
`@aphrody-code/eslint-plugin-m3`) and degrade gracefully to portable, knowledge-
driven behavior in any other project.

bun only — never `npm` / `pnpm`.
