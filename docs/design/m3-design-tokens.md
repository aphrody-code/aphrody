<!-- SPDX-License-Identifier: Apache-2.0 -->
# Material 3 design tokens — foundation

A recap of the Material Design 3 **design-tokens foundation**, mapped from
<https://m3.material.io/foundations/design-tokens>. All explanations below are
**paraphrased in our own words** (short summaries, not Google's verbatim text);
defer to the source page for canonical wording. The closing section maps these
concepts onto our [`crates/m3-tokens`](../../crates/m3-tokens) crate. See also
the [M3 glossary](m3-glossary.md) and the [aphrody token subset](aphrody-m3-tokens.md).

## What a design token is

A design token is a small, reusable, named design decision. Instead of pasting
a raw value (a hex color, a font, a measurement) everywhere, you name it once
and reference the name. A token has two parts:

- a **code-like name** (e.g. `md.ref.palette.secondary90`), and
- an **associated value** (e.g. `#E8DEF8`) — which may itself be another token.

Because the name expresses intent ("secondary container color") rather than a
literal value, design and code can both point at the same token and stay in sync
even when the underlying hex/measurement is later changed.

## Why tokens matter

Tokens give a design system a single source of truth: style choices live in one
tracked repository, and updates propagate consistently across a product or a
whole suite of products. They let designers and engineers "speak the same
language" at handoff, and they make system-wide reskins (high-contrast palette,
larger type scale for TV) a matter of re-pointing tokens rather than editing
call sites. M3 considers tokens worthwhile when you are building from scratch,
shipping across multiple platforms, or want features like dynamic color; less so
for a frozen hard-coded app that will not change.

## The three-tier token model (ref / sys / comp)

Material defines three **classes** of tokens. Layering them lets a team change a
value globally or scope a change to one component:

1. **Reference tokens** (`md.ref.*`) — the full palette of raw options
   (every tonal-palette swatch, each typeface). They usually point to a static
   value (hex code, font size) but can point to another reference token. They do
   **not** change with context. Think of them as the approved starting kit.
2. **System tokens** (`md.sys.*`) — the *decisions and roles* that give the
   system its character: which reference token plays the "primary" role, which
   typeface is "label medium", what shape/elevation means here. **This is where
   theming happens** — a system token can resolve to different reference tokens
   per context (light vs dark). System tokens should point at reference tokens,
   not raw values, whenever possible.
3. **Component tokens** (`md.comp.*`) — the properties of a specific component's
   elements (a FAB's container color, a button icon's color, label text). They
   should point at a system or reference token rather than embedding a hex. Not
   every component property must be a token, but anything shared across
   similar-intent components should be. (M3 marks this class "in development".)

So a value flows: **reference → system → component → resolved pixel on screen.**
Example chain: `md.comp.fab.primary.container.color` → `md.sys.color.primary` →
`md.ref.palette.primary40` → `#6750A4`.

## Naming scheme: `md.{ref,sys,comp}.*`

Token names read general → specific, segments separated by periods:

1. **System prefix** — `md` for Material Design (your own system would use its
   own prefix).
2. **Token class** — `ref`, `sys`, or `comp`.
3. **Role / descriptor** — the purpose, e.g. `color.on-secondary`,
   `motion.easing.emphasized`, `typescale.label-medium.font`.

Examples: `md.ref.palette.secondary90`, `md.sys.color.secondary-container`,
`md.sys.motion.duration.medium2`, `md.comp.fab.primary.container.color`.

## Aliasing

Aliasing is the act of one token pointing at another instead of at a raw value.
System tokens alias reference tokens; component tokens alias system (or
reference) tokens. The chain is what makes a single hex change ripple through
every consumer without touching any token *name* — the names and their meanings
stay stable while the resolved value moves.

## Contexts (incl. light/dark modes)

A **context** is a tagged condition under which a token resolves to a non-default
value — dark theme, dense layout, high contrast, RTL, a given form factor. Think
of a context as a tag: a value tagged "dark theme" overrides the default in a
dark-theme context. Light/dark mode is the canonical example — the *same* system
token (e.g. background color) points at a different reference token depending on
whether the dark-theme context is active. A "light scheme" is a mapping of roles
to tones; it is not the same thing as a "light theme" surface treatment.

---

## → aphrody m3-tokens

Mapping the M3 model onto [`crates/m3-tokens`](../../crates/m3-tokens):

