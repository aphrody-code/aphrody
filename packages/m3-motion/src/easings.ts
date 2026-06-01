/**
 * Material Design 3 Easing Presets.
 * Represented as cubic-bezier array coordinates [x1, y1, x2, y2].
 *
 * Source of truth: packages/material-web/tokens/versions/v0_192/_md-sys-motion.scss
 * and docs/01-md3-spec-foundations.md §7.1
 *
 * Note on `emphasized`: the full M3 emphasized path is a 2-segment spline
 * (not expressible as a single cubic-bezier). The token file maps
 * easing-emphasized to cubic-bezier(0.2, 0, 0, 1) — identical to standard —
 * because that is the closest single-bezier approximation used in web
 * implementations. Use emphasizedDecelerate / emphasizedAccelerate for
 * enter/exit transitions respectively (recommended practice per spec §7.1).
 */
export const m3Easings = {
  /** Closest single-bezier approximation. Use emphasizedDecelerate/Accelerate for transitions. */
  emphasized: [0.2, 0.0, 0.0, 1.0],
  emphasizedDecelerate: [0.05, 0.7, 0.1, 1.0],
  emphasizedAccelerate: [0.3, 0.0, 0.8, 0.15],
  standard: [0.2, 0.0, 0.0, 1.0],
  standardDecelerate: [0.0, 0.0, 0.0, 1.0],
  standardAccelerate: [0.3, 0.0, 1.0, 1.0],
  linear: [0.0, 0.0, 1.0, 1.0],
  /** Legacy (M2 default) — kept for migration compatibility. */
  legacy: [0.4, 0.0, 0.2, 1.0],
  legacyDecelerate: [0.0, 0.0, 0.2, 1.0],
  legacyAccelerate: [0.4, 0.0, 1.0, 1.0],
} as const;

export type M3EasingKey = keyof typeof m3Easings;
export type M3EasingValue = (typeof m3Easings)[M3EasingKey];

/**
 * Material Design 3 Duration Presets (in seconds, compatible with Motion duration option).
 *
 * Canonical values from _md-sys-motion.scss:
 * short1=50ms, short2=100ms, short3=150ms, short4=200ms
 * medium1=250ms, medium2=300ms, medium3=350ms, medium4=400ms
 * long1=450ms, long2=500ms, long3=550ms, long4=600ms
 * extraLong1=700ms, extraLong2=800ms, extraLong3=900ms, extraLong4=1000ms
 */
export const m3Durations = {
  short1: 0.05,
  short2: 0.1,
  short3: 0.15,
  short4: 0.2,
  medium1: 0.25,
  medium2: 0.3,
  medium3: 0.35,
  medium4: 0.4,
  long1: 0.45,
  long2: 0.5,
  long3: 0.55,
  long4: 0.6,
  extraLong1: 0.7,
  extraLong2: 0.8,
  extraLong3: 0.9,
  extraLong4: 1.0,
} as const;

export type M3DurationKey = keyof typeof m3Durations;
export type M3DurationValue = (typeof m3Durations)[M3DurationKey];
