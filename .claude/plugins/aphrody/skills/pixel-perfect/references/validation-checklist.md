# pixel-perfect — validation checklist

Concrete pass/fail items the skill runs against a single component. The
order matches the workflow in `SKILL.md`. Each item has a severity
(P0 = must-fix, P1 = should-fix, P2 = nice-to-have) and a grep hint the
skill can execute mechanically.

---

## A. Wrapper integrity (P0)

- [ ] `tsx` file imports the matching `@material/web/<family>/<elem>.js`
      module. Hint:
      `Grep -n "@material/web/" packages/ui/components/<name>/*.tsx`
- [ ] No import from `@radix-ui/*`. Hint:
      `Grep -n "@radix-ui/" packages/ui/components/<name>/`
- [ ] No import from `class-variance-authority` or `cva`. Hint:
      `Grep -n "class-variance-authority\|from ['\"]cva" packages/ui/components/<name>/`
- [ ] No `tailwind-merge` / `clsx` chains that produce raw color or
      shape utility classes. Hint:
      `Grep -nE "(bg|text|border|rounded|shadow)-(\\[|[a-z]+-)" packages/ui/components/<name>/`
- [ ] Component exposes the variants listed in `m3-spec.md` for its
      family (e.g. button → filled/outlined/text/elevated/tonal/fab).

## B. Color tokens (P0)

For every CSS rule in the component:

- [ ] `color:` value is `var(--md-sys-color-on-*)` or `currentColor`.
- [ ] `background-color:` value is `var(--md-sys-color-{primary|secondary|tertiary|error|surface|surface-container*|inverse-surface})`
      or `transparent`.
- [ ] `border-color:` value is `var(--md-sys-color-outline*)` or an
      accent token.
- [ ] `fill:` / `stroke:` values resolve to a color token.
- [ ] No hex literals (`#[0-9a-fA-F]{3,8}`) and no `rgb(`/`rgba(`/`hsl(`
      / `hsla(` outside of fallback definitions in `tokens/m3.json`.
      Hint:
      `Grep -nE "#[0-9a-fA-F]{3,8}|rgba?\\(|hsla?\\(" packages/ui/components/<name>/`

## C. Typography tokens (P0)

- [ ] Every `font-family` resolves to `var(--md-sys-typescale-*-font)`.
- [ ] Every `font-size` resolves to `var(--md-sys-typescale-*-size)`.
- [ ] Every `font-weight` resolves to `var(--md-sys-typescale-*-weight)`.
- [ ] Every `letter-spacing` resolves to
      `var(--md-sys-typescale-*-tracking)`.
- [ ] Every `line-height` resolves to
      `var(--md-sys-typescale-*-line-height)`.
- [ ] No `'Roboto'` / `'Inter'` / `'system-ui'` literals (unless inside
      `tokens/m3.json` as fallback). Hint:
      `Grep -nE "font-family\\s*:\\s*['\"]" packages/ui/components/<name>/`

## D. Shape tokens (P1)

- [ ] Every `border-radius` resolves to
      `var(--md-sys-shape-corner-*)`.
- [ ] No raw `px`/`rem` literals on `border-radius`. Hint:
      `Grep -nE "border-radius\\s*:\\s*[0-9]" packages/ui/components/<name>/`

## E. Motion tokens (P1)

- [ ] Every `transition-duration` / `animation-duration` resolves to
      `var(--md-sys-motion-duration-*)`.
- [ ] Every `transition-timing-function` /
      `animation-timing-function` resolves to
      `var(--md-sys-motion-easing-*)`.
- [ ] No `cubic-bezier(`/`ease-in-out`/`ease`/`linear` literals
      (`linear` is allowed only if it maps to the token). Hint:
      `Grep -nE "cubic-bezier\\(|ease-in|ease-out|ease\\b" packages/ui/components/<name>/`

## F. Elevation tokens (P1)

- [ ] Every `box-shadow` consumes
      `var(--md-sys-elevation-level{0..5})` directly OR the component
      renders `<md-elevation>` as a child.
- [ ] No `box-shadow: 0px Xpx Ypx` literals. Hint:
      `Grep -nE "box-shadow\\s*:\\s*[0-9]" packages/ui/components/<name>/`

## G. State layers (P1)

- [ ] Component renders `<md-ripple>` (or is composed of a Material Web
      element that owns its own ripple).
- [ ] No naked `:hover { opacity: 0.X }` / `:focus { background: ... }`
      that bypasses the state layer system. Hint:
      `Grep -nE ":hover|:focus|:active" packages/ui/components/<name>/*.css`

## H. Spacing grid (P2)

- [ ] All `margin`/`padding`/`gap`/`top`/`left`/`right`/`bottom` values
      are multiples of 4px (or CSS vars that resolve to such).

## I. Token bundle freshness (P0)

- [ ] `packages/ui/tokens/m3.json` exists.
- [ ] All `--md-sys-*` tokens referenced by the component are present
      in `packages/ui/tokens/m3.json`.
      Hint: read the JSON, build a Set of keys, intersect with the
      tokens grepped from the component, list the diff.
- [ ] If anything is missing, the skill MUST instruct the user to run
      `/tokens` (or invoke it via the MCP tool) and re-run.

## J. Optional visual diff (P1)

- [ ] `mcp__bxc-scrapper__bxc_recon` returned a screenshot.
- [ ] `mcp__bxc-scrapper__vision_analyze` returned at least one of
      `{elements, text, colors, fonts, hierarchy}`.
- [ ] For each top-3 color extracted, the closest token in the
      component is within ΔE2000 ≤ 5.

---

## Report format

```
# pixel-perfect — <name>

Component path : packages/ui/components/<name>/Button.tsx
M3 family      : button
Material Web   : @material/web/button/filled-button.js
Variants found : filled, outlined, text, tonal
Token bundle   : packages/ui/tokens/m3.json (487 tokens, last refresh 2026-05-17)

## Violations

P0 (1)
  - [color] Button.tsx:42 hard-coded `#1976d2` — use `var(--md-sys-color-primary)`.
    Fix:
      - background-color: #1976d2;
      + background-color: var(--md-sys-color-primary);

P1 (2)
  - [shape] Button.tsx:51 border-radius: 16px — use `var(--md-sys-shape-corner-large)`.
  - [motion] Button.tsx:60 transition-duration: 200ms — use `var(--md-sys-motion-duration-short4)`.

P2 (0)

## Visual diff
ΔE2000 primary    : 0.4 (PASS)
ΔE2000 secondary  : 2.1 (PASS)
ΔE2000 error      : unaudited (vision_analyze returned BXC_UNAVAILABLE)
```

End the report with the one-line summary used by the skill exit:

```
pixel-perfect: button — 1 P0 / 2 P1 / 0 P2
```
