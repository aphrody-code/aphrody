# Google-style UI — what "on-brand Material" means in practice

This reference answers the question "does this look like Google?" in operational terms. It is a companion to `../SKILL.md` and `color-and-tokens.md`. All guidance is grounded in M3 foundations — not aesthetic opinion.

---

## 1. Clarity and restraint

Google-style UI is legible before it is beautiful. The hierarchy is always readable at a glance: one primary action per screen, secondary elements subordinated, decoration absent unless it carries meaning.

Rules to enforce:

- **One primary accent per screen.** `primary` appears on the most important interactive element (the main CTA, the active state). Using `primary` on three competing elements removes its signal value — demote the others to `secondary`, `tertiary`, or an unaccented surface.
- **No decorative gradients.** M3 does not use gradient fills for surfaces or containers. Tonal elevation via `surface-container-*` roles replaces visual layering. If a gradient appears in your design, audit whether it is carrying a semantic meaning that a proper role could convey instead.
- **No shadow by reflex.** Shadows indicate that a surface floats above another; reserve them for modals, FABs, and menus. Do not apply shadows to cards on a contrasting surface — the `surface-container-*` hierarchy is sufficient.
- **Remove borders unless they carry semantic weight.** M3 uses `outline` and `outline-variant` for input field strokes and dividers, not for card containment. Cards that need separation rely on `surface-container` vs `surface-container-high` contrast, not a border.

---

## 2. Spacing on the 4dp grid

All spacing decisions — margin, padding, gap, icon size, component height — must be multiples of **4dp**. The preferred granularity in practice is **8dp** for component-level spacing and layout gaps. Tighter spacings (4dp, 12dp) are acceptable for intra-component density.

Quick reference:

- Icon with label: 8dp gap
- Intra-card padding: 16dp
- List item padding: 16dp horizontal, 12-16dp vertical
- Section spacing: 24dp or 32dp
- Page margin Compact: 16dp; Medium/Expanded: 24dp+

Never use odd numbers (5dp, 7dp, 15dp). If the visual design requires a 6dp value, round to 4dp or 8dp and adjust proportions.

---

## 3. Surface hierarchy instead of borders

The `surface-container-*` scale creates visual separation without borders or shadows. Learn the five levels:

| Role                        | When to reach for it                                |
| --------------------------- | --------------------------------------------------- |
| `surface-container-lowest`  | Inset or recessed content; the "hole" in the layout |
| `surface-container-low`     | Secondary cards, side panels                        |
| `surface-container`         | Default card background                             |
| `surface-container-high`    | Popover, elevated card, selected list item          |
| `surface-container-highest` | Top app bar, tooltip, chip                          |

The background (`surface` / `background`) is the baseline. Each step up the scale makes the surface more prominent visually. In dark mode, the tone moves in the opposite direction automatically — this is built into the scheme and requires no additional code.

When you use these roles correctly, **dark mode is free**: you never write `@media (prefers-color-scheme: dark)` overrides for surface colors.

---

## 4. Typography: Roboto and its variants

The canonical M3 typeface is **Roboto**. For web delivery, prefer **Roboto Flex** (variable font) — one file provides the full weight/width range without multiple font requests.

Load via Google Fonts:

```html
<link rel="preconnect" href="https://fonts.gstatic.com" crossorigin />
<link
  href="https://fonts.googleapis.com/css2?family=Roboto+Flex:opsz,wght@8..144,100..900&display=swap"
  rel="stylesheet"
/>
```

Or self-host the woff2 with `@font-face` for performance and offline support.

**Google Sans** (also known as Product Sans for Google branding) appears in Google's own product UIs (Search, Gmail, Drive) but is **not publicly licensed**. Do not use it in third-party M3 implementations. Roboto is the correct public typeface.

Type scale discipline:

- Pick one style per semantic level in your hierarchy. Avoid mixing Display Large with Body Small in the same card — the scale is self-consistent.
- Weight 400 for reading, weight 500 for labels and interactive elements. Do not use weight 700 or bold for M3 UI copy; that is M2 behavior.
- See `color-and-tokens.md` for the full 15-row table with exact px, line-height, and tracking.

---

## 5. Material Symbols for iconography

M3 uses **Material Symbols** — the variable font successor to Material Icons. Material Symbols exposes four axes:

| Axis | CSS token        | Range     | Effect                                 |
| ---- | ---------------- | --------- | -------------------------------------- |
| FILL | `--md-icon-fill` | 0 – 1     | Outline (0) to filled (1)              |
| wght | `--md-icon-wght` | 100 – 700 | Stroke weight; match body text weight  |
| GRAD | `--md-icon-grad` | -50 – 200 | Subtle contrast adjustment             |
| opsz | `--md-icon-opsz` | 20 – 48   | Optical sizing; match icon render size |

In `@aphrody-code/material-web`, the `md-icon` component inherits these as CSS custom properties. Load the font via the project's helper:

