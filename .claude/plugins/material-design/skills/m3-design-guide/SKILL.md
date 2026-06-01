---
name: m3-design-guide
description: >
  Load this skill whenever the user wants to design or build UI that looks Material Design 3 / Material You / Google-style, including requests like "make this look Material", "apply M3 styling", "which color role or token should I use", "design a Google-style component", "is this on-brand for Material", "M3 color system", "how do I use MD3 tokens on the web", "style this with Material Design", "use M3 elevation or shape", "add M3 motion or animation", "pick the right surface level", "make it look like Google Workspace", or any question about MD3 semantics, color roles, type scale, elevation, shape, motion easing, state layers, or window size classes.
---

# M3 Design Guide — actionable reference for Claude

This skill equips you to produce UI that is correct, on-brand, and accessible according to Material Design 3 (Material You) as implemented on the **web in 2026**. Work from principles and semantic tokens, not raw values. The sections below are directives, not explanations.

---

## Core mindset

Never hard-code a color hex, font size, shadow value, or border-radius into a component. Every visual decision has a named token. Components reference **roles** (semantic meaning), which resolve to values through the three-tier token system. This indirection is what makes theming, dark mode, and dynamic color automatic.

The three tiers:

- **Reference tokens** (`md.ref.*`) — raw values: a specific hex, a font family, a dp measurement. Never used directly in components.
- **System tokens** (`md.sys.*`) — semantic decisions: `md.sys.color.primary`, `md.sys.typescale.body-large-size`. These are the tokens components consume.
- **Component tokens** (`md.comp.<name>.*`) — per-component overrides that point back to sys tokens.

On the **web**, this maps to CSS custom properties:

- `--md-sys-color-<role>` — the ~47 color roles. These are **runtime variables** injected via JavaScript (e.g. `applyDynamicColor`, the Theme Builder output, or `@aphrody-code/m3-tokens/m3-tokens.css`). You can override them with any valid hex/hsl/oklch value at `:root` or a scoped selector.
- `--md-sys-typescale-*`, `--md-sys-shape-corner-*`, `--md-sys-motion-*` — these are resolved at **Sass compile time** in `@material/web` and emitted as static values. They are NOT live CSS variables in the shipped bundle; do not try to override them at runtime by reassigning those names unless you are in a Sass build context. Reach for `--md-sys-color-*` at runtime; reach for Sass tokens or utility classes at build time for the rest.

---

## Color system

M3 uses the **HCT** color space (Hue 0-360, Chroma 0-~120, Tone 0-100). Tone is perceptually uniform — two colors with different tones have a predictable, measurable contrast ratio. This is why accessibility is built in.

From a single seed color, `material-color-utilities` derives five tonal palettes (Primary, Secondary, Tertiary, Neutral, Neutral Variant) plus a fixed Error palette. Each palette is a range varying only in Tone; the standard palette exposes 13 tones: 0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 95, 99, 100.

**Color roles** assign semantic meaning to specific tones. Components use roles, never palette entries directly. Role families:

- **Primary / on-primary / primary-container / on-primary-container** — main accent, CTAs, FABs, filled buttons, active states.
- **Secondary / on-secondary / secondary-container / on-secondary-container** — supporting accent, less prominent interactive surfaces.
- **Tertiary / on-tertiary / tertiary-container / on-tertiary-container** — third accent, used sparingly for contrast or complementary highlight.
- **Error / on-error / error-container / on-error-container** — error states, destructive actions.
- **Surface roles** — use these for hierarchy without accent color:
  - `surface` — default background for cards, sheets, dialogs.
  - `surface-dim`, `surface-bright` — darkest/lightest variant of surface.
  - `surface-container-lowest`, `surface-container-low`, `surface-container`, `surface-container-high`, `surface-container-highest` — ordered from least to most contrast against the base background; use these to layer UI without relying on shadows or borders.
  - `surface-variant` — legacy; prefer `surface-container-*` in new work.
- **`on-surface` / `on-surface-variant`** — primary and secondary text/icons on any surface.
- **`outline` / `outline-variant`** — strong and subtle borders/separators.
- **`background` / `on-background`** — the page-level backdrop (often equal to `surface`).
- **Inverse roles** — `inverse-surface`, `inverse-on-surface`, `inverse-primary` — for snackbars and elements that must invert against the current scheme.
- **Utility** — `surface-tint` (equals `primary`; used for tonal elevation), `scrim` (modal backdrop), `shadow` (drop shadow color).

**The `on-X` convention**: `on-primary` is always the content color placed on top of `primary`. Follow this rule for every role: the content sitting on a container uses the matching `on-*` role. Never put `primary` text on a `primary` background.

