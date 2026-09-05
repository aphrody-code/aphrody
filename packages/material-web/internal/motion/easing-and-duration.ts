/**
 * @license
 * Copyright 2026 Google LLC
 * SPDX-License-Identifier: Apache-2.0
 */

/**
 * @fileoverview The full Material Design 3 motion system — the seven canonical
 * easing curves and the sixteen duration tokens — exposed as typed constants,
 * CSS custom-property maps, and small composable helpers.
 *
 * Values are aligned byte-for-byte with the Rust source of truth
 * (`crates/m3-tokens/src/motion.rs`) so that a component animated in TypeScript
 * matches one painted by the native (mui-rs) renderer.
 *
 * Reference:
 * https://m3.material.io/styles/motion/easing-and-duration/tokens-specs
 */

/**
 * The seven M3 easing curves as CSS `cubic-bezier(...)` strings.
 *
 * Unlike the legacy {@link ./animation.js#EASING} table (kept for backwards
 * compatibility with the upstream components), `EMPHASIZED` here uses the exact
 * spec curve rather than an approximation.
 */
export const EASING = {
  /** Default easing for most transitions (decelerate-in). */
  EMPHASIZED: "cubic-bezier(0.2, 0, 0, 1)",
  /** Elements entering the screen. */
  EMPHASIZED_DECELERATE: "cubic-bezier(0.05, 0.7, 0.1, 1)",
  /** Elements leaving the screen. */
  EMPHASIZED_ACCELERATE: "cubic-bezier(0.3, 0, 0.8, 0.15)",
  /** General-purpose easing for elements that stay on screen. */
  STANDARD: "cubic-bezier(0.2, 0, 0, 1)",
  /** Entering/expanding element with the standard curve. */
  STANDARD_DECELERATE: "cubic-bezier(0, 0, 0, 1)",
  /** Exiting/collapsing element with the standard curve. */
  STANDARD_ACCELERATE: "cubic-bezier(0.3, 0, 1, 1)",
  /** Constant rate, for continuous indicators (progress, scrubbing). */
  LINEAR: "cubic-bezier(0, 0, 1, 1)",
} as const;

/** Name of an easing curve in {@link EASING}. */
export type EasingName = keyof typeof EASING;

/**
 * The sixteen M3 duration tokens, in milliseconds, ascending from `short1`
 * (50 ms) to `extraLong4` (1000 ms).
 */
export const DURATION = {
  SHORT1: 50,
  SHORT2: 100,
  SHORT3: 150,
  SHORT4: 200,
  MEDIUM1: 250,
  MEDIUM2: 300,
  MEDIUM3: 350,
  MEDIUM4: 400,
  LONG1: 450,
  LONG2: 500,
  LONG3: 550,
  LONG4: 600,
  EXTRA_LONG1: 700,
  EXTRA_LONG2: 800,
  EXTRA_LONG3: 900,
  EXTRA_LONG4: 1000,
} as const;

/** Name of a duration token in {@link DURATION}. */
export type DurationName = keyof typeof DURATION;

/**
 * The `--md-sys-motion-*` custom properties as a `name: value` record, ready to
 * be serialized into a `:root` block or applied to an element's style.
 *
 * Token names follow the M3 spec (`emphasized`, `short-1`, `extra-long-4`, …)
 * so authored CSS can read `var(--md-sys-motion-easing-emphasized)` and
 * `var(--md-sys-motion-duration-medium-2)`.
 */
export function motionTokenVars(): Record<string, string> {
  const vars: Record<string, string> = {};
  const kebab = (name: string) =>
    name
      .toLowerCase()
      .replace(/_/g, "-")
      .replace(/([a-z])(\d)/g, "$1-$2");
  for (const [name, curve] of Object.entries(EASING)) {
    vars[`--md-sys-motion-easing-${kebab(name)}`] = curve;
  }
  for (const [name, ms] of Object.entries(DURATION)) {
    vars[`--md-sys-motion-duration-${kebab(name)}`] = `${ms}ms`;
  }
  return vars;
}

/**
 * Builds a CSS `transition` shorthand from one or more properties using an M3
 * duration and easing. Defaults to `medium2` / `standard`, the canonical
 * navigation-transition pairing.
 *
 * @example
 * // "opacity 300ms cubic-bezier(0.2, 0, 0, 1), transform 300ms cubic-bezier(0.2, 0, 0, 1)"
 * transition(['opacity', 'transform']);
 */
export function transition(
  properties: string | readonly string[],
  duration: DurationName = "MEDIUM2",
  easing: EasingName = "STANDARD",
  delayMs = 0,
): string {
  const props = typeof properties === "string" ? [properties] : properties;
  const ms = DURATION[duration];
  const curve = EASING[easing];
  const delay = delayMs ? ` ${delayMs}ms` : "";
  return props.map((p) => `${p} ${ms}ms ${curve}${delay}`).join(", ");
}

/**
 * Honors the user's reduced-motion preference. Returns `true` when the user has
 * requested reduced motion, in which case callers should skip or shorten
 * non-essential animation. Safe to call in non-browser (SSR) contexts.
 */
export function prefersReducedMotion(): boolean {
  return typeof matchMedia === "function" && matchMedia("(prefers-reduced-motion: reduce)").matches;
}

/**
 * Resolves an M3 `KeyframeAnimationOptions` object for `Element.animate()`,
 * collapsing to a near-instant duration when reduced motion is requested.
 */
export function animationOptions(
  duration: DurationName = "MEDIUM2",
  easing: EasingName = "STANDARD",
  extra: KeyframeAnimationOptions = {},
): KeyframeAnimationOptions {
  return {
    duration: prefersReducedMotion() ? 1 : DURATION[duration],
    easing: EASING[easing],
    fill: "both",
    ...extra,
  };
}