| M3 concept | aphrody crate equivalent |
|---|---|
| **System color tokens** (`md.sys.color.*`) | `color::ColorRoles` struct — each field (`primary`, `on_primary`, `surface_container_high`, …) is exactly one `md.sys.color.*` role, stored as ARGB `u32`. |
| **Reference tokens / tonal palette** (`md.ref.palette.*`) | `tonal` + `dynamic` + `hct` modules — `dynamic::seed_to_palette` runs the HCT seed → 13-tone palette algorithm (the `material-color-utilities` engine), i.e. it generates the `ref.palette` layer at runtime. |
| **Contexts (light/dark)** | distinct `ColorRoles` consts: `BASELINE`/`BASELINE_DARK` (M3 purple `#6750A4`) and `APHRODY`/`APHRODY_DARK` (rust `#CE422B`, dark-first). Same role names, context-specific values — exactly the M3 light/dark context model. |
| **System type scale** (`md.sys.typescale.*`) | `typography` module — the canonical 15-style scale (asserted `== 15` in `lib.rs`). |
| **System shape / elevation / state / motion** (`md.sys.shape/elevation/state/motion.*`) | `shape`, `elevation`, `state`, `motion` modules (see [`m3-motion.md`](m3-motion.md) for the motion gap analysis). |
| **Component tokens** (`md.comp.*`) | not modeled — the crate stops at the system tier; consumers (UXP GUI, WASM demos) bind sys tokens to component elements directly. |
| **Extended colors** (beyond key colors) | `brand-rust` / `success` are aphrody extended colors (documented in `aphrody-m3-tokens.md`); not part of core `ColorRoles`. |

**CSS export.** `color::export_css(&ColorRoles)` emits a `:root { … }` block of
**`--md-sys-color-*`** custom properties (one per role, `#RRGGBB`, alpha
dropped). This is the same flat `md-sys-color-*` shape the official DSP emits in
`css/variables.css` — see the next section — so our output drops straight into
Material Web Components / wgpu UIs. The crate currently exports **36** color
variables, including the expanded surface set
(`surface-dim`, `surface-bright`, `surface-container-{lowest,low,,high,highest}`).

> **Divergence note:** the crate uses Rust `snake_case` field names
> (`on_primary`, `surface_container_high`), which serialize to the M3
> `kebab-case` role names (`on-primary`, `surface-container-high`) in the CSS
> export. The dotted `md.sys.color.` prefix becomes the CSS
> `--md-sys-color-` prefix. Naming is **aligned** with `md.sys.*`; only the
> separator changes per target syntax. There is no exporter yet for the motion
> or shape system tokens (color only).

---

## Source: material-foundation/material-tokens

The official token repo <https://github.com/material-foundation/material-tokens>
publishes the M3 **baseline** theme as an Adobe **Design System Package (DSP)** —
the open folder format for sharing design-system data across tools (it loads in
the Material Theme Builder and the Adobe XD VSCode extension, which runs
**Style Dictionary** to generate per-platform code).

**Repo structure (key paths):**

```
material-tokens/
├── css/                        # hand-published flat CSS, one file per token group
│   ├── colors.css  palette.css  typography.css  shape.css
│   ├── elevation.css  motion.css  state.css  baseline.css
│   └── theme/{light,dark}.css  # the two color contexts
├── dsp/
│   ├── dsp.json                # DSP manifest (spec v0.93)
│   ├── data/                   # source-of-truth token data
│   │   ├── tokens.json         # primary token definitions (~72 KB)
│   │   ├── fonts.json  components.json  docs.json
│   └── dist/styledictionary/   # generated outputs, one folder per target
│       ├── css/variables.css   android/colors.xml  ios-swift/StyleDictionary.swift
│       ├── flutter/style_dictionary.dart  js/tokens.js  scss/variables.scss
│       ├── properties/*.json   # Style Dictionary intermediate properties
│       └── config.js           # Style Dictionary build config
├── tokens.md  README.md  CONTRIBUTING.md  LICENSE
```

**Token format.** Source data lives in `dsp/data/tokens.json` (DSP/Style
Dictionary JSON, **not** the W3C `$value`/`$type` design-tokens format). The DSP
manifest `dsp.json` declares `name: "Material"`, a `md_` snippet prefix, the
four `data/*.json` imports, and ~28 language/framework export targets. Style
Dictionary then flattens those into platform files.

**Naming convention in the generated code.** The flat outputs use the
`md-sys-…` form with **hyphen** separators (the dotted `md.sys.…` written as
CSS-safe segments). Notably the published CSS represents:

- **Color** as `--md-sys-color-<role>` (e.g. `--md-sys-color-primary`) — this is
  exactly the shape `m3-tokens::color::export_css` produces.
- **Easing** decomposed into the four bézier control points as separate vars,
  e.g. `--md-sys-motion-easing-emphasized-decelerate-{x0,y0,x1,y1}`
  (`0.05, 0.7, 0.1, 1.0`), rather than a single `cubic-bezier()` string.
- **Duration** as numeric-keyed vars `--md-sys-motion-duration-50` …
  `--md-sys-motion-duration-1000` (50/100/150/200/250/300/350/400/450/500/550/600/700/800/900/1000 ms).

So the official repo and our crate share the `md-sys-*` system-token naming;
they differ only in (a) format of motion values (control-points vs cubic-bezier
string — see [`m3-motion.md`](m3-motion.md)) and (b) the repo additionally ships
the `ref`/palette and component layers as data, which our crate generates at
runtime (palette) or leaves to consumers (components).

## Source provenance

- Token concept / ref-sys-comp model / naming / contexts:
  <https://m3.material.io/foundations/design-tokens> — fetched 2026-05-21.
- Official DSP repo structure + formats:
  <https://github.com/material-foundation/material-tokens> — fetched 2026-05-21.
- Definitions paraphrased; the source pages are authoritative.
