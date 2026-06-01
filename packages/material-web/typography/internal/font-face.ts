/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

/**
 * @fileoverview Helpers to register the Google Sans Flex variable font. Two
 * sources are supported:
 *
 * 1. A self-hosted variable TTF (all six axes, fully offline) — preferred for
 *    aphrody, which ships the font under the SIL OFL at
 *    `assets/fonts/google-sans-flex/`.
 * 2. The Google Fonts CDN (`fonts.google.com/specimen/Google+Sans+Flex`) for
 *    quick prototypes; note the CSS2 API only exposes the registered `opsz`
 *    and `wght` axes in its URL, so prefer the self-hosted font when `GRAD`,
 *    `ROND`, `slnt`, or `wdth` control is required.
 */

/** The canonical UI/display font-family name. */
export const FONT_FAMILY = "Google Sans Flex";

/**
 * The canonical code/monospace font-family name. Google Sans Code is the face
 * Google ships for code surfaces (Gemini, AI Studio); it exposes a `MONO` axis
 * (0 = proportional, 1 = monospace) and `wght` (300–800). Confirmed via the bxc
 * Google-design recon (`packages/bxc/scripts/google-design-recon.ts`).
 */
export const CODE_FONT_FAMILY = "Google Sans Code";

/**
 * Build a self-hosted `@font-face` rule for the full variable font, declaring
 * the complete weight/stretch/oblique ranges so every axis is interpolable.
 * Mirrors `m3_tokens::google_sans_flex::export_font_face`.
 *
 * @param url URL of the variable TTF (e.g. an absolute path served by the app).
 */
export function fontFaceCss(url: string): string {
  return `@font-face {
  font-family: '${FONT_FAMILY}';
  src: url('${url}') format('truetype-variations');
  font-weight: 100 900;
  font-stretch: 75% 125%;
  font-style: oblique -10deg 0deg;
  font-display: swap;
}
`;
}

/**
 * The Google Fonts CSS2 stylesheet URL for Google Sans Flex over the full
 * optical-size and weight ranges. Insert as `<link rel="stylesheet">`.
 */
export function googleFontsHref(): string {
  return "https://fonts.googleapis.com/css2?family=Google+Sans+Flex:opsz,wght@9..144,100..900&display=swap";
}

/**
 * Build a self-hosted `@font-face` rule for the Google Sans Code variable font
 * (`MONO` + `wght` axes). Mirrors `m3_tokens::google_sans_code`.
 *
 * @param url URL of the variable TTF.
 */
export function codeFontFaceCss(url: string): string {
  return `@font-face {
  font-family: '${CODE_FONT_FAMILY}';
  src: url('${url}') format('truetype-variations');
  font-weight: 300 800;
  font-display: swap;
}
`;
}

/** The Google Fonts CSS2 stylesheet URL for Google Sans Code (weight range). */
export function codeGoogleFontsHref(): string {
  return "https://fonts.googleapis.com/css2?family=Google+Sans+Code:wght@300..800&display=swap";
}

/**
 * Inject the self-hosted `@font-face` into the document once (idempotent). No-op
 * outside a browser. Returns `true` if it injected, `false` if already present
 * or unavailable.
 */
export function ensureFontFace(url: string): boolean {
  if (typeof document === "undefined") {
    return false;
  }
  const id = "md-google-sans-flex-face";
  if (document.getElementById(id)) {
    return false;
  }
  const style = document.createElement("style");
  style.id = id;
  style.textContent = fontFaceCss(url);
  document.head.appendChild(style);
  return true;
}