**Light/dark**: roles resolve to different tones in each scheme (e.g., `primary` = tone 40 in light, tone 80 in dark). The component never changes — only the scheme changes. Provide both schemes by injecting the appropriate `--md-sys-color-*` values; `prefers-color-scheme` media query or a `data-theme` attribute is the standard web pattern.

**Contrast levels**: Standard (0.0), Medium (0.5), High (1.0). If using `material-color-utilities`, pass the `contrastLevel` parameter to the scheme constructor. With dynamic color this is automatic.

---

## Design tokens: web reality

Web implementation note from this project's CLAUDE.md:

> Only `--md-sys-color-*` is a runtime CSS variable. Typescale, shape, elevation, and motion are resolved at Sass compile time — they are static in `@material/web`'s output.

Practical consequence:

- **Do**: set `--md-sys-color-primary: #6750A4;` at runtime to re-theme.
- **Do**: import `@aphrody-code/m3-tokens/m3-tokens.css` for 62 runtime CSS vars (color + supplemental tokens).
- **Do not**: try to patch `--md-sys-typescale-body-large-size` at runtime and expect it to cascade through the web component styles. That var does not exist as a live custom property in the shipped bundle.
- For exact px values of the type scale, see `references/color-and-tokens.md`.

---

## Typography

5 roles x 3 sizes = 15 styles. Default typeface: **Roboto**. Only two weights are used: **400 (Regular)** and **500 (Medium)**. The full table with exact px, line-height, and tracking is in `references/color-and-tokens.md`.

Role-to-use mapping:

| Role     | When to use                                                                           |
| -------- | ------------------------------------------------------------------------------------- |
| Display  | Large hero text, expressive marketing headers — decorative, not functional navigation |
| Headline | Section titles, screen titles                                                         |
| Title    | Component-level headings, app bar titles, list item headers                           |
| Body     | Running text, descriptions, paragraphs                                                |
| Label    | Button text, chip labels, tab labels, captions on interactive controls                |

Use the smallest style that maintains legibility. Reserve Display for moments of emphasis; use Body for content the user must read. Title Medium and Label Large (both weight 500) carry emphasis at component scale.

**Roboto Flex** (variable font) is the recommended web delivery — one file covers all weights and widths. Load it with `font-display: swap` and subset if needed.

M3 Expressive adds 15 "emphasized" styles (heavier weights). These are **not available on `@material/web` in 2026** — do not reference them.

---

## Elevation

M3 elevation is the z-distance between surfaces. It signals hierarchy via two mechanisms:

- **Tonal elevation (surface tint)**: the surface color is shifted toward `primary`. More elevation = stronger tint. This is the preferred mechanism; it works in both light and dark without relying on shadow visibility.
- **Shadow elevation**: traditional drop shadows. Reserve for elements on complex/photographic backgrounds where tint alone cannot visually separate the surface.

6 levels (0-5): 0dp, 1dp, 3dp, 6dp, 8dp, 12dp. Full table in `references/color-and-tokens.md`.

In practice, prefer **`surface-container-*` roles** for hierarchy. Reaching for `surface-container-high` instead of `surface` automatically creates the visual separation that elevation used to require via shadows or tint overlays — and it adapts automatically to dark mode.

---

## Shape

Shape tokens define corner radius. Classic M3 scale: None (0dp), Extra Small (4dp), Small (8dp), Medium (12dp), Large (16dp), Extra Large (28dp), Full (9999dp / pill). Web token: `--md-sys-shape-corner-<token>`.

M3 Expressive adds intermediate values and shape morphing (animated form transitions). **Neither is available on `@material/web` in 2026.** Use the classic 7-step scale.

Corner family defaults to **rounded**. Individual per-corner radii are supported; use them only when the component spec calls for it (e.g., bottom sheets use Large top corners only).

---

## Motion

Web uses the easing/duration model. M3 Expressive's physics/springs system is **not available on the web in 2026**.

**Easing tokens** (use the cubic-bezier values directly):

| Token                                                   | Use case                                                       |
| ------------------------------------------------------- | -------------------------------------------------------------- |
| Standard `cubic-bezier(0.2, 0, 0, 1)`                   | Transitions that begin and end on screen                       |
| Standard Decelerate `cubic-bezier(0, 0, 0, 1)`          | Elements entering the screen                                   |
| Standard Accelerate `cubic-bezier(0.3, 0, 1, 1)`        | Elements leaving the screen                                    |
| Emphasized Decelerate `cubic-bezier(0.05, 0.7, 0.1, 1)` | Prominent entering transitions (preferred for primary content) |
| Emphasized Accelerate `cubic-bezier(0.3, 0, 0.8, 0.15)` | Prominent exiting transitions                                  |
| Linear `cubic-bezier(0, 0, 1, 1)`                       | Crossfades, opacity only                                       |

