<!-- SPDX-License-Identifier: Apache-2.0 -->

# Aphrody Material 3 extensions

This fork of `@material/web` is completed by aphrody with the M3 components
upstream never shipped stable, the adaptive layout family, full Google Sans
Flex typography control, and WebGPU brand effects. Everything is written in Lit
and is **self-contained**: each component consumes the `--md-sys-*` design
tokens directly (with fallbacks) and paints its own elevation/state layers, so
the bundle builds with `tsc` + `lit` alone — no SASS pipeline required.

Entry points:

- **`aphrody-components.ts`** — all new self-contained components (below).
- **`aphrody-labs.ts`** — promoted upstream `labs/` components (badge, cards,
  navigation bar/drawer/tab, outlined segmented button set).
- Both are re-exported from **`all.ts`**.
- React wrappers: **`apps/m3-react`** (`@aphrody-code/m3-react`), one per element.

## New components (24 custom elements)

| Tag                                              | Category          | Notes                                                                                                          |
| ------------------------------------------------ | ----------------- | -------------------------------------------------------------------------------------------------------------- |
| `md-snackbar`                                    | communication     | dismissive/non-dismissive, slot action, auto-dismiss with hover/focus pause                                    |
| `md-loading-indicator`                           | communication     | M3 Expressive morphing indicator, determinate/indeterminate                                                    |
| `md-navigation-rail` + `md-navigation-rail-item` | navigation        | collapsed (80dp) / expanded (≥220dp), menu + FAB slots, active-indicator pill                                  |
| `md-top-app-bar`                                 | navigation        | small / center / medium / large + on-scroll fill                                                               |
| `md-bottom-app-bar`                              | navigation        | 80dp, actions + FAB slot                                                                                       |
| `md-search-bar`                                  | navigation        | docked / fullscreen search view, results slot                                                                  |
| `md-toolbar`                                     | navigation/action | docked + floating                                                                                              |
| `md-bottom-sheet`                                | containment       | standard/modal, 28dp top radius, drag-to-dismiss                                                               |
| `md-side-sheet`                                  | containment       | start/end, standard/modal, RTL-aware                                                                           |
| `md-carousel` + `md-carousel-item`               | containment       | hero / multi-browse, CSS scroll-snap                                                                           |
| `md-button-group`                                | action            | connected group, single/multi-select, roving tabindex                                                          |
| `md-fab-menu` + `md-fab-menu-item`               | action            | 2–6 staggered actions, +→✕ morph                                                                               |
| `md-date-picker`                                 | selection         | docked calendar, real date arithmetic, min/max                                                                 |
| `md-time-picker`                                 | selection         | input variant, 12h/24h, normalized output                                                                      |
| `md-scaffold`                                    | layout            | adaptive regions, ResizeObserver → `size-class`                                                                |
| `md-pane`                                        | layout            | fixed / flexible pane                                                                                          |
| `md-list-detail`                                 | layout            | adaptive list-detail (1 pane compact, 2 panes expanded+)                                                       |
| `md-supporting-pane`                             | layout            | adaptive main + supporting pane                                                                                |
| `md-type`                                        | typography        | M3 type scale + per-axis Google Sans Flex control, animatable; `code` mode → Google Sans Code (MONO/wght axes) |
| `md-webgpu-canvas`                               | effects           | spectrum-shift / sparkle / glimmer (WGSL + CSS fallback)                                                       |

### Angular Material parity (+ CDK)

Equivalents for every `angular/components` `src/material/*` component were added:
`md-tooltip`, `md-expansion-panel`/`md-accordion`, `md-grid-list`/`md-grid-tile`,
`md-table` (with column sort), `md-paginator`, `md-stepper`/`md-step`,
`md-autocomplete`, `md-tree`/`md-tree-item`, plus the CDK virtual-scroll gap
(`md-virtual-scroller`). See `docs/design/angular-material-parity.md`. **94
`md-*` tags total; `tsc` + `bun build` green.**

## Shared internals

- **`internal/motion/easing-and-duration.ts`** — the 7 M3 easings + 16 duration
  tokens (aligned with `crates/m3-tokens/src/motion.rs`), `transition()`,
  `animationOptions()`, `prefersReducedMotion()`.
