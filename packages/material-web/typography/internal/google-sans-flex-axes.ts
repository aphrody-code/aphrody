/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

/**
 * @fileoverview Google Sans Flex variable-font axes, mirrored byte-for-byte
 * from the Rust source of truth (`crates/m3-tokens/src/google_sans_flex.rs`),
 * itself distilled from https://design.google/library/google-sans-flex-font.
 *
 * Google Sans Flex ships six interpolation axes in a single variable TTF:
 *
 * | Tag    | Description                                | Min  | Default | Max  |
 * |--------|--------------------------------------------|------|---------|------|
 * | `wght` | Weight (CSS `font-weight`)                 |  100 |   400   |  900 |
 * | `opsz` | Optical size (target render size, pt)      |    9 |    14   |  144 |
 * | `wdth` | Width (relative percentage)                |   75 |   100   |  125 |
 * | `GRAD` | Grade (weight nudge, line width unchanged) | -200 |     0   |  150 |
 * | `slnt` | Slant (degrees, negative = italic lean)    |  -10 |     0   |    0 |
 * | `ROND` | Roundness (terminal-shape continuum)       |    0 |     0   |  100 |
 */

/** One variable-font axis descriptor. */
export interface Axis {
  /** 4-char OpenType axis tag (case-sensitive: `wght`, `opsz`, `GRAD`, …). */
  readonly tag: string;
  /** Lower bound of the legal range. */
  readonly min: number;
  /** The font's default value for this axis. */
  readonly default: number;
  /** Upper bound of the legal range. */
  readonly max: number;
}

/** Weight axis (CSS `font-weight`). */
export const WGHT: Axis = { tag: "wght", min: 100, default: 400, max: 900 };
/** Optical-size axis (target render size, points). */
export const OPSZ: Axis = { tag: "opsz", min: 9, default: 14, max: 144 };
/** Width axis (relative percentage; 100 = neutral). */
export const WDTH: Axis = { tag: "wdth", min: 75, default: 100, max: 125 };
/** Grade axis (weight nudge that does not affect line metrics). */
export const GRAD: Axis = { tag: "GRAD", min: -200, default: 0, max: 150 };
/** Slant axis (degrees; -10 = italic lean, 0 = upright). */
export const SLNT: Axis = { tag: "slnt", min: -10, default: 0, max: 0 };
/** Roundness axis (0 = sharp terminals, 100 = fully rounded). */
export const ROND: Axis = { tag: "ROND", min: 0, default: 0, max: 100 };

/** All six axes in canonical declaration order. */
export const ALL_AXES: readonly Axis[] = [WGHT, OPSZ, WDTH, GRAD, SLNT, ROND];

/** Clamp `value` into `[axis.min, axis.max]`. */
export function clampAxis(axis: Axis, value: number): number {
  if (value < axis.min) {
    return axis.min;
  }
  if (value > axis.max) {
    return axis.max;
  }
  return value;
}

/** Current value for each variable axis. Partial — unset axes use defaults. */
export interface AxisSettings {
  wght?: number;
  opsz?: number;
  wdth?: number;
  grad?: number;
  slnt?: number;
  rond?: number;
}

/** The font's default axis values (matches the static "Regular" cut). */
export const DEFAULT_AXES: Required<AxisSettings> = {
  wght: 400,
  opsz: 14,
  wdth: 100,
  grad: 0,
  slnt: 0,
  rond: 0,
};

/**
 * Build a CSS `font-variation-settings` value from a (partial) {@link
 * AxisSettings}. Only the axes that are provided are emitted, each clamped to
 * its legal range, so callers can override one axis (e.g. just `ROND`) without
 * pinning the others.
 *
 * @example
 * fontVariationSettings({wght: 500, opsz: 24, rond: 60});
 * // => '"wght" 500, "opsz" 24, "ROND" 60'
 */
export function fontVariationSettings(settings: AxisSettings): string {
  const parts: string[] = [];
  const push = (axis: Axis, value: number | undefined) => {
    if (value !== undefined && !Number.isNaN(value)) {
      parts.push(`"${axis.tag}" ${clampAxis(axis, value)}`);
    }
  };
  push(WGHT, settings.wght);
  push(OPSZ, settings.opsz);
  push(WDTH, settings.wdth);
  push(GRAD, settings.grad);
  push(SLNT, settings.slnt);
  push(ROND, settings.rond);
  return parts.join(", ");
}
