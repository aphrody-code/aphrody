<!-- SPDX-License-Identifier: Apache-2.0 -->

# @aphrody-code/m3-theme

Canonical **Material 3 fusion** tokens — one generated stylesheet
(`tokens.css`), light and dark, that maps the **shadcn/ui** and **Tailwind v4**
colour aliases onto the Material 3 system roles (`--md-sys-color-*`) emitted by
[`@aphrody-code/material-web`](../material-web). Import it once and style with
either vocabulary; no app hand-rolls colour values.

## What's in `tokens.css`

- the M3 system palette `--md-sys-color-*` (light `:root`, dark via
  `@media (prefers-color-scheme: dark)` **and** a `.dark` class for a forced
  theme);
- the **shadcn/ui** alias block (`--background`, `--primary`, `--border`, …)
  mapped onto the M3 roles;
- the **Tailwind v4** `@theme inline` block (`--color-background`, …).

Because the shadcn and Tailwind aliases reference `var(--md-sys-color-*)`, the
dark theme only re-declares the M3 palette — the aliases follow automatically.

## Use

```ts
// In any Bun/web app — Bun bundles the CSS into the page.
import "@aphrody-code/m3-theme/tokens.css";
```

```html
<!-- force the dark theme regardless of the OS preference -->
<html class="dark"></html>
```

Then style with the M3 roles (`background: var(--md-sys-color-surface)`) or the
shadcn aliases (`var(--background)` / `var(--primary)`).

## Regenerate

```sh
bun run generate     # regenerates tokens.css, then formats it (oxfmt)
```

`generate.ts` is self-contained and bun-native: the source of truth is the
Material 3 **baseline** palette (seed `#6750A4`) plus the shadcn/Tailwind alias
maps, both declared once in the script. The dark palette and both alias layers
are derived from those declarations so the sheet never drifts by hand. To theme
from a different seed, swap the palette for the output of
[`@aphrody-code/m3-tokens/dynamic-color`](../m3-tokens) `schemeFromSeed(seed, { dark })`.

## Dynamic Runtime Theme System

In addition to the static `tokens.css` stylesheet, `@aphrody-code/m3-theme` provides a fully dynamic runtime theme system for both JavaScript/TypeScript environments and React applications.

This system derives the complete set of Material 3 custom properties (`--md-sys-color-*`) dynamically from a single seed color, automatically maps the shadcn/ui and Tailwind v4 color variables, and manages light/dark/system mode.

### 1. Vanilla JS/TS: `applyDynamicFusionTheme`

To dynamically update the theme on the DOM without a full rebuild:

```ts
import { applyDynamicFusionTheme } from "@aphrody-code/m3-theme";

// Applies theme inline to document.documentElement (HTML element)
applyDynamicFusionTheme("#6750A4", {
  dark: false, // forces light mode (omitting resolves from DOM/OS preference)
  contrastLevel: 0.0, // standard contrast, ranges from -1.0 to 1.0
  variant: "tonalSpot", // "tonalSpot" | "content" | "fidelity" | "expressive" | "vibrant" | "neutral" | "monochrome"
});
```

Options:

- `dark`: Forces light/dark mode. If omitted, it will automatically detect the theme from the target's `.dark` class or the system's `(prefers-color-scheme: dark)` media query.
- `target`: The target `HTMLElement` (defaults to `document.documentElement`).
- `contrastLevel`: Color contrast offset in `[-1.0, 1.0]` (0.0 standard).
- `variant`: The dynamic scheme algorithm variant (defaults to `"tonalSpot"`).

### 2. React Hook & Provider: `M3ThemeProvider` & `useM3Theme`

Wrap your React app in the provider to handle user-facing theme configuration, light/dark/system mode, and seed color switching. This updates both `@aphrody-code/material-web` (Lit components) and Tailwind v4 / shadcn/ui (React layout components) synchronously.

```tsx
import { M3ThemeProvider, useM3Theme } from "@aphrody-code/m3-theme/react";

function App() {
  return (
    <M3ThemeProvider defaultSeedColor="#6750a4" defaultThemeMode="system">
      <ThemePicker />
      <YourMainAppLayout />
    </M3ThemeProvider>
  );
}

function ThemePicker() {
  const { themeMode, setThemeMode, seedColor, setSeedColor, resolvedTheme } = useM3Theme();

  return (
    <div className="p-4 flex gap-4">
      <select
        value={themeMode}
        onChange={(e) => setThemeMode(e.target.value as any)}
        className="border p-2 rounded"
      >
        <option value="light">Light</option>
        <option value="dark">Dark</option>
        <option value="system">System ({resolvedTheme})</option>
      </select>

      <input
        type="color"
        value={seedColor}
        onChange={(e) => setSeedColor(e.target.value)}
        className="w-12 h-10 border rounded"
      />
    </div>
  );
}
```

### 3. Tauri 2.0 Integration: `useM3TauriThemeSync` & `M3TauriTitlebar`

For React applications compiled into Tauri 2.0 desktop shells, this package provides seamless, zero-dependency integration helpers to manage native window styling:

*   **`useM3TauriThemeSync`**: A custom React hook that automatically synchronizes the M3 theme mode (light/dark) back to the Tauri native window context, allowing native context menus and dialog boxes to match your application's appearance.
*   **`M3TauriTitlebar`**: A highly polished, Material Design 3 custom titlebar component. It handles window dragging (`data-tauri-drag-region`), maximize/restore state, and window actions (minimize, maximize, close) without external library requirements, automatically hiding when the application runs in a normal web browser.

```tsx
import { M3ThemeProvider } from "@aphrody-code/m3-theme/react";
import { M3TauriTitlebar, useM3TauriThemeSync } from "@aphrody-code/m3-theme/tauri";

function MainAppLayout() {
  // Syncs M3 theme mode back to Tauri window
  useM3TauriThemeSync();

  return (
    <div className="flex flex-col h-screen">
      {/* Renders a custom M3 borderless titlebar for the desktop app */}
      <M3TauriTitlebar title="My Material App" logo={<AppLogo />} />
      <div className="flex-1 overflow-auto">
        {/* Your content */}
      </div>
    </div>
  );
}
```

## Relationship to `@aphrody-code/m3-tokens`

`m3-tokens` is the **engine** (MUI-theme → M3 token generation, runtime Material
You via `material-color-utilities`, the raw `--md-sys-color-*` and breakpoint
CSS). `m3-theme` is the **opinionated fusion sheet** built on top of it: the M3
palette plus the shadcn + Tailwind alias surface, ready to drop into a site that
mixes M3 web components with shadcn/Tailwind utilities.
