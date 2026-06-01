---
name: responsive-adaptive
description: >
  Teaches Claude how to apply Material Design 3 adaptive layout: window size
  classes, navigation pattern swaps, canonical layouts, grid/margin values, and
  touch-target rules. Invoke when the user says "make this responsive",
  "adaptive layout", "what breakpoint", "window size class", "navigation rail
  vs bar vs drawer", "list-detail layout", "how should this reflow on
  tablet/desktop", or "M3 layout".
---

# M3 Responsive and Adaptive Layout

## Responsive vs. Adaptive -- the M3 distinction

Responsive design scales one layout fluidly to any window width.
Adaptive design -- what M3 uses -- swaps structure, component choices, and
pane counts at discrete breakpoints. Both techniques apply simultaneously:
within a breakpoint, a pane stretches responsively; crossing a breakpoint
triggers an adaptive swap (e.g., bottom navigation bar becomes a navigation
rail).

Design for _available window width_, not for specific devices. The same phone
can land in any breakpoint depending on orientation, split-screen, or foldable
state. Never hardcode device names into layout decisions.

---

## Window size classes

M3 defines five width breakpoints. Copy these exact values -- do not invent
alternatives.

| Class       | dp range        | CSS min-width | Typical context                                  |
| ----------- | --------------- | ------------- | ------------------------------------------------ |
| Compact     | < 600 dp        | (default)     | Phone portrait                                   |
| Medium      | 600 -- 839 dp   | 600 px        | Small tablet, foldable portrait, phone landscape |
| Expanded    | 840 -- 1199 dp  | 840 px        | Tablet landscape, desktop                        |
| Large       | 1200 -- 1599 dp | 1200 px       | Desktop                                          |
| Extra-large | >= 1600 dp      | 1600 px       | Wide monitor, connected display                  |

Because 1 dp = 1 CSS px at 1× density (standard browser baseline), map dp
directly to px in media queries. Height breakpoints exist in Android but rarely
drive web layout decisions because content scrolls vertically.

### CSS media query template

```css
/* Compact: no query needed -- it is the default */

@media (min-width: 600px) {
  /* Medium */
}

@media (min-width: 840px) {
  /* Expanded */
}

@media (min-width: 1200px) {
  /* Large */
}

@media (min-width: 1600px) {
  /* Extra-large */
}
```

For component-level adaptivity (a card that reflowes based on its container,
not the viewport), prefer **container queries** over media queries. Apply
`container-type: inline-size` on the parent wrapper and mirror the same
breakpoint thresholds inside `@container` rules.

---

## Navigation pattern per size class

Swap navigation components as the window grows. Use the project's
`md-navigation-bar`, `md-navigation-rail`, and `md-navigation-drawer`
components (React: `MdNavigationBar`, `MdNavigationRail`, `MdNavigationDrawer`
from `@aphrody-code/m3-react`).

| Size class  | Navigation pattern                            | Notes                                                |
| ----------- | --------------------------------------------- | ---------------------------------------------------- |
| Compact     | `md-navigation-bar` (bottom navigation bar)   | Full width, 3-5 destinations                         |
| Medium      | `md-navigation-rail` (collapsed)              | Left side, icon + optional label                     |
| Expanded    | `md-navigation-rail` or modal drawer          | Rail default; promote to drawer if >= 5 destinations |
| Large       | `md-navigation-drawer` permanent              | Left side, always open, labels visible               |
| Extra-large | `md-navigation-drawer` permanent + multi-pane | Same, wider panes alongside it                       |

Rules:

- Never show a navigation bar AND a navigation rail simultaneously.
- Show or hide navigation components by toggling a CSS class on the shell
  element, not by mounting/unmounting in JS (avoid layout shift).
- FAB stays visible in all size classes; promote to extended FAB at Medium+
  when horizontal space permits.

---

## Canonical layouts and pane counts

| Size class  | Recommended pane count | Canonical layouts available             |
| ----------- | ---------------------- | --------------------------------------- |
| Compact     | 1                      | Single-pane (list OR detail, not both)  |
| Medium      | 1 or 2                 | 2 panes for list-detail at low density  |
| Expanded    | 1 or 2 (prefer 2)      | List-detail, supporting pane            |
| Large       | 1 or 2 (prefer 2)      | List-detail, supporting pane, dashboard |
| Extra-large | 1 to 3                 | Three-pane feeds, complex dashboards    |

Three canonical layouts:

1. **List-detail** -- explorable list in the primary pane, selected item
   detail in the secondary pane (e.g., email inbox + open thread). On
   Compact, show only one pane at a time, with back navigation.
