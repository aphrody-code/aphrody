---
name: m3-lint
description: >
  Set up and run @aphrody-code/eslint-plugin-m3 to lint a site that consumes material-web.
  Use when asked to "lint M3 usage", "set up the m3 eslint/oxlint plugin", "catch sx on
  md-*", "validate icon names", "enforce M3 color roles", "find leftover MUI props", or
  "guard a migration with lint".
---

# m3-lint

`@aphrody-code/eslint-plugin-m3` lints the **sites that consume** material-web
(`@aphrody-code/m3-react` + Material Symbols) — not the library itself. It is the
continuous complement to the migration codemods: what the codemods leave behind,
the plugin catches in CI. ESLint-compatible API → runs under **oxlint** (`jsPlugins`)
and **ESLint** (`plugins`). JS-only (ESM, no build).

## The 8 rules

| Rule                           | Default | Fix  | Catches                                                                                                                                                              |
| ------------------------------ | ------- | :--: | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `m3/valid-icon-name`           | error   | auto | `<md-icon>`/`<MdIcon>` child must be a real Material Symbols glyph (snake_case), validated vs the 4253 names. Fixes PascalCase MUI leftovers (`Delete` -> `delete`). |
| `m3/valid-color-role`          | error   |  —   | `var(--md-sys-color-<role>)` must be one of the ~49 real roles (typo catcher; suggests the closest).                                                                 |
| `m3/no-sx-prop`                | error   |  —   | `sx` has no effect on `md-*` (no Emotion runtime).                                                                                                                   |
| `m3/no-mui-prop-on-md`         | error   | auto | MUI prop names left on a wrapper (`checked`->`selected`, `title`->`text`, `open`->`opened`, `value`->`activeTabIndex`).                                              |
| `m3/no-mui-import`             | warn    |  —   | residual `@mui/material` / `@mui/icons-material` / `@mui/x-*` imports.                                                                                               |
| `m3/prefer-icon-token`         | warn    |  —   | inline `fontVariationSettings` on an icon -> `--md-icon-*` axis tokens.                                                                                              |
| `m3/require-icon-button-label` | warn    |  —   | icon-only buttons without an accessible name (`aria-label`) — WCAG 4.1.2.                                                                                            |
| `m3/no-hardcoded-color`        | warn    |  —   | hardcoded `#hex`/`rgb()` in `style`/`sx` of an `md-*` -> `var(--md-sys-color-*)`.                                                                                    |

Two rules are auto-fixable: run `oxlint --fix` (or `eslint --fix`) to rename
leftover MUI props and snake_case icon glyphs in one pass.

## Setup — oxlint (recommended, Rust, fast)

```bash
bun add -D @aphrody-code/eslint-plugin-m3 oxlint
```

`.oxlintrc.json`:

```json
{
  "jsPlugins": ["./node_modules/@aphrody-code/eslint-plugin-m3/index.js"],
  "rules": {
    "m3/valid-icon-name": "error",
    "m3/valid-color-role": "error",
    "m3/no-sx-prop": "error",
    "m3/no-mui-prop-on-md": "error",
    "m3/no-mui-import": "warn",
    "m3/prefer-icon-token": "warn",
    "m3/require-icon-button-label": "warn",
    "m3/no-hardcoded-color": "warn"
  }
}
```

> `jsPlugins` paths resolve relative to the `.oxlintrc.json` file. Verified on oxlint 1.67.

## Setup — ESLint (flat config)

```js
import m3 from "@aphrody-code/eslint-plugin-m3";
export default [
  m3.configs.recommended, // structural rules error, suggestions warn
  // or m3.configs.strict — everything error (use to close out a MUI->M3 port)
];
```

## Use during migration

Turn it on from the first file of a `migrate-mui` port and run `--fix` repeatedly:
the auto-fixers clear the mechanical MUI residue, and the remaining `error`s
(`no-sx-prop`) mark the irreducible manual styling work. Pairs with the
`m3-correct` skill (which remediates) and the `migrate-mui` skill (which ports).

Verify in-repo: `cd packages/eslint-plugin-m3 && bun test/run.ts` (runs real oxlint
on fixtures, 8/8 fire on bad, 0 on good).
