/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

/**
 * @fileoverview Helpers to register the **Material Symbols** icon font used by
 * `<md-icon>` (and every component that slots an icon). Mirrors the Google Sans
 * helpers in `typography/internal/font-face.ts`.
 *
 * `<md-icon>` only declares `font-family: var(--md-icon-font, 'Material Symbols
 * Outlined')` and the four variable axes (FILL/GRAD/opsz/wght) — it never loads
 * the font itself. The integrator must register it once. Three paths:
 *
 * 1. **Google Fonts CDN, variable ranges** (`materialSymbolsHref()`): loads the
 *    full axis ranges so the `--md-icon-fill/wght/grad/opsz` tokens act at
 *    runtime. Prefer this over a fixed instance (`@24,400,0,0`) which freezes
 *    FILL/wght.
 * 2. **Google Fonts CDN, subset** (`materialSymbolsHref({iconNames})`): adds
 *    `&icon_names=` so Google serves a glyph subset — the single biggest payload
 *    win when you know your icon set (e.g. the names a codemod collected).
 * 3. **Self-hosted woff2/ttf** (`materialSymbolsFontFaceCss(url)`): fully
 *    offline, no CDN dependency.
 */

/** The three Material Symbols styles. Outlined is the M3 default. */
export type MaterialSymbolsStyle = "Outlined" | "Rounded" | "Sharp";

/** The canonical font-family name for the default (Outlined) style. */
export const MATERIAL_SYMBOLS_FAMILY = "Material Symbols Outlined";

/** Variable-axis ranges of the Material Symbols variable font (per Google). */
export const MATERIAL_SYMBOLS_AXES = {
  /** optical size, dp. */
  opsz: [20, 48] as const,
  /** weight. */
  wght: [100, 700] as const,
  /** fill (0 = outline, 1 = filled). */
  FILL: [0, 1] as const,
  /** grade (emphasis, can be negative for dark UIs). */
  GRAD: [-50, 200] as const,
};

/** family name for a given style. */
export function materialSymbolsFamily(style: MaterialSymbolsStyle = "Outlined"): string {
  return `Material Symbols ${style}`;
}

export interface MaterialSymbolsHrefOptions {
  /** Which style to load. Default `Outlined` (the M3 baseline). */
  style?: MaterialSymbolsStyle;
  /**
   * Optional glyph subset (Material Symbols names, snake_case, e.g. `["home",
   * "search", "settings"]`). When provided, Google serves only those glyphs —
   * the dominant payload optimisation. Omit to load the full set.
   */
  iconNames?: readonly string[];
}

/**
 * Google Fonts CSS2 stylesheet URL for Material Symbols over the **full variable
 * ranges** (so the md-icon axis tokens work). Insert as `<link rel="stylesheet">`.
 */
export function materialSymbolsHref(options: MaterialSymbolsHrefOptions = {}): string {
  const style = options.style ?? "Outlined";
  const family = `Material+Symbols+${style}`;
  const a = MATERIAL_SYMBOLS_AXES;
  // Axis names must be sorted (opsz,wght,FILL,GRAD per Google API) and the value
  // tuples follow in the same order.
  const ranges =
    `opsz,wght,FILL,GRAD@` +
    `${a.opsz[0]}..${a.opsz[1]},` +
    `${a.wght[0]}..${a.wght[1]},` +
    `${a.FILL[0]}..${a.FILL[1]},` +
    `${a.GRAD[0]}..${a.GRAD[1]}`;
  let url = `https://fonts.googleapis.com/css2?family=${family}:${ranges}&display=block`;
  if (options.iconNames && options.iconNames.length > 0) {
    // Dedupe + sort for a stable, cache-friendly URL.
    const names = [...new Set(options.iconNames)].sort().join(",");
    url += `&icon_names=${names}`;
  }
  return url;
}

/**
 * Build a self-hosted `@font-face` rule for a Material Symbols variable font,
 * declaring the full weight range so `wght` is interpolable. The other axes
 * (FILL/GRAD/opsz) are exposed via `font-variation-settings` on `<md-icon>`.
 *
 * @param url URL of the variable woff2/ttf served by the app.
 * @param style Which style this file is (sets the font-family name). Default
 *   `Outlined`.
 */
export function materialSymbolsFontFaceCss(
  url: string,
  style: MaterialSymbolsStyle = "Outlined",
): string {
  const format = url.endsWith(".woff2")
    ? "woff2"
    : url.endsWith(".ttf")
      ? "truetype-variations"
      : "woff2";
  return `@font-face {
  font-family: '${materialSymbolsFamily(style)}';
  src: url('${url}') format('${format}');
  font-weight: ${MATERIAL_SYMBOLS_AXES.wght[0]} ${MATERIAL_SYMBOLS_AXES.wght[1]};
  font-display: block;
}
`;
}

/**
 * Inject the Material Symbols stylesheet `<link>` into the document once
 * (idempotent, keyed by style). No-op outside a browser. Returns `true` if it
 * injected, `false` if already present or unavailable.
 *
 * @example
 * ```ts
 * import {ensureMaterialSymbols} from '@aphrody-code/material-web/icon/material-symbols.js';
 * ensureMaterialSymbols(); // full Outlined set, variable ranges
 * ensureMaterialSymbols({iconNames: ['home', 'search', 'settings']}); // subset
 * ```
 */
export function ensureMaterialSymbols(options: MaterialSymbolsHrefOptions = {}): boolean {
  if (typeof document === "undefined") {
    return false;
  }
  const style = options.style ?? "Outlined";
  const id = `md-material-symbols-${style.toLowerCase()}`;
  if (document.getElementById(id)) {
    return false;
  }
  const link = document.createElement("link");
  link.id = id;
  link.rel = "stylesheet";
  link.href = materialSymbolsHref(options);
  document.head.appendChild(link);
  return true;
}

/**
 * Inject a self-hosted Material Symbols `@font-face` once (idempotent). No-op
 * outside a browser. Returns `true` if it injected.
 *
 * @param url URL of the variable woff2/ttf.
 * @param style Which style the file provides. Default `Outlined`.
 */
export function ensureMaterialSymbolsFontFace(
  url: string,
  style: MaterialSymbolsStyle = "Outlined",
): boolean {
  if (typeof document === "undefined") {
    return false;
  }
  const id = `md-material-symbols-face-${style.toLowerCase()}`;
  if (document.getElementById(id)) {
    return false;
  }
  const el = document.createElement("style");
  el.id = id;
  el.textContent = materialSymbolsFontFaceCss(url, style);
  document.head.appendChild(el);
  return true;
}
