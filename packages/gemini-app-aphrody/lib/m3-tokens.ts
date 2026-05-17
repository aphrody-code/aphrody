// SPDX-License-Identifier: Apache-2.0
//
// CSS / TS mirror of:
//
//   - crates/m3-tokens/src/gemini_brand.rs (Gemini brand colors + gradients)
//   - crates/m3-tokens/src/google_sans_flex.rs (variable-font axes)
//
// Source of truth lives in Rust. This module re-encodes the constants in
// TypeScript so the Next.js layer can emit them into a <style> block at
// SSR time without spawning a Cargo build. Tests in tests/components.test.tsx
// pin the hex/string outputs against the Rust constants to keep them in sync.

// ---------------------------------------------------------------------------
// Gemini brand color constants — mirror gemini_brand.rs lines 50-66.
// ---------------------------------------------------------------------------

export const GEMINI_BRAND_BLUE = "#4285F4";
export const GEMINI_BRAND_PURPLE = "#9168C0";
export const GEMINI_BRAND_PINK = "#EC4899";
export const GEMINI_BRAND_YELLOW = "#FAE366";
export const GEMINI_BRAND_GREEN = "#BFF28D";

/** Google four-color dot lineage — gemini_brand.rs FOUR_COLOR_DOTS. */
export const FOUR_COLOR_DOTS = [
  "#4285F4",
  "#EA4335",
  "#FBBC04",
  "#34A853",
] as const;

// ---------------------------------------------------------------------------
// Canonical gradients — mirror SPECTRUM_SHIFT_GRADIENT / SPARKLE_GRADIENT /
// WARM_TONE_GRADIENT from gemini_brand.rs.
// ---------------------------------------------------------------------------

export const SPECTRUM_SHIFT_CSS = `linear-gradient(90deg, ${GEMINI_BRAND_BLUE} 0%, ${GEMINI_BRAND_PURPLE} 50%, ${GEMINI_BRAND_PINK} 100%)`;

export const SPARKLE_CSS = `linear-gradient(135deg, ${FOUR_COLOR_DOTS[0]} 0%, ${FOUR_COLOR_DOTS[1]} 33%, ${FOUR_COLOR_DOTS[2]} 66%, ${FOUR_COLOR_DOTS[3]} 100%)`;

export const WARM_TONE_CSS = `linear-gradient(45deg, ${GEMINI_BRAND_YELLOW} 0%, ${GEMINI_BRAND_PINK} 100%)`;

// ---------------------------------------------------------------------------
// Brand corner radii — gemini_brand.rs lines 133-139.
// ---------------------------------------------------------------------------

export const BRAND_CORNER_PROMPT_BAR_PX = 28;
export const BRAND_CORNER_MESSAGE_PX = 24;
export const BRAND_CORNER_CHIP_PX = 18;

// ---------------------------------------------------------------------------
// M3 baseline color tokens — subset used by the Gemini surface.
// Mirrors crates/m3-tokens/src/color.rs constants (baseline light theme).
// ---------------------------------------------------------------------------

export const M3_BASELINE_LIGHT = {
  primary: "#6750A4",
  onPrimary: "#FFFFFF",
  surface: "#FEF7FF",
  surfaceVariant: "#E7E0EC",
  surfaceContainerLow: "#F7F2FA",
  surfaceContainer: "#F3EDF7",
  surfaceContainerHigh: "#ECE6F0",
  onSurface: "#1D1B20",
  onSurfaceVariant: "#49454F",
  outline: "#79747E",
  outlineVariant: "#CAC4D0",
} as const;

// ---------------------------------------------------------------------------
// Google Sans Flex axis settings — mirror google_sans_flex.rs lines 55-95.
// ---------------------------------------------------------------------------

export interface AxisSettings {
  wght: number;
  opsz: number;
  wdth: number;
  grad: number;
  slnt: number;
  rond: number;
}

const AXIS_BOUNDS = {
  wght: { min: 100, max: 900, default: 400 },
  opsz: { min: 9, max: 144, default: 14 },
  wdth: { min: 75, max: 125, default: 100 },
  grad: { min: -200, max: 150, default: 0 },
  slnt: { min: -10, max: 0, default: 0 },
  rond: { min: 0, max: 100, default: 0 },
} as const;