- **`typography/internal/google-sans-flex-axes.ts`** — the 6 variable axes
  (`wght`/`opsz`/`wdth`/`GRAD`/`slnt`/`ROND`), mirror of the Rust source.
- **`typography/internal/type-scale.ts`** — the 15 M3 type styles with their
  Google Sans Flex axis values.
- **`typography/internal/font-face.ts`** — `@font-face` for the self-hosted
  variable TTF + the Google Fonts CDN href.
- **`layout/internal/scaffold.ts`** — window-size-class classifier, boundaries
  (`600/840/1200/1600`) matching `crates/m3-tokens/src/adaptive.rs`.

## Build & validation (bun direct)

The aphrody bundle bypasses the upstream `wireit → SASS → css-to-ts → tsc`
pipeline — components carry their styles as Lit `css` literals, so a single
Bun pass suffices (see https://bun.com/docs/bundler/loaders and
https://bun.com/docs/bundler/css).

```sh
# Type-check (bun-installed global tsc, dedicated project):
bun run typecheck:aphrody    # tsc -p tsconfig.aphrody.json  → exit 0

# Bundle (Bun's native ts loader; lit kept external):
bun run build:aphrody        # bun run aphrody-build.ts → dist-aphrody/  (~102 KB)
```

`bun build` transpiles TypeScript (decorators honored via `experimentalDecorators`)
and bundles in ~300 ms (74 modules). The JS minifier runs with
`{whitespace, syntax, identifiers, keepNames}` — `keepNames` preserves the
component class names in DevTools — plus `drop: ['debugger']`.

**CSS-in-JS (`aphrody-css-minify.ts`, opt-in).** Bun's JS minifier never touches
the contents of `css` template literals. The optional `aphrody-css-in-js` plugin
routes each literal through Bun's LightningCSS port in a single up-front
subprocess (run in `onStart`; an in-process nested `Bun.build` would deadlock
the bundler). It both strips whitespace and _transpiles for older browsers_
(vendor prefixes, color fallbacks, logical-property lowering) — which slightly
**increases** size, so it ships off by default:

```sh
bun run aphrody-build.ts --css-transpile   # widen CSS support (~107 KB)
bun run aphrody-build.ts --no-min          # readable JS for debugging
```

`dist-aphrody/` is git-ignored.

## Modern web platform (via modern-web-guidance)

Components are progressively enhanced with current platform features, per the
GoogleChrome `modern-web-guidance` skill. **Browser-support policy:** these
components target modern engines (WebGPU, Shadow DOM) — Baseline-Newly-Available
features are used with **feature detection + graceful degradation**, no
polyfills.

- **`md-snackbar` → Popover API.** The surface is `popover="manual"`, so it
  lives in the browser **top layer** and can never be occluded by dialogs or
  other overlays (the M3-recommended toast pattern). Entry/exit is declarative
  CSS — `@starting-style` + `transition-behavior: allow-discrete` transitioning
  `overlay` — replacing the previous Web Animations code. Falls back to inline
  rendering where the Popover API is absent.
- **`md-top-app-bar` → scroll-driven CSS.** The on-scroll fill uses
  `animation-timeline: scroll(block nearest)` (compositor-driven, no main-thread
  scroll work), guarded by `@supports`. The JS scroll listener now runs only as
  a fallback — when a custom `scrollTarget` is set or scroll-driven animations
  are unsupported (e.g. Firefox); the `js-scroll` attribute switches between the
  two so they never conflict. Honors `prefers-reduced-motion`.

Candidate next passes (search the skill, then apply): anchor positioning for
`md-menu`/tooltips/`md-fab-menu`, `@starting-style`/popover for the sheets &
dialog, scroll-snap state queries for `md-carousel`, View Transitions for
`md-scaffold`/`md-list-detail` navigation.

## Token alignment

All values trace to the Rust source of truth in `crates/m3-tokens` and to the
canonical Google references (`m3.material.io`, `design.google`,
`fonts.google.com/specimen/Google+Sans+Flex`). Theme with
`aphrody design tokens --fusion`.