The Emphasized full curve is a two-segment spline not expressible as a single `cubic-bezier`. On the web, use Emphasized Decelerate for enter and Emphasized Accelerate for exit.

**Duration buckets** (16 tokens, Short 1 = 50ms through Extra Long 4 = 1000ms). Rule: duration grows with the spatial size of the animation.

- Micro-interactions (hover, state layer ripple): Short (50-200ms), Standard easing.
- Component transitions (expand a card, open a menu): Medium (250-400ms), Emphasized easing.
- Navigation / full-screen container transforms: Long (450-600ms), Emphasized easing.

Full token table in `references/color-and-tokens.md`.

---

## State layers

State layers are semi-transparent overlays that communicate interaction. The overlay color is always the component's **content (`on-*`) color** — only opacity varies.

| State   | Opacity |
| ------- | ------- |
| Enabled | 0%      |
| Hover   | 8%      |
| Focus   | 10%     |
| Pressed | 10%     |
| Dragged | 16%     |

Only one state layer applies at a time. The overlay sits between container and content in the z-stack.

Disabled is a separate mechanism (not an overlay): content (text/icon) at **38% opacity**, container (background) at **12% opacity**. Disabled elements are exempt from contrast ratio requirements.

Implementation on the web: `md-ripple` from `@material/web` handles the pressed/dragged ripple. For hover and focus, apply a pseudo-element with `background: var(--md-sys-color-on-<role>); opacity: 0.08` (hover) or `0.10` (focus), `border-radius` matching the container.

---

## Decision checklist

Run this before finalizing any UI surface:

1. **Color**: Am I using a role name (`--md-sys-color-primary`) or a raw hex? Use the role. If a role does not exist for the semantic need, look at the full list in `references/color-and-tokens.md` before inventing a value.
2. **Content on container**: Is every text/icon color the `on-*` pair of its background role? (`on-primary` on `primary`, `on-surface` on `surface`, etc.)
3. **Hierarchy**: Am I reaching for borders/shadows by reflex? Try `surface-container-low` → `surface-container` → `surface-container-high` → `surface-container-highest` first.
4. **Touch targets**: Is every interactive element at least **48x48dp**? If the visual size is smaller (e.g. a 24dp icon), add transparent padding. Minimum **8dp gap** between adjacent targets.
5. **Spacing**: Are all margins, gaps, and padding multiples of **4dp** (prefer multiples of 8dp for larger gaps)?
6. **Dark mode**: Have I provided both light and dark `--md-sys-color-*` values, or am I relying on a dynamic-color loader? Roles handle dark mode automatically when the correct scheme is injected.
7. **Type scale**: Am I using a named style (Body Large, Label Medium) rather than a one-off font size? Map to the 15-style scale.
8. **Motion**: Is the animation purposeful — does it communicate a state change or spatial relationship? If it is decorative only, cut it or reduce duration to Short 1-2.
9. **State layers**: Do all interactive elements implement hover (8%), focus (10%), pressed (10%) overlays using the correct `on-*` color?
10. **Accessibility**: WCAG contrast — body text 4.5:1, large text and UI controls 3:1. Do not encode information with color alone; pair with icon, label, or shape.

---

## M3 Expressive caveat

M3 Expressive (announced May 2025, shipping Android 16 / Wear OS 6) brings spring physics, shape morphing, new type roles, and new components (split buttons, button groups, FAB menus, loading indicators). **None of these are implemented in `@material/web` in 2026.** The web platform uses the tokenized M3 classic foundations: color roles, type scale, shape scale, easing/duration motion, and state layers. Do not promise Expressive features for web deliverables.

---

## Going deeper

- **Exact numeric tables** (all 47+ color roles, full 15-row type scale, elevation table, shape scale, motion easing and duration tables, state-layer opacity table): `references/color-and-tokens.md`
- **What "Google-style" means in practice** (restraint, spacing, Roboto, Material Symbols, purposeful motion, tonal contrast as accessibility): `references/google-style.md`
- Responsive layouts, window size classes, canonical patterns: sibling skill `responsive-adaptive`
- Spec checking an existing design against M3 rules: sibling skill `m3-spec-check`
- Scaffolding a new M3 UI from a template: sibling skill `m3-template`
- Deep-dive on a specific M3 component (`md-button`, `md-dialog`, `md-navigation-bar`, etc.): sibling skill `m3-component`
