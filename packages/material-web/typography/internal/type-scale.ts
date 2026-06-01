/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

/**
 * @fileoverview The Material 3 type scale (30 styles) expressed in Google Sans
 * Flex. Each role carries its size/line-height/tracking plus the variable-font
 * axis values that give display/headline cuts the warm, rounded Gemini-family
 * fingerprint (higher `opsz`, positive `GRAD`/`ROND`) while body/label stay
 * neutral for legibility.
 *
 * Two families are exposed: the 15 BASELINE roles (`display-large` …
 * `label-small`) and the 15 M3 Expressive EMPHASIZED roles
 * (`display-large-emphasized` … `label-small-emphasized`). Each emphasized
 * role keeps the metrics (size / line-height / tracking) of its baseline
 * namesake but carries more `wght` and `ROND` for a more emotive, expressive
 * presentation. See docs/strategy/m3-expressive-evolution.md section 6.
 *
 * Values match `DESIGN.md` and `crates/m3-tokens/src/typography.rs`.
 */

import { AxisSettings, fontVariationSettings } from "./google-sans-flex-axes.js";

/** A single M3 type-scale role. */
export interface TypeStyle {
  /** Role name, e.g. `display-large`. */
  readonly role: TypeScaleRole;
  /** Font size in px. */
  readonly sizePx: number;
  /** Line height in px. */
  readonly lineHeightPx: number;
  /** Letter spacing (tracking) in px. */
  readonly trackingPx: number;
  /** The variable-font axis values for this role. */
  readonly axes: AxisSettings;
}

/** The 15 canonical (baseline) M3 type-scale role names. */
export type BaselineTypeScaleRole =
  | "display-large"
  | "display-medium"
  | "display-small"
  | "headline-large"
  | "headline-medium"
  | "headline-small"
  | "title-large"
  | "title-medium"
  | "title-small"
  | "body-large"
  | "body-medium"
  | "body-small"
  | "label-large"
  | "label-medium"
  | "label-small";

/**
 * The 15 M3 Expressive "emphasized" type-scale role names. Each mirrors a
 * baseline role with the `-emphasized` suffix.
 */
export type EmphasizedTypeScaleRole =
  | "display-large-emphasized"
  | "display-medium-emphasized"
  | "display-small-emphasized"
  | "headline-large-emphasized"
  | "headline-medium-emphasized"
  | "headline-small-emphasized"
  | "title-large-emphasized"
  | "title-medium-emphasized"
  | "title-small-emphasized"
  | "body-large-emphasized"
  | "body-medium-emphasized"
  | "body-small-emphasized"
  | "label-large-emphasized"
  | "label-medium-emphasized"
  | "label-small-emphasized";

/**
 * All 30 M3 type-scale role names: the 15 baseline roles plus the 15 M3
 * Expressive emphasized roles.
 */
export type TypeScaleRole = BaselineTypeScaleRole | EmphasizedTypeScaleRole;

