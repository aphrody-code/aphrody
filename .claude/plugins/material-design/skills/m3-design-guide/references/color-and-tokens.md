# Color, tokens, and numeric values — M3 reference

This file is the "look up exact numbers" companion to `../SKILL.md`. All values are copied from the authoritative foundations document at `/home/ubuntu/aphrody/packages/material-web/docs/01-md3-spec-foundations.md` and attributed to [m3.material.io](https://m3.material.io/) and the official Material repositories.

---

## Color roles (complete list)

### Accent families (x4: primary, secondary, tertiary, error)

Each accent family exposes four roles. The table uses primary as the example; secondary, tertiary, and error follow the same pattern with their own tonal palettes.

| Role                     | Typical tone (light / dark) | Semantic use                                         |
| ------------------------ | --------------------------- | ---------------------------------------------------- |
| `primary`                | 40 / 80                     | Main accent; FABs, filled buttons, active indicators |
| `on-primary`             | 100 / 20                    | Content on `primary` (text, icons)                   |
| `primary-container`      | 90 / 30                     | Less emphatic tonal container                        |
| `on-primary-container`   | 30→10 / 90                  | Content on `primary-container`                       |
| `secondary`              | 40 / 80                     | Supporting accent                                    |
| `on-secondary`           | 100 / 20                    | Content on `secondary`                               |
| `secondary-container`    | 90 / 30                     | Supporting tonal container                           |
| `on-secondary-container` | 10 / 90                     | Content on `secondary-container`                     |
| `tertiary`               | 40 / 80                     | Complementary/contrast accent, used sparingly        |
| `on-tertiary`            | 100 / 20                    | Content on `tertiary`                                |
| `tertiary-container`     | 90 / 30                     | Tertiary tonal container                             |
| `on-tertiary-container`  | 10 / 90                     | Content on `tertiary-container`                      |
| `error`                  | 40 / 80                     | Error states, destructive actions                    |
| `on-error`               | 100 / 20                    | Content on `error`                                   |
| `error-container`        | 90 / 30                     | Error tonal container                                |
| `on-error-container`     | 10 / 90                     | Content on `error-container`                         |

### Surface and neutral roles

| Role                        | Semantic use                                       |
| --------------------------- | -------------------------------------------------- |
| `surface`                   | Default background for cards, sheets, dialogs      |
| `surface-dim`               | Darkest surface variant (light scheme)             |
| `surface-bright`            | Brightest surface variant (light scheme)           |
| `surface-container-lowest`  | Lowest container; most contrast against background |
| `surface-container-low`     | Low container                                      |
| `surface-container`         | Default container                                  |
| `surface-container-high`    | High container                                     |
| `surface-container-highest` | Highest container; most prominent without accent   |
| `surface-variant`           | Legacy; use `surface-container-*` in new work      |
| `on-surface`                | Primary text and icons on any surface              |
| `on-surface-variant`        | Secondary/subdued text and icons, icon outlines    |
| `outline`                   | Strong borders, separators requiring high contrast |
| `outline-variant`           | Subtle decorative borders, dividers                |
| `background`                | Page-level backdrop (usually equals `surface`)     |
| `on-background`             | Content on `background`                            |

### Inverse and utility roles

| Role                 | Semantic use                                |
| -------------------- | ------------------------------------------- |
| `inverse-surface`    | Inverted surface (e.g. snackbar background) |
| `inverse-on-surface` | Content on `inverse-surface`                |
| `inverse-primary`    | Primary readable on `inverse-surface`       |
| `surface-tint`       | Elevation tint color (equals `primary`)     |
| `scrim`              | Opaque veil behind modals and bottom sheets |
| `shadow`             | Drop shadow color                           |

---

## Type scale — 15 styles

Source: `docs/01-md3-spec-foundations.md` §4, attributed to [m3.material.io/styles/typography](https://m3.material.io/styles/typography/applying-type).

Default typeface: **Roboto**. Only two weights in the base scale: **400 (Regular)** and **500 (Medium)**.

| Style           | Size (px) | Line height (px) | Weight | Tracking (letter-spacing) |
| --------------- | --------- | ---------------- | ------ | ------------------------- |
| Display Large   | 57        | 64               | 400    | -0.25 px                  |
| Display Medium  | 45        | 52               | 400    | 0 px                      |
| Display Small   | 36        | 44               | 400    | 0 px                      |
| Headline Large  | 32        | 40               | 400    | 0 px                      |
| Headline Medium | 28        | 36               | 400    | 0 px                      |
| Headline Small  | 24        | 32               | 400    | 0 px                      |
| Title Large     | 22        | 28               | 400    | 0 px                      |
| Title Medium    | 16        | 24               | 500    | 0.15 px                   |
| Title Small     | 14        | 20               | 500    | 0.1 px                    |
| Body Large      | 16        | 24               | 400    | 0.5 px                    |
| Body Medium     | 14        | 20               | 400    | 0.25 px                   |
| Body Small      | 12        | 16               | 400    | 0.4 px                    |
| Label Large     | 14        | 20               | 500    | 0.1 px                    |
| Label Medium    | 12        | 16               | 500    | 0.5 px                    |
| Label Small     | 11        | 16               | 500    | 0.5 px                    |

Web CSS token pattern (example):

```css
--md-sys-typescale-body-large-font: "Roboto", sans-serif;
--md-sys-typescale-body-large-size: 1rem; /* 16px */
--md-sys-typescale-body-large-line-height: 1.5rem; /* 24px */
--md-sys-typescale-body-large-weight: 400;
--md-sys-typescale-body-large-tracking: 0.5px;
```

Note: in `@material/web`, these typescale tokens are resolved at Sass compile time, not as live runtime CSS custom properties.

---

## Elevation levels

Source: `docs/01-md3-spec-foundations.md` §5, attributed to [m3.material.io/styles/elevation](https://m3.material.io/styles/elevation/applying-elevation).

| Level   | dp    | Surface tint (indicative) | Typical components                   |
| ------- | ----- | ------------------------- | ------------------------------------ |
| Level 0 | 0 dp  | 0%                        | Flat filled buttons, outlined cards  |
| Level 1 | 1 dp  | ~5%                       | Elevated cards, bottom sheets        |
| Level 2 | 3 dp  | ~8%                       | Navigation bar, menus                |
| Level 3 | 6 dp  | ~11%                      | FAB, dialogs                         |
| Level 4 | 8 dp  | ~12%                      | Navigation drawers, transient states |
| Level 5 | 12 dp | ~14%                      | Maximum elevation                    |

Prefer `surface-container-*` roles for elevation-based hierarchy; they adapt automatically to dark mode without dynamic tint overlays.

---

## Shape scale

Source: `docs/01-md3-spec-foundations.md` §6, attributed to [m3.material.io/styles/shape](https://m3.material.io/styles/shape/corner-radius-scale).

Classic M3 scale (7 tokens):

| Token       | Corner radius | Web CSS token                             |
| ----------- | ------------- | ----------------------------------------- |
| None        | 0 dp          | `--md-sys-shape-corner-none: 0px`         |
| Extra Small | 4 dp          | `--md-sys-shape-corner-extra-small: 4px`  |
| Small       | 8 dp          | `--md-sys-shape-corner-small: 8px`        |
| Medium      | 12 dp         | `--md-sys-shape-corner-medium: 12px`      |
| Large       | 16 dp         | `--md-sys-shape-corner-large: 16px`       |
| Extra Large | 28 dp         | `--md-sys-shape-corner-extra-large: 28px` |
| Full        | 9999 dp       | `--md-sys-shape-corner-full: 9999px`      |

M3 Expressive adds intermediate tokens (Large Increased 20dp, Extra Large Increased 32dp, Extra Extra Large 48dp) and shape morphing. Not available on `@material/web` in 2026.

---

## Motion easing tokens

Source: `docs/01-md3-spec-foundations.md` §7, attributed to [m3.material.io/styles/motion/easing-and-duration](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs).

| Token                 | CSS value                           | Use case                                                         |
| --------------------- | ----------------------------------- | ---------------------------------------------------------------- |
| Standard              | `cubic-bezier(0.2, 0, 0, 1)`        | Transitions beginning and ending on screen                       |
| Standard Decelerate   | `cubic-bezier(0, 0, 0, 1)`          | Elements entering the screen                                     |
| Standard Accelerate   | `cubic-bezier(0.3, 0, 1, 1)`        | Elements exiting the screen                                      |
| Emphasized            | Two-segment spline (see note below) | Prominent on-screen transitions                                  |
| Emphasized Decelerate | `cubic-bezier(0.05, 0.7, 0.1, 1)`   | Primary content entering; use this on web for "Emphasized" enter |
| Emphasized Accelerate | `cubic-bezier(0.3, 0, 0.8, 0.15)`   | Primary content exiting; use this on web for "Emphasized" exit   |
| Linear                | `cubic-bezier(0, 0, 1, 1)`          | Crossfades, opacity-only transitions                             |

> The Emphasized full curve is a two-segment path: `M 0,0 C 0.05,0 0.133333,0.06 0.166666,0.4 C 0.208333,0.82 0.25,1 1,1`. It cannot be expressed as a single `cubic-bezier`. On the web, approximate it using Emphasized Decelerate (enter) and Emphasized Accelerate (exit), or implement via a `@keyframes` block following the path.

---

## Motion duration tokens

16 tokens in four buckets. Rule: longer duration for larger spatial area of animation.

| Token    | ms  | Token        | ms   |
| -------- | --- | ------------ | ---- |
| Short 1  | 50  | Long 1       | 450  |
| Short 2  | 100 | Long 2       | 500  |
| Short 3  | 150 | Long 3       | 550  |
| Short 4  | 200 | Long 4       | 600  |
| Medium 1 | 250 | Extra Long 1 | 700  |
| Medium 2 | 300 | Extra Long 2 | 800  |
| Medium 3 | 350 | Extra Long 3 | 900  |
| Medium 4 | 400 | Extra Long 4 | 1000 |

Typical pairings:

- Micro-interactions (hover feedback, ripple): Short 1-4 + Standard
- Component transitions (expand, open menu): Medium 1-4 + Emphasized Decelerate/Accelerate
- Navigation / container transform: Long 1-4 + Emphasized

---

## State-layer opacity table

Source: `docs/01-md3-spec-foundations.md` §8, attributed to [m3.material.io/foundations/interaction/states](https://m3.material.io/foundations/interaction/states/state-layers).

State layer color = the component's `on-*` content color (never a fixed gray).

| State   | Overlay opacity |
| ------- | --------------- |
| Enabled | 0%              |
| Hover   | 8%              |
| Focus   | 10%             |
| Pressed | 10%             |
| Dragged | 16%             |

Disabled (not an overlay — direct opacity on the element):

| Disabled element             | Opacity |
| ---------------------------- | ------- |
| Content (text, icons)        | 38%     |
| Container (background, fill) | 12%     |

Disabled elements are exempt from WCAG contrast requirements.

---

## Accessibility ratios

Source: `docs/01-md3-spec-foundations.md` §10.

| Element                                         | Minimum contrast ratio (WCAG) |
| ----------------------------------------------- | ----------------------------- |
| Normal / body text                              | 4.5:1                         |
| Large text (>=18pt regular or >=14pt bold)      | 3:1                           |
| Non-text UI elements (icons, borders, controls) | 3:1                           |

Touch target minimums: 48x48dp per target, with at least 8dp gap between adjacent targets. For pointer (mouse/trackpad) input the target may be smaller.

---

## Window size classes (breakpoints)

Source: `docs/01-md3-spec-foundations.md` §9.2. Translate dp to px for CSS `min-width` media queries.

| Class       | Width       | Typical targets                            | Recommended navigation                   |
| ----------- | ----------- | ------------------------------------------ | ---------------------------------------- |
| Compact     | < 600dp     | Phones portrait                            | Bottom navigation bar                    |
| Medium      | 600-839dp   | Small tablets, foldables, phones landscape | Navigation rail                          |
| Expanded    | 840-1199dp  | Tablets, desktops                          | Navigation rail or drawer                |
| Large       | 1200-1599dp | Large desktops                             | Permanent navigation drawer + multi-pane |
| Extra-large | >=1600dp    | Very large displays                        | Permanent drawer + multi-pane            |

---

## Sources

- [m3.material.io](https://m3.material.io/) — official Material Design 3 site
- [m3.material.io/styles/color](https://m3.material.io/styles/color/system/how-the-system-works) — color system
- [m3.material.io/styles/typography](https://m3.material.io/styles/typography/applying-type) — type scale
- [m3.material.io/styles/elevation](https://m3.material.io/styles/elevation/applying-elevation) — elevation
- [m3.material.io/styles/shape](https://m3.material.io/styles/shape/corner-radius-scale) — shape
- [m3.material.io/styles/motion/easing-and-duration](https://m3.material.io/styles/motion/easing-and-duration/tokens-specs) — motion
- [m3.material.io/foundations/interaction/states](https://m3.material.io/foundations/interaction/states/state-layers) — state layers
- [github.com/material-foundation/material-tokens](https://github.com/material-foundation/material-tokens/blob/main/tokens.md) — token reference
- Project foundations doc: `docs/01-md3-spec-foundations.md`