function clampAxis(name: keyof typeof AXIS_BOUNDS, value: number): number {
  const b = AXIS_BOUNDS[name];
  if (value < b.min) return b.min;
  if (value > b.max) return b.max;
  return value;
}

/** Mirror of google_sans_flex.rs::font_variation_settings. */
export function fontVariationSettings(s: Partial<AxisSettings>): string {
  const wght = clampAxis("wght", s.wght ?? AXIS_BOUNDS.wght.default);
  const opsz = clampAxis("opsz", s.opsz ?? AXIS_BOUNDS.opsz.default);
  const wdth = clampAxis("wdth", s.wdth ?? AXIS_BOUNDS.wdth.default);
  const grad = clampAxis("grad", s.grad ?? AXIS_BOUNDS.grad.default);
  const slnt = clampAxis("slnt", s.slnt ?? AXIS_BOUNDS.slnt.default);
  const rond = clampAxis("rond", s.rond ?? AXIS_BOUNDS.rond.default);
  return `"wght" ${wght.toFixed(0)}, "opsz" ${opsz.toFixed(0)}, "wdth" ${wdth.toFixed(0)}, "GRAD" ${grad.toFixed(0)}, "slnt" ${slnt.toFixed(0)}, "ROND" ${rond.toFixed(0)}`;
}

/** Emit the @font-face block pointing at the public/fonts asset. */
export function fontFaceCss(): string {
  return [
    "@font-face {",
    "  font-family: 'Google Sans Flex';",
    "  src: url('/fonts/google-sans-flex/GoogleSansFlex-VariableFont_GRAD,ROND,opsz,slnt,wdth,wght.ttf') format('truetype-variations');",
    "  font-weight: 100 900;",
    "  font-stretch: 75% 125%;",
    "  font-style: oblique -10deg 0deg;",
    "  font-display: swap;",
    "}",
  ].join("\n");
}

/**
 * Emit the full :root token block — combines M3 baseline + Gemini brand.
 * Importable from globals.css build pipeline or tests.
 */
export function rootTokensCss(): string {
  return [
    ":root {",
    `  --md-sys-color-primary: ${M3_BASELINE_LIGHT.primary};`,
    `  --md-sys-color-on-primary: ${M3_BASELINE_LIGHT.onPrimary};`,
    `  --md-sys-color-surface: ${M3_BASELINE_LIGHT.surface};`,
    `  --md-sys-color-surface-variant: ${M3_BASELINE_LIGHT.surfaceVariant};`,
    `  --md-sys-color-surface-container-low: ${M3_BASELINE_LIGHT.surfaceContainerLow};`,
    `  --md-sys-color-surface-container: ${M3_BASELINE_LIGHT.surfaceContainer};`,
    `  --md-sys-color-surface-container-high: ${M3_BASELINE_LIGHT.surfaceContainerHigh};`,
    `  --md-sys-color-on-surface: ${M3_BASELINE_LIGHT.onSurface};`,
    `  --md-sys-color-on-surface-variant: ${M3_BASELINE_LIGHT.onSurfaceVariant};`,
    `  --md-sys-color-outline: ${M3_BASELINE_LIGHT.outline};`,
    `  --md-sys-color-outline-variant: ${M3_BASELINE_LIGHT.outlineVariant};`,
    `  --gemini-brand-blue: ${GEMINI_BRAND_BLUE};`,
    `  --gemini-brand-purple: ${GEMINI_BRAND_PURPLE};`,
    `  --gemini-brand-pink: ${GEMINI_BRAND_PINK};`,
    `  --gemini-brand-yellow: ${GEMINI_BRAND_YELLOW};`,
    `  --gemini-brand-green: ${GEMINI_BRAND_GREEN};`,
    `  --gemini-spectrum-shift: ${SPECTRUM_SHIFT_CSS};`,
    `  --gemini-sparkle: ${SPARKLE_CSS};`,
    `  --gemini-warm-tone: ${WARM_TONE_CSS};`,
    `  --gemini-corner-prompt-bar: ${BRAND_CORNER_PROMPT_BAR_PX}px;`,
    `  --gemini-corner-message: ${BRAND_CORNER_MESSAGE_PX}px;`,
    `  --gemini-corner-chip: ${BRAND_CORNER_CHIP_PX}px;`,
    "}",
  ].join("\n");
}
