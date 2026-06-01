---
name: m3-dynamic-color
description: >
  Generate and apply Material You dynamic color and M3 design tokens at runtime with
  @aphrody-code/m3-tokens. Use when asked to "theme from a seed color", "Material You
  runtime", "derive --md-sys-color-* roles", "re-theme live", "dark mode tokens",
  "convert a MUI theme to M3 tokens", "M3 breakpoints / window size classes", or
  "expose M3 tokens to Tailwind".
---

# m3-dynamic-color

`@aphrody-code/m3-tokens` is the pure-TS token engine of the monorepo. It has three
entry points — runtime Material You color, a MUI-theme converter, and the canonical
breakpoints — plus two prebuilt stylesheets. No DOM is required (usable from Node,
Bun, SSR, even React Native for the values).

## 1. Material You runtime — `dynamic-color`

Derive the ~49 `--md-sys-color-*` roles (light + dark) from any single seed via
`SchemeTonalSpot` (material-color-utilities).

```ts
import {
  schemeFromSeed, // (hex, { dark?, variant? }) -> { "--md-sys-color-primary": "#…", … }
  cssFromSeed, // (hex, opts) -> CSS string for :root / [data-theme=dark]
  applyDynamicColor, // (hex, { dark?, target? }) -> mutate inline styles live (no rebuild)
  clearDynamicColor, // (target?) -> remove the inline roles this module set
  ROLES, // the ~49 role names (camelCase)
} from "@aphrody-code/m3-tokens/dynamic-color";

// Re-theme every <md-*> on the page from a brand seed, current mode:
applyDynamicColor("#00658f");
applyDynamicColor("#00658f", { dark: true }); // dark variant
const css = cssFromSeed("#00658f"); // for SSR / a <style> tag
clearDynamicColor(); // revert to the stylesheet theme
```

This is the canonical way to implement a live seed/theme picker (the showcase does
exactly this). It works because every `md-*` component is self-contained on the
`--md-sys-color-*` custom properties — set the roles, the whole UI re-themes.

## 2. MUI theme -> M3 tokens — `theme-to-tokens`

Port a hand-rolled MUI `createTheme` palette to an M3 token stylesheet.

```ts
import {
  muiThemeToTokens,
  normalizeHex,
  type MuiTheme,
} from "@aphrody-code/m3-tokens/theme-to-tokens";

const { css, mcuAvailable } = await muiThemeToTokens(muiTheme, { dark: true });
// css = :root { --md-sys-color-*: … } derived from the MUI palette/typography/shape.
```

Use this in step 7 of a migration (`migrate-mui`) to seed the M3 theme from the
existing brand colors instead of re-picking them.

## 3. Breakpoints — `breakpoints` (platform-agnostic)

The single source of truth for the 5 M3 window size classes (compact / medium /
expanded / large / extra-large at 600 / 840 / 1200 / 1600 dp).

```ts
import {
  BREAKPOINTS,
  WINDOW_SIZE_CLASSES,
  classifyWidth, // (px) -> "compact" | "medium" | "expanded" | "large" | "extra-large"
  mediaQuery, // (sizeClass) -> "(min-width: …px)"
  marginFor, // (sizeClass) -> dp margin
  navigationFor, // (sizeClass) -> "bar" | "rail" | "rail-expanded"
  panesFor, // (sizeClass) -> 1 | 2 | 3
  breakpointsCss, // -> CSS custom properties for the breakpoints
} from "@aphrody-code/m3-tokens/breakpoints";
```

The React reactivity wrappers (`useWindowSizeClass`, `useMediaQuery`, `useAtLeast`)
live in `@aphrody-code/m3-react/adaptive` and re-export these. See the
`responsive-adaptive` skill for layout decisions.

## 4. Prebuilt stylesheets

- `@aphrody-code/m3-tokens/m3-tokens.css` — 62 runtime vars (color 49 roles +
  typescale, shape, motion easings, static elevation, state). Import this to expose
  the non-color tokens to Tailwind `@theme` (the lib itself only emits the color
  roles at runtime; the rest are compile-time Sass). See the `m3-tailwind` skill.
- `@aphrody-code/m3-tokens/breakpoints.css` — the breakpoint custom properties.

## Pitfalls

- Roles are camelCase in `ROLES`/`schemeFromSeed` keys map to kebab-case CSS vars
  (`onSurfaceVariant` -> `--md-sys-color-on-surface-variant`).
- `applyDynamicColor` sets INLINE styles on the target (default `:root`); it wins
  over the stylesheet. Call `clearDynamicColor()` to fall back to the sheet.
- material-color-utilities 0.4.0 has an ESM specifier bug patched via `bun patch`
  in this repo — in a consumer project, ensure the patched/fixed MCU is used or the
  import breaks under native Node.

Verify in-repo: `cd packages/m3-tokens && bun run demo` (seed picker) and `bun test`.