```js
import { ensureMaterialSymbols } from "@aphrody-code/material-web/icon/material-symbols.js";

ensureMaterialSymbols({ iconNames: ["search", "close", "home"] });
// Loads a variable font subset from Google Fonts CDN covering the requested glyphs
```

Or for self-hosted woff2:

```js
import { ensureMaterialSymbolsFontFace } from "@aphrody-code/material-web/icon/material-symbols.js";
ensureMaterialSymbolsFontFace("/fonts/MaterialSymbolsOutlined.woff2");
```

Load the **variable range** axes (e.g. `wght` 100..700, not `wght` 400) — a fixed-instance font ignores FILL/wght axis overrides entirely.

Icon sizing: default is 24dp. Match `--md-icon-opsz` to the rendered pixel size for optimal optical quality.

**Brand logos are absent from Material Symbols by Google's policy.** Do not use `md-icon` with text like "google" or "youtube" expecting a logo glyph — those are not in the Symbols set. Use `<img>` or SVG for brand logos.

Icon validation: `@aphrody-code/eslint-plugin-m3`'s `m3/valid-icon-name` rule validates icon names against the full 4253-glyph dataset at lint time.

---

## 6. Color: one accent, many surfaces

The signature of Google-style UI is that `primary` is used **sparingly**. Most of the interface is neutral — various shades of `surface-container-*` with `on-surface` text. Primary appears only where the user's attention should be directed to an action or state.

Patterns to avoid:

- `primary` background on a navigation bar (use `surface-container` or `secondary-container` for the active chip)
- `primary` text for body copy (use `on-surface`)
- All four accent families (`primary`, `secondary`, `tertiary`, `error`) visible simultaneously — one or two at most in normal UI
- Custom hex colors anywhere outside the scheme generation pipeline

Pattern to adopt: **every color in the UI has a name**. If you cannot point to the role in `color-and-tokens.md` for a color you are applying, it should not be there.

---

## 7. Motion: purposeful, not decorative

Google-style motion communicates spatial relationships and state changes. It is never ornamental.

Questions to ask before adding animation:

1. Does this motion tell the user where an element came from or where it went? (container transform, shared axis)
2. Does it acknowledge an interaction the user just performed? (ripple, state layer)
3. Does it signal a status change (loading, success, error)?

If the answer to all three is no, remove the animation or reduce it to a Short-duration opacity fade (50-100ms).

Specific patterns:

- **Micro-interactions** (button press, hover): state layer is instantaneous to the eye; do not add a CSS `transition` on the state layer opacity beyond Short 1 (50ms).
- **Component-level transitions** (opening a dialog, expanding an accordion): Medium 2-3 (300-350ms) + Emphasized Decelerate for enter, Emphasized Accelerate for exit.
- **Page/navigation transitions**: Long 1-2 (450-500ms) + Emphasized. These should feel deliberate and calm, not fast. Google's own apps use transitions in this range for content-to-content navigation.
- **Do not animate layout properties** (width, height) on the hot path — animate transform and opacity; these are GPU-composited and do not trigger layout.

---

## 8. Accessibility by construction

Correct use of M3's token system produces accessible UI without extra work. The mechanisms:

- **Tonal contrast**: HCT tone arithmetic ensures `on-X` colors meet WCAG ratios against `X` by default. Use the roles as designed and contrast is guaranteed.
- **State layers**: hover, focus, and pressed states are always visible because the overlay is relative to the element's content color — it has contrast by construction.
- **Touch targets**: 48x48dp minimum. Add transparent padding if the visual element is smaller. Never reduce targets for "aesthetic" reasons.
- **Disabled opacity**: 38% content / 12% container. These values are intentionally below WCAG thresholds — disabled elements are not interactive and are exempt.
- **Do not encode information with color alone.** Add a label, icon, or shape change alongside any color-based state. Example: an error field needs a red border (`error` role) AND an error icon AND a text message.

Contrast minimums (WCAG): 4.5:1 for body text, 3:1 for large text and UI controls.

---

## 9. What Google-style is NOT

To maintain clarity on scope:

- **It is not minimalism for its own sake.** M3 uses color, shape, and motion meaningfully; the restraint is semantic, not decorative emptiness.
- **It is not flat design.** Elevation, surface hierarchy, and state layers create a tangible sense of layering.
- **It is not M3 Expressive on the web.** Springs, shape morphing, and expressive type are Android-native features in 2026. Web deliverables use the tokenized M3 foundations.
- **It is not the same as MUI (Material UI for React).** MUI has its own theming API and component behaviors. This project (`@aphrody-code/m3-react`) is a direct wrapper over `@material/web` web components, not MUI.

---

## Back to SKILL.md

For the decision checklist and token usage directives, return to `../SKILL.md`. For exact numeric tables (color roles, type scale px, motion ms, state opacities), see `color-and-tokens.md`.
