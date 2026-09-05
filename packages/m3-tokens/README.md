<!-- SPDX-License-Identifier: Apache-2.0 -->

# @aphrody/m3-tokens

Material Design 3 design tokens for the web, in TypeScript. Two capabilities:

1. **`theme-to-tokens`** — convert a MUI (`createTheme`) theme into a block of
   M3 `--md-sys-*` token CSS. The entry point of the MUI → Material 3 migration.
2. **`dynamic-color`** — runtime **Material You**: derive the full set of M3
   colour roles (light **and** dark) from any single seed colour, and apply them
   live. Seven scheme variants, configurable contrast level. WCAG **AA** by
   default, **AAA** in high-contrast mode (proven by the package's contrast test
   suite, across many seeds, all variants, and both modes).

Built on [`@material/material-color-utilities`](https://www.npmjs.com/package/@material/material-color-utilities).

## Install

```bash
bun add @aphrody/m3-tokens
```

## Material You — `dynamic-color`

```ts
import {
  schemeFromSeed,
  cssFromSeed,
  applyDynamicColor,
  clearDynamicColor,
  type SchemeVariant,
} from "@aphrody/m3-tokens/dynamic-color";

// 1. A role map for one mode: { "--md-sys-color-primary": "#…", … } (~47 roles)
const light = schemeFromSeed("#00658f");
const dark = schemeFromSeed("#00658f", { dark: true });

// Scheme variant (default: "tonalSpot" — the canonical M3/Android 12 scheme)
const expressive = schemeFromSeed("#00658f", { variant: "expressive" });
const mono = schemeFromSeed("#00658f", { variant: "monochrome" });

// Contrast level (-1..1 ; 0 = standard / AA, 0.5 = medium, 1 = maximum / AAA)
const hc = schemeFromSeed("#00658f", { contrastLevel: 1 });

// All options combined
const custom = schemeFromSeed("#00658f", {
  dark: true,
  variant: "vibrant",
  contrastLevel: 0.5,
});

// 2. A full stylesheet for SSR / static theming (`:root` + `[data-theme=dark]`)
const css = cssFromSeed("#00658f");
const cssMono = cssFromSeed("#00658f", { variant: "monochrome" });

// 3. Apply live in the browser (overrides :root inline on <html>)
applyDynamicColor("#00658f", { dark: matchMedia("(prefers-color-scheme: dark)").matches });
applyDynamicColor("#00658f", { variant: "expressive", contrastLevel: 1 });
clearDynamicColor(); // revert to the stylesheet baseline
```

Because `@aphrody/material-web` only emits `--md-sys-color-*` at runtime, setting those
roles re-themes **every** `<md-*>` element (and every `@aphrody/m3-react`
wrapper) with no rebuild — the signature Material 3 capability.

### Scheme variants

| `variant`    | Character                                                                           |
| ------------ | ----------------------------------------------------------------------------------- |
| `tonalSpot`  | **Default** — canonical M3 (Android 12+): low–medium chroma, complementary tertiary |
| `content`    | Source colour in `primaryContainer`, high fidelity to the input colour              |
| `fidelity`   | Like `content` but tertiary from temperature complement                             |
| `expressive` | Intentionally detached from source — bold, diverse, expressive palette              |
| `vibrant`    | Maximum chroma at every tonal position                                              |
| `neutral`    | Near-greyscale, very low chroma                                                     |
| `monochrome` | Pure greyscale — zero chroma                                                        |

### Contrast levels

| `contrastLevel` | Effect                                     |
| --------------- | ------------------------------------------ |
| `-1`            | Reduced contrast (below M3 standard)       |
| `0`             | **Default** — M3 standard (WCAG AA, 4.5:1) |
| `0.5`           | Medium contrast                            |
| `1`             | Maximum contrast (WCAG AAA, 7:1)           |

> **Legacy alias**: `contrast` is still accepted for backwards compatibility.
> When both `contrast` and `contrastLevel` are provided, `contrastLevel` takes
> precedence.

### API

```ts
// Types
type SchemeVariant =
  | "tonalSpot"
  | "content"
  | "fidelity"
  | "expressive"
  | "vibrant"
  | "neutral"
  | "monochrome";

interface SchemeOptions {
  dark?: boolean; // default false
  variant?: SchemeVariant; // default "tonalSpot"
  contrastLevel?: number; // default 0  (-1..1)
  contrast?: number; // @deprecated alias for contrastLevel
}

interface ApplyOptions extends SchemeOptions {
  target?: HTMLElement | null; // default document.documentElement
}

// Functions
function schemeFromSeed(hex: string, opts?: SchemeOptions): Record<string, string>;
function cssFromSeed(hex: string, opts?: SchemeOptions): string;
function applyDynamicColor(hex: string, opts?: ApplyOptions): void;
function clearDynamicColor(target?: HTMLElement | null): void;

// Constants
const ROLES: readonly string[]; // ~47 camelCase role names
```

## MUI → M3 — `theme-to-tokens`

```ts
import { muiThemeToTokens } from "@aphrody/m3-tokens/theme-to-tokens";

const { css, lightRoles, darkRoles, typescale, shape } = await muiThemeToTokens(muiTheme, {
  darkTheme: muiDarkTheme,
});
```

Maps the MUI palette (and honours explicit `primary.main` / `contrastText`),
derives the M3 roles MUI lacks (tertiary, `*-container`, surface-variant,
outline, inverse-\*), and maps typography variants and `shape.borderRadius` to
the M3 typescale and corner family.

CLI (bun):

```bash
bun run demo            # from a realistic MUI theme
bun run demo '#6750A4'  # from a Material You seed colour
```

## Runtime token asset

`@aphrody/m3-tokens/m3-tokens.css` declares the M3 token families
`@aphrody/material-web` does **not** project as runtime variables (typescale, shape,
elevation, motion, state) on `:root` — useful for Tailwind `@theme` consumption.

## Toolchain

bun-only. Ships ESM `dist/` (`.js` + `.d.ts`) built with tsup + `tsc`.