2. **Supporting pane** -- main content (~2/3 width) plus a supporting panel
   (~1/3 width) for contextual content (e.g., editor + outline, map +
   search results).
3. **Feed** -- cards arranged in a configurable grid; column count grows with
   window width.

At Medium breakpoints, only use two panes for low-density content with clear
primary actions. Avoid two panes for dense, tabular information at Medium.

---

## Grid, margins, and gutters

The base grid unit is **4 dp**. All spacing values must be multiples of 4.
M3 Expressive promotes an 8 dp step for layout-level spacing.

| Size class | Margin (screen edge to content) | Gutter (between columns) |
| ---------- | ------------------------------- | ------------------------ |
| Compact    | 16 dp (16 px)                   | 8 -- 16 dp               |
| Medium     | 24 dp (24 px)                   | 16 -- 24 dp              |
| Expanded+  | 24 dp or more                   | 24 dp+                   |

Tune line length to 40--60 characters per line as the window grows. Do not let
text span the full viewport width on Large and Extra-large. Use max-width
constraints or multi-column layouts to limit measure.

### Tailwind utilities on the 4dp grid

Use Tailwind for host-level layout (flex/grid/gap/padding/margin on `<div>`
wrappers). Do NOT apply Tailwind classes to style the internals of `md-*`
components -- Shadow DOM blocks them. Tailwind utilities may be placed on the
host element of an `md-*` to affect its box model (width, margin).

```html
<!-- Correct: Tailwind on the wrapper, tokens inside the component -->
<div class="flex flex-col gap-4 px-4 md:px-6">
  <md-filled-button class="w-full">Submit</md-filled-button>
</div>
```

---

## Touch targets

Keep every interactive element at minimum **48 x 48 dp** on Compact. A visual
icon of 24 dp is acceptable if surrounded by transparent padding that brings the
interactive hit area to 48 x 48 dp. Maintain at least 8 dp between adjacent
touch targets to prevent mis-taps. On Medium+ (pointer-primary contexts), this
minimum may relax, but never remove it on touch-capable surfaces.

---

## Adaptive strategies for panes

When crossing a breakpoint, apply one or more of these strategies:

- **Reveal**: expose content hidden on smaller sizes (side panel, extended FAB
  label, inline metadata).
- **Divide**: split one pane into two as space allows.
- **Resize**: stretch pane widths; constrain text columns to 40--60 chars/line.
- **Reposition**: reflow elements from stacked to side-by-side, shift actions to
  a rail instead of a bottom bar.
- **Swap**: replace a component with a functionally equivalent one suited to the
  new size class (navigation bar ↔ rail ↔ drawer; bottom sheet ↔ dialog; full-
  screen dialog ↔ basic dialog). Only swap between functionally equivalent
  components -- never button ↔ chip.

---

## Decision checklist

Before laying out a screen, answer these questions:

1. What are the target size classes? (Compact-only? Compact through Expanded?)
2. Which navigation pattern does each size class use?
3. How many panes: 1 (Compact), 1-2 (Medium), 2 (Expanded/Large), 2-3 (XL)?
4. Which canonical layout fits: list-detail, supporting pane, or feed?
5. What adapts vs. what swaps? List all component substitutions.
6. Are all touch targets >= 48 dp on Compact?
7. Are margins 16 px on Compact, 24 px on Medium+?
8. Is line length constrained to 40--60 chars on Large/XL?
9. Are color tokens (`var(--md-sys-color-*)`) used -- no hardcoded colors?
10. Do RTL and LTR both work (navigation flips to the right side in RTL)?

---

## Project component map

| M3 element            | Web component                    | React wrapper (m3-react)      |
| --------------------- | -------------------------------- | ----------------------------- |
| Bottom navigation bar | `<md-navigation-bar>`            | `<MdNavigationBar>`           |
| Navigation rail       | `<md-navigation-rail>`           | `<MdNavigationRail>`          |
| Navigation drawer     | `<md-navigation-drawer>`         | `<MdNavigationDrawer>`        |
| Top app bar           | `<md-top-app-bar>`               | `<MdTopAppBar>`               |
| FAB                   | `<md-fab>` / `<md-extended-fab>` | `<MdFab>` / `<MdExtendedFab>` |

Use `@aphrody-code/m3-react` in React apps; use `@material/web` (via
`aphrody-components.ts`) in plain-web or Lit contexts.

---

## Related skills

- `m3-template`: invoke to scaffold a complete adaptive screen from a chosen
  canonical layout.
- `m3-design-guide`: color tokens, typography, elevation, and theming.
