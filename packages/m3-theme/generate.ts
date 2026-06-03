// SPDX-License-Identifier: Apache-2.0
//! Regenerate tokens.css — the canonical Material 3 fusion sheet.
//
// Self-contained and bun-native: no external CLI. The source of truth is derived
// using native Rust FFI from the Material Design 3 *baseline* palette seed (#6750A4)
// or a custom seed via process.env.M3_SEED.
//
// Run with: bun run generate   (also formats the output via oxfmt).

import { argbToHct, deriveScheme } from "@aphrody/bun-rs";

/** One M3 system colour role: its `--md-sys-color-<name>` light + dark hex. */
type Role = readonly [name: string, light: string, dark: string];

// Resolve the seed color (default to M3 baseline #6750A4)
const seed = process.env.M3_SEED || "#6750a4";
const argb = seed.startsWith("#")
  ? parseInt(seed.slice(1), 16) | 0xff000000
  : parseInt(seed, 16) | 0xff000000;

const hct = argbToHct(argb);
const lightMap = deriveScheme(hct.hue, hct.chroma, false);
const darkMap = deriveScheme(hct.hue, hct.chroma, true);

const roleNames = [
  "primary",
  "on-primary",
  "primary-container",
  "on-primary-container",
  "secondary",
  "on-secondary",
  "secondary-container",
  "on-secondary-container",
  "tertiary",
  "on-tertiary",
  "tertiary-container",
  "on-tertiary-container",
  "error",
  "on-error",
  "error-container",
  "on-error-container",
  "background",
  "on-background",
  "surface",
  "on-surface",
  "surface-variant",
  "on-surface-variant",
  "outline",
  "outline-variant",
  "shadow",
  "scrim",
  "inverse-surface",
  "inverse-on-surface",
  "inverse-primary",
  "surface-dim",
  "surface-bright",
  "surface-container-lowest",
  "surface-container-low",
  "surface-container",
  "surface-container-high",
  "surface-container-highest",
];

const PALETTE: readonly Role[] = roleNames.map((name) => {
  const light = lightMap[`--md-sys-color-${name}`];
  const dark = darkMap[`--md-sys-color-${name}`];
  return [name, light, dark] as const;
});

// shadcn/ui token -> M3 system role. The Tailwind v4 `@theme inline` block is
// derived from the same map (each `--<alias>` gets a `--color-<alias>` twin).
const ALIASES: readonly (readonly [alias: string, role: string])[] = [
  ["background", "surface"],
  ["foreground", "on-surface"],
  ["card", "surface-container-low"],
  ["card-foreground", "on-surface"],
  ["popover", "surface-container"],
  ["popover-foreground", "on-surface"],
  ["primary", "primary"],
  ["primary-foreground", "on-primary"],
  ["secondary", "secondary-container"],
  ["secondary-foreground", "on-secondary-container"],
  ["muted", "surface-variant"],
  ["muted-foreground", "on-surface-variant"],
  ["accent", "tertiary-container"],
  ["accent-foreground", "on-tertiary-container"],
  ["destructive", "error"],
  ["destructive-foreground", "on-error"],
  ["border", "outline-variant"],
  ["input", "outline-variant"],
  ["ring", "primary"],
];

const palette = (mode: "light" | "dark"): string =>
  PALETTE.map(
    ([name, light, dark]) => `    --md-sys-color-${name}: ${mode === "light" ? light : dark};`,
  ).join("\n");

const shadcnAliases = (): string =>
  ALIASES.map(([alias, role]) => `    --${alias}: var(--md-sys-color-${role});`).join("\n");

const tailwindTheme = (): string =>
  ALIASES.map(([alias, role]) => `    --color-${alias}: var(--md-sys-color-${role});`).join("\n");

const out = `/* SPDX-License-Identifier: Apache-2.0 */
/*
 * Canonical Material 3 fusion tokens (M3 + shadcn/ui + Tailwind v4), light + dark.
 * GENERATED - do not hand-edit. Source of truth: generate.ts (M3 baseline palette
 * + the shadcn/Tailwind alias maps). Regenerate with: bun run generate
 */

:root {
${palette("light")}
}

:root {
${shadcnAliases()}
}

@theme inline {
${tailwindTheme()}
}

/* Dark palette. The shadcn + Tailwind aliases above reference these vars, so
   only the M3 palette is re-declared. Automatic via prefers-color-scheme;
   force it with class="dark" on a root element. */
@media (prefers-color-scheme: dark) {
    :root {
${PALETTE.map(([name, , dark]) => `        --md-sys-color-${name}: ${dark};`).join("\n")}
    }
}

:root.dark,
.dark {
${palette("dark")}
}
`;

await Bun.write(new URL("./tokens.css", import.meta.url), out);
console.log(`wrote tokens.css (${out.split("\n").length} lines) from seed: ${seed}`);