/** The 15 baseline M3 type styles, keyed by role. */
export const BASELINE_TYPE_SCALE: Readonly<Record<BaselineTypeScaleRole, TypeStyle>> = {
  "display-large": {
    role: "display-large",
    sizePx: 57,
    lineHeightPx: 64,
    trackingPx: -0.25,
    axes: { wght: 400, opsz: 96, grad: 30, rond: 60 },
  },
  "display-medium": {
    role: "display-medium",
    sizePx: 45,
    lineHeightPx: 52,
    trackingPx: 0,
    axes: { wght: 400, opsz: 72, grad: 20, rond: 50 },
  },
  "display-small": {
    role: "display-small",
    sizePx: 36,
    lineHeightPx: 44,
    trackingPx: 0,
    axes: { wght: 400, opsz: 48, rond: 40 },
  },
  "headline-large": {
    role: "headline-large",
    sizePx: 32,
    lineHeightPx: 40,
    trackingPx: 0,
    axes: { wght: 500, opsz: 36, rond: 30 },
  },
  "headline-medium": {
    role: "headline-medium",
    sizePx: 28,
    lineHeightPx: 36,
    trackingPx: 0,
    axes: { wght: 500, opsz: 28, rond: 25 },
  },
  "headline-small": {
    role: "headline-small",
    sizePx: 24,
    lineHeightPx: 32,
    trackingPx: 0,
    axes: { wght: 500, opsz: 24, rond: 20 },
  },
  "title-large": {
    role: "title-large",
    sizePx: 22,
    lineHeightPx: 28,
    trackingPx: 0,
    axes: { wght: 500, opsz: 22, rond: 15 },
  },
  "title-medium": {
    role: "title-medium",
    sizePx: 16,
    lineHeightPx: 24,
    trackingPx: 0.15,
    axes: { wght: 500, opsz: 16 },
  },
  "title-small": {
    role: "title-small",
    sizePx: 14,
    lineHeightPx: 20,
    trackingPx: 0.1,
    axes: { wght: 500, opsz: 14 },
  },
  "body-large": {
    role: "body-large",
    sizePx: 16,
    lineHeightPx: 24,
    trackingPx: 0.5,
    axes: { wght: 400, opsz: 16, rond: 20 },
  },
  "body-medium": {
    role: "body-medium",
    sizePx: 14,
    lineHeightPx: 20,
    trackingPx: 0.25,
    axes: { wght: 400, opsz: 14 },
  },
  "body-small": {
    role: "body-small",
    sizePx: 12,
    lineHeightPx: 16,
    trackingPx: 0.4,
    axes: { wght: 400, opsz: 12 },
  },
  "label-large": {
    role: "label-large",
    sizePx: 14,
    lineHeightPx: 20,
    trackingPx: 0.1,
    axes: { wght: 500, opsz: 14 },
  },
  "label-medium": {
    role: "label-medium",
    sizePx: 12,
    lineHeightPx: 16,
    trackingPx: 0.5,
    axes: { wght: 500, opsz: 12 },
  },
  "label-small": {
    role: "label-small",
    sizePx: 11,
    lineHeightPx: 16,
    trackingPx: 0.5,
    axes: { wght: 500, opsz: 11 },
  },
};

/**
 * The 15 M3 Expressive "emphasized" type styles, keyed by role. Each shares
 * its metrics (size / line-height / tracking) with the baseline role of the
 * same name; the `wght` and `ROND` axes are raised for emphasis. The exact
 * `display-large-emphasized` and `headline-large-emphasized` values come from
 * docs/strategy/m3-expressive-evolution.md section 6; the rest derive
 * coherently (baseline `wght` + ~200, capped at the 900 `wght` ceiling, with a
 * growing `ROND` for display/headline/title and a moderate one for
 * body/label). All axis values sit within the Google Sans Flex ranges
 * declared in google-sans-flex-axes.ts (`wght` 100..900, `ROND` 0..100), so
 * `fontVariationSettings` clamps none of them.
 */
