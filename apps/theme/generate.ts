// SPDX-License-Identifier: Apache-2.0
//! Regenerate tokens.css from the aphrody CLI (the single source of truth).
//
// Requires `aphrody` on PATH (build + deploy it from the Rust repo at
// C:\src\aphrody). Run with: bun run generate (which also formats the output).

import { $ } from "bun";

const light = (await $`aphrody design tokens --fusion`.text()).trimEnd();
const dark = (await $`aphrody design tokens --dark`.text()).trim();

// The shadcn + Tailwind v4 alias blocks (in `light`) reference
// var(--md-sys-color-*), so only the M3 palette differs between light and dark.
const darkLines = dark.split("\n");
const darkPalette = darkLines.slice(1, -1).join("\n"); // strip ":root {" and "}"
const darkIndented = darkLines.map((line) => `  ${line}`).join("\n");

const out = `/* SPDX-License-Identifier: Apache-2.0 */
/*
 * Canonical aphrody UI tokens - the Material 3 fusion (M3 + shadcn/ui + Tailwind v4).
 * GENERATED - do not hand-edit. Source of truth: the aphrody CLI.
 *   aphrody design tokens --fusion   (light :root + shadcn + Tailwind aliases)
 *   aphrody design tokens --dark     (dark M3 palette, override below)
 * Regenerate: bun run generate
 */

${light}

/* Dark palette. The shadcn + Tailwind aliases above reference these vars, so
   only the M3 palette is re-declared. Automatic via prefers-color-scheme;
   force it with class="dark" on a root element. */
@media (prefers-color-scheme: dark) {
${darkIndented}
}

:root.dark,
.dark {
${darkPalette}
}
`;

await Bun.write(new URL("./tokens.css", import.meta.url), out);
console.log(`wrote tokens.css (${out.split("\n").length} lines)`);
