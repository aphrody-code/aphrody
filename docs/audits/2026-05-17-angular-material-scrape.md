<!-- SPDX-License-Identifier: Apache-2.0 -->

# Audit — Angular Material scrape + source clone

**Date (UTC):** 2026-05-17
**Initiator:** user request — `https://github.com/angular/components` +
              `https://material.angular.dev/components/categories`
**Pipeline:** `scripts/bxc-mass-scrape.ts` + shallow `gh repo clone`
**URL list:** `scripts/bxc-mass-scrape.angular-material.urls.json` (38 URLs)
**Local clone:** `C:/worktree/components` (4 770 files, 39 MB)
**Cache dir:** `var/data/bxc-cache/` (merged with main corpus)

## 1. Two-track capture

### Track A — HTML scrape (38 URLs)

Flags: `--profile=static --concurrency=6 --retry=2 --timeout=30000`.
Run: **38/38 OK, 0 failed, 2 215.9 KB, 1 860 ms** (~20 URL/s).

| Category                       | Count | Bytes (sum) | Notes                                 |
|---|---|---|---|
| `angular-material` (GitHub)    |  3    | 1 010 297   | Real SSR HTML (repo + tree + README) |
| `angular-material-dev` (guide) |  7    |   251 748   | Mixed SSR / SPA shell                |
| `angular-material-component`   | 28    | 1 006 992   | **SPA shell, 35 964 B identical**    |

Same SPA-vs-SSR split as `design.google`:
`material.angular.dev/components/*/overview` ships the empty Angular
shell (35 964 B every page) because the API table + examples are
hydrated client-side. Real component documentation is only reachable
with a JS-executing profile (`fast` / `stealth` / `max` → requires
`bxc-engine` binary not yet installed).

GitHub pages render server-side and gave us real content:
- `github.com/angular/components` — repo landing (~337 KB)
- `github.com/angular/components/tree/main/src/material` — folder
  listing of 45 entries
- `github.com/angular/components/blob/main/README.md` — full README

### Track B — Source repo clone (full source access)

```pwsh
gh repo clone angular/components -- --depth=1 --no-tags --filter=blob:none
```

Landed at `C:/worktree/components/` (intentional default — matches
the pattern used for `C:/worktree/bxc/`). Stats:

- 4 770 files materialized
- 39 MB disk footprint
- `src/material/` contains **42 component source directories** plus
  Bazel + SCSS infrastructure

The shallow + `blob:none` flags keep the clone fast and lazy — file
contents are fetched on demand when read.

## 2. What the source gives us (the real win)

For each Angular Material component (e.g. `src/material/button/`):

- `_button-base.scss` — base mixin (typography/layout, theme-agnostic)
- `_button-theme.scss` — entry point, dispatches by theme version
- `_m2-button.scss` — Material 2 tokens
- `_m3-button.scss` — **Material 3 tokens** (the gold reference)
- `_fab-theme.scss`, `_icon-button-theme.scss` — variants
- `button-base.ts`, `button-module.ts`, `button.html` — TS + template
- `_button.scss`, `button-high-contrast.scss` — runtime CSS

Example excerpt from `src/material/button/_button-theme.scss:1-20`:

```scss
@use '../core/theming/inspection';
@use '../core/tokens/token-utils';
@use '../core/typography/typography';
@use './m2-button';
@use './m3-button';
@use 'sass:map';

@mixin base($theme) {
  $tokens: map.get(m2-button.get-tokens($theme), base);
  @if inspection.get-theme-version($theme) == 1 {
    $tokens: map.get(m3-button.get-tokens($theme), base);
  }
  @include token-utils.values($tokens);
}
```

This is the **canonical 3-way parity matrix** Aphrody can use to
verify our `crates/m3-tokens`, `crates/shadcn-bridge`, and the
`m3-shadcn-pixel-perfect.html` demo all agree with Google's official
Angular implementation.

## 3. Cross-reference matrix (shadcn ↔ MWC3 ↔ Angular Material)

For the 12 primitives already covered by `crates/shadcn-bridge`:

| shadcn     | MWC3 (`<md-*>`)         | Angular Material (`mat-*`)        | Source path                           |
|---|---|---|---|
| Button     | `md-filled-button` etc. | `mat-button`, `mat-flat-button`   | `src/material/button/`                |
| Input      | `md-outlined-text-field`| `mat-form-field` + `matInput`     | `src/material/input/`, `form-field/`  |
| Card       | (custom div + elevation)| `mat-card`                        | `src/material/card/`                  |
| Dialog     | `md-dialog`             | `mat-dialog` (overlay)            | `src/material/dialog/`                |
| Tabs       | `md-tabs` + `md-*-tab`  | `mat-tab-group` + `mat-tab`       | `src/material/tabs/`                  |
| Toast      | `md-snackbar`           | `mat-snack-bar`                   | `src/material/snack-bar/`             |
| Select     | `md-outlined-select`    | `mat-select` + `mat-option`       | `src/material/select/`                |
| Checkbox   | `md-checkbox`           | `mat-checkbox`                    | `src/material/checkbox/`              |
| RadioGroup | `md-radio`              | `mat-radio-group` + `mat-radio`   | `src/material/radio/`                 |
| Switch     | `md-switch`             | `mat-slide-toggle`                | `src/material/slide-toggle/`          |
| Slider     | `md-slider`             | `mat-slider`                      | `src/material/slider/`                |
| Avatar     | (custom div + img)      | (not in Angular Material)         | n/a — bespoke per app                 |

11/12 have a 1:1 Angular Material equivalent. Avatar is the only
shadcn primitive without an Angular Material counterpart — confirms
the existing `crates/shadcn-bridge/src/avatar.rs` bespoke
implementation was the right call.

## 4. Files committed in this pass

- `scripts/bxc-mass-scrape.angular-material.urls.json` — 38-URL set
- `docs/audits/2026-05-17-angular-material-scrape.md` — this report

Out of repo (intentional — same convention as `C:/worktree/bxc/`):

- `C:/worktree/components/` — full Angular Material source clone

## 5. Next actions

- When `bxc-engine` binary is installed, re-run with
  `--profile=fast --mode=full` to capture full `material.angular.dev`
  component pages (currently SPA shell only).
- Add Angular Material `_m3-*.scss` token files to the
  `crates/m3-tokens` fallback table as a third source of truth
  (alongside the existing M3 spec values + the M3 web tokens scrape).
- Consider a `crates/m3-tokens/build.rs` that reads selected
  `_m3-*.scss` files from `C:/worktree/components/src/material/` and
  emits a Rust validation table to ensure our token values stay in
  lock-step with the upstream Angular Material implementation.