export const EMPHASIZED_TYPE_SCALE: Readonly<Record<EmphasizedTypeScaleRole, TypeStyle>> = {
  "display-large-emphasized": {
    role: "display-large-emphasized",
    sizePx: 57,
    lineHeightPx: 64,
    trackingPx: -0.25,
    axes: { wght: 700, opsz: 96, grad: 50, rond: 80 },
  },
  "display-medium-emphasized": {
    role: "display-medium-emphasized",
    sizePx: 45,
    lineHeightPx: 52,
    trackingPx: 0,
    axes: { wght: 700, opsz: 72, grad: 40, rond: 75 },
  },
  "display-small-emphasized": {
    role: "display-small-emphasized",
    sizePx: 36,
    lineHeightPx: 44,
    trackingPx: 0,
    axes: { wght: 700, opsz: 48, grad: 20, rond: 70 },
  },
  "headline-large-emphasized": {
    role: "headline-large-emphasized",
    sizePx: 32,
    lineHeightPx: 40,
    trackingPx: 0,
    axes: { wght: 800, opsz: 36, rond: 60 },
  },
  "headline-medium-emphasized": {
    role: "headline-medium-emphasized",
    sizePx: 28,
    lineHeightPx: 36,
    trackingPx: 0,
    axes: { wght: 700, opsz: 28, rond: 55 },
  },
  "headline-small-emphasized": {
    role: "headline-small-emphasized",
    sizePx: 24,
    lineHeightPx: 32,
    trackingPx: 0,
    axes: { wght: 700, opsz: 24, rond: 50 },
  },
  "title-large-emphasized": {
    role: "title-large-emphasized",
    sizePx: 22,
    lineHeightPx: 28,
    trackingPx: 0,
    axes: { wght: 700, opsz: 22, rond: 45 },
  },
  "title-medium-emphasized": {
    role: "title-medium-emphasized",
    sizePx: 16,
    lineHeightPx: 24,
    trackingPx: 0.15,
    axes: { wght: 700, opsz: 16, rond: 40 },
  },
  "title-small-emphasized": {
    role: "title-small-emphasized",
    sizePx: 14,
    lineHeightPx: 20,
    trackingPx: 0.1,
    axes: { wght: 700, opsz: 14, rond: 40 },
  },
  "body-large-emphasized": {
    role: "body-large-emphasized",
    sizePx: 16,
    lineHeightPx: 24,
    trackingPx: 0.5,
    axes: { wght: 600, opsz: 16, rond: 30 },
  },
  "body-medium-emphasized": {
    role: "body-medium-emphasized",
    sizePx: 14,
    lineHeightPx: 20,
    trackingPx: 0.25,
    axes: { wght: 600, opsz: 14, rond: 30 },
  },
  "body-small-emphasized": {
    role: "body-small-emphasized",
    sizePx: 12,
    lineHeightPx: 16,
    trackingPx: 0.4,
    axes: { wght: 600, opsz: 12, rond: 30 },
  },
  "label-large-emphasized": {
    role: "label-large-emphasized",
    sizePx: 14,
    lineHeightPx: 20,
    trackingPx: 0.1,
    axes: { wght: 700, opsz: 14, rond: 35 },
  },
  "label-medium-emphasized": {
    role: "label-medium-emphasized",
    sizePx: 12,
    lineHeightPx: 16,
    trackingPx: 0.5,
    axes: { wght: 700, opsz: 12, rond: 35 },
  },
  "label-small-emphasized": {
    role: "label-small-emphasized",
    sizePx: 11,
    lineHeightPx: 16,
    trackingPx: 0.5,
    axes: { wght: 700, opsz: 11, rond: 35 },
  },
};

/**
 * The full M3 type scale (30 roles): the 15 baseline roles followed by the 15
 * M3 Expressive emphasized roles. This is the runtime source of truth consumed
 * by `<md-type>`, `typeStyleCss`, and `typeScaleVars`.
 */
export const TYPE_SCALE: Readonly<Record<TypeScaleRole, TypeStyle>> = {
  ...BASELINE_TYPE_SCALE,
  ...EMPHASIZED_TYPE_SCALE,
};

/**
 * All 30 type styles in canonical order (15 baseline, then 15 emphasized).
 */
export const ALL_TYPE_STYLES: readonly TypeStyle[] = Object.values(TYPE_SCALE);

/**
 * Resolve a role to the inline CSS declarations that render it: family, size,
 * line-height, tracking, weight, and `font-variation-settings`. Returns a map
 * suitable for `styleMap()` or direct assignment.
 */
export function typeStyleCss(role: TypeScaleRole): Record<string, string> {
  const s = TYPE_SCALE[role];
  return {
    "font-family":
      "var(--md-sys-typescale-font, 'Google Sans Flex', Roboto, system-ui, sans-serif)",
    "font-size": `${s.sizePx}px`,
    "line-height": `${s.lineHeightPx}px`,
    "letter-spacing": `${s.trackingPx}px`,
    "font-weight": `${s.axes.wght ?? 400}`,
    "font-variation-settings": fontVariationSettings(s.axes),
  };
}

/**
 * Emit the M3 typescale CSS custom properties (`--md-sys-typescale-<role>-*`)
 * for the whole scale, as a `name: value` record. Lets a page register the
 * scale once and have components read it through `var()`.
 */
export function typeScaleVars(): Record<string, string> {
  const vars: Record<string, string> = {};
  for (const s of ALL_TYPE_STYLES) {
    const p = `--md-sys-typescale-${s.role}`;
    vars[`${p}-size`] = `${s.sizePx}px`;
    vars[`${p}-line-height`] = `${s.lineHeightPx}px`;
    vars[`${p}-tracking`] = `${s.trackingPx}px`;
    vars[`${p}-weight`] = `${s.axes.wght ?? 400}`;
    vars[`${p}-variation`] = fontVariationSettings(s.axes);
  }
  return vars;
}
