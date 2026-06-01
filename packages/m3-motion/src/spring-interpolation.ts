// SPDX-License-Identifier: Apache-2.0

/**
 * Physical spring interpolation for the Web platform (M3 Expressive, Phase 3.1).
 *
 * The React components in this package drive springs through `motion/react`,
 * which runs a per-frame numerical integrator. Lit components from
 * `@material/web` live inside a Shadow DOM and have no access to Motion, so
 * they can only animate through CSS transitions (a single `cubic-bezier`) or
 * the Web Animations API (`element.animate(keyframes, options)`).
 *
 * This module is the bridge: it turns the same physical spring parameters
 * (`stiffness` / `damping` / `mass`) used by {@link m3Springs} into:
 *
 *   1. a settle-time estimate in milliseconds ({@link springDurationMs}),
 *   2. a single `cubic-bezier(...)` CSS easing approximation
 *      ({@link springToCssEasing}), and
 *   3. an exact sampling of the analytical damped-spring solution as WAAPI
 *      keyframes ({@link springToWaapiKeyframes}), which — unlike a
 *      cubic-bezier — preserves overshoot/bounce.
 *
 * All functions are pure, deterministic and SSR-safe: there is no DOM access
 * at module load or call time. `Keyframe` is referenced only as a DOM lib
 * type. The maths handle the three damping regimes (under-, critically- and
 * over-damped) without dividing by zero.
 *
 * Physical model (mass-spring-damper, m x'' + c x' + k x = 0):
 *   - natural angular frequency   w0   = sqrt(k / m)
 *   - damping ratio               zeta = c / (2 * sqrt(k * m))
 *   - damped angular frequency    wd   = w0 * sqrt(1 - zeta^2)   (under-damped)
 *
 * Here `k` = stiffness, `c` = damping, `m` = mass.
 */

/** Physical parameters of a mass-spring-damper system. */
export interface SpringParams {
  /** Spring stiffness `k` (> 0). Higher = snappier. */
  stiffness: number;
  /** Damping coefficient `c` (>= 0). Higher = less / no bounce. */
  damping: number;
  /** Mass `m` (> 0). Defaults to 1. Higher = slower, heavier feel. */
  mass?: number;
}

/** One sample of the analytical spring solution, progress-normalised. */
export interface SpringSample {
  /** Timeline position in [0, 1]. */
  offset: number;
  /**
   * Animated value at this offset. With `from`/`to` defaults (0 -> 1) this
   * is the normalised progress and may exceed 1 (overshoot) for
   * under-damped springs — that is the whole point of WAAPI sampling.
   */
  value: number;
}

/** Result bundle for WAAPI consumption. */
export interface SpringWaapi {
  /** Progress-normalised samples (always present, easy to remap). */
  samples: SpringSample[];
  /**
   * Ready-to-use `Keyframe[]`. Empty `{ offset }`-only objects unless a
   * property mapper is supplied via {@link springToWaapiKeyframes} options;
   * use {@link springKeyframesForProperty} to project `samples` onto a CSS
   * property such as `transform` or `opacity`.
   */
  keyframes: Keyframe[];
  /** WAAPI `KeyframeEffectOptions`-compatible timing. */
  options: { duration: number; easing: string };
}

const MIN_DURATION_MS = 50;
const MAX_DURATION_MS = 2000;
/** Envelope threshold for "settled": within 0.5% of the rest position. */
const SETTLE_THRESHOLD = 0.005;
const DEFAULT_MASS = 1;
const DEFAULT_STEPS = 60;

/** Clamp helper (deterministic, NaN-safe via the final fallback). */
function clamp(value: number, min: number, max: number): number {
  if (value < min) return min;
  if (value > max) return max;
  return value;
}

/**
 * Normalise raw params: enforce a positive mass/stiffness and a non-negative
 * damping so the downstream maths never produce NaN/Infinity.
 */
function normalize(params: SpringParams): { k: number; c: number; m: number } {
  const m = params.mass != null && params.mass > 0 ? params.mass : DEFAULT_MASS;
  const k = params.stiffness > 0 ? params.stiffness : 1;
  const c = params.damping >= 0 ? params.damping : 0;
  return { k, c, m };
}

/** Natural angular frequency `w0 = sqrt(k / m)`. */
function naturalFrequency(k: number, m: number): number {
  return Math.sqrt(k / m);
}

/** Damping ratio `zeta = c / (2 * sqrt(k * m))`. */
function dampingRatio(k: number, c: number, m: number): number {
  const denom = 2 * Math.sqrt(k * m);
  return denom > 0 ? c / denom : 0;
}

/**
 * Estimate the spring settle time in milliseconds.
 *
 * The motion decays under the exponential envelope `exp(-zeta * w0 * t)`. The
 * system is considered settled once that envelope drops below
 * {@link SETTLE_THRESHOLD} (0.5%):
 *
 *     exp(-zeta * w0 * t) = threshold
 *  => t = -ln(threshold) / (zeta * w0)
 *
 * For over-damped systems (`zeta > 1`) the slow real root dominates the decay,
 * so we use the smaller-magnitude pole `s = w0 * (zeta - sqrt(zeta^2 - 1))`
 * instead of `zeta * w0`, which would otherwise badly under-estimate the tail.
 *
 * The result is clamped to a sensible UI range
 * [{@link MIN_DURATION_MS}, {@link MAX_DURATION_MS}].
 */
export function springDurationMs(params: SpringParams): number {
  const { k, c, m } = normalize(params);
  const w0 = naturalFrequency(k, m);
  const zeta = dampingRatio(k, c, m);

  // Effective decay rate (1/s). For under/critically-damped this is zeta*w0;
  // for over-damped the dominant (slowest) pole governs the tail.
  let decayRate: number;
  if (zeta > 1) {
    const root = Math.sqrt(zeta * zeta - 1);
    decayRate = w0 * (zeta - root);
  } else {
    decayRate = zeta * w0;
  }

  if (!(decayRate > 0) || !Number.isFinite(decayRate)) {
    // Undamped or degenerate: cap at the maximum.
    return MAX_DURATION_MS;
  }

  const settleSeconds = -Math.log(SETTLE_THRESHOLD) / decayRate;
  const settleMs = settleSeconds * 1000;
  return clamp(settleMs, MIN_DURATION_MS, MAX_DURATION_MS);
}

/**
 * Approximate a spring with a single CSS `cubic-bezier(x1,y1,x2,y2)` easing.
 *
 * LIMITATION: a cubic-bezier easing is monotonic in time and its output is
 * conceptually clamped to [0, 1] by the CSS engine for most properties, so it
 * **cannot represent overshoot/bounce**. For bouncy (under-damped) spatial
 * springs we therefore fall back to an expressive M3 decelerate curve
 * (`cubic-bezier(0.05, 0.7, 0.1, 1)` = `emphasizedDecelerate`) which reads as
 * a lively settle without the impossible overshoot. Use
 * {@link springToWaapiKeyframes} when real bounce is required.
 *
 * Control points are derived from the damping ratio `zeta`:
 *   - low zeta  (bouncy)  -> sharper attack, snappier feel
 *   - high zeta (smooth)  -> gentle ease-out, closer to standard decelerate
 *
 * The mapping is deterministic and the control points are bounded to valid
 * cubic-bezier ranges (x in [0, 1]; y unbounded by spec but kept in [0, 1]).
 */
export function springToCssEasing(params: SpringParams): string {
  const { k, c, m } = normalize(params);
  const zeta = dampingRatio(k, c, m);

  // Bouncy springs (visibly under-damped) overshoot, which a cubic-bezier
  // cannot express -> fall back to an expressive M3 decelerate.
  if (zeta < 0.85) {
    const [x1, y1, x2, y2] = [0.05, 0.7, 0.1, 1.0];
    return `cubic-bezier(${x1}, ${y1}, ${x2}, ${y2})`;
  }

  // For near-critical / over-damped springs we synthesise a decelerate
  // curve whose attack sharpens as zeta drops toward 1. Map zeta in
  // [0.85, ~2+] onto an interpolation factor t in [1, 0] (clamped), where
  // t=1 is the sharpest valid attack and t=0 a soft standard ease-out.
  const t = clamp((zeta - 0.85) / (1.6 - 0.85), 0, 1);

  // Endpoints of the interpolation:
  //   sharp  (t = 0): emphasized-ish decelerate, vivid attack
  //   smooth (t = 1): standard decelerate, gentle
  const x1 = lerp(0.1, 0.0, t);
  const y1 = lerp(0.65, 0.0, t);
  const x2 = lerp(0.2, 0.0, t);
  const y2 = 1.0;

  return `cubic-bezier(${round(x1)}, ${round(y1)}, ${round(x2)}, ${round(y2)})`;
}

function lerp(a: number, b: number, t: number): number {
  return a + (b - a) * t;
}

function round(n: number): number {
  return Math.round(n * 1000) / 1000;
}

/**
 * Evaluate the analytical position of a unit-step damped spring at time `t`
 * (seconds), starting at rest at 0 and settling to 1.
 *
 * Closed-form solutions of `m x'' + c x' + k x = k` with `x(0)=0`, `x'(0)=0`:
 *
 *  - Under-damped  (zeta < 1):
 *      x(t) = 1 - e^{-zeta w0 t} ( cos(wd t) + (zeta w0 / wd) sin(wd t) )
 *      with wd = w0 sqrt(1 - zeta^2)
 *  - Critically damped (zeta == 1):
 *      x(t) = 1 - e^{-w0 t} (1 + w0 t)
 *  - Over-damped  (zeta > 1):
 *      with poles s1,2 = -w0 (zeta -/+ sqrt(zeta^2 - 1)),
 *      x(t) = 1 - ( s2 e^{s1 t} - s1 e^{s2 t} ) / (s2 - s1)
 *
 * The under-damped branch is the only one that can return values > 1
 * (overshoot). The `wd` divide is guarded: it is only used when
 * `1 - zeta^2 > 0`, so there is no division by zero at the regime boundaries.
 */
function springPosition(t: number, w0: number, zeta: number): number {
  if (t <= 0) return 0;

  if (zeta < 1 - 1e-6) {
    // Under-damped.
    const wd = w0 * Math.sqrt(1 - zeta * zeta);
    const envelope = Math.exp(-zeta * w0 * t);
    const osc = Math.cos(wd * t) + ((zeta * w0) / wd) * Math.sin(wd * t);
    return 1 - envelope * osc;
  }

  if (zeta <= 1 + 1e-6) {
    // Critically damped (treat the boundary band as critical for stability).
    const envelope = Math.exp(-w0 * t);
    return 1 - envelope * (1 + w0 * t);
  }

  // Over-damped: two distinct real poles.
  const root = Math.sqrt(zeta * zeta - 1);
  const s1 = -w0 * (zeta - root);
  const s2 = -w0 * (zeta + root);
  const e1 = Math.exp(s1 * t);
  const e2 = Math.exp(s2 * t);
  return 1 - (s2 * e1 - s1 * e2) / (s2 - s1);
}

/**
 * Sample the analytical damped-spring solution into WAAPI-ready data.
 *
 * Returns {@link SpringWaapi} containing:
 *   - `samples`: `{ offset, value }[]` of the progress curve (value in `from`
 *     -> `to` units; may overshoot past `to` for under-damped springs),
 *   - `keyframes`: `Keyframe[]` carrying only `offset` unless `opts.property`
 *     is given (see below),
 *   - `options`: `{ duration, easing: "linear" }` — easing is `linear`
 *     because the spring shape is fully baked into the per-frame samples.
 *
 * Options:
 *   - `steps`  : number of samples (default {@link DEFAULT_STEPS}, min 2).
 *   - `from`/`to` : start/end values (defaults 0 -> 1).
 *   - `property` + `unit`/`format` : project samples onto a CSS property so the
 *     returned `keyframes` are directly consumable by `element.animate`.
 *
 * Examples:
 *   springToWaapiKeyframes(p, { property: "transform", format: (v) => `translateX(${v}px)`, from: 100, to: 0 })
 *   springToWaapiKeyframes(p, { property: "opacity" })
 *
 * The first sample is exactly `{ offset: 0, value: from }`; the last is
 * `{ offset: 1, value: to }` (snapped to the rest position so the animation
 * always finishes precisely on target, even though the analytical value at the
 * truncated settle time may differ by the sub-threshold tail).
 */
export function springToWaapiKeyframes(
  params: SpringParams,
  opts?: {
    steps?: number;
    from?: number;
    to?: number;
    /** CSS property name to emit on each keyframe (e.g. "transform"). */
    property?: string;
    /** Format a numeric value into the property string (overrides `unit`). */
    format?: (value: number) => string;
    /** Unit suffix when `format` is absent (e.g. "px", "%"). */
    unit?: string;
  },
): SpringWaapi {
  const { k, c, m } = normalize(params);
  const w0 = naturalFrequency(k, m);
  const zeta = dampingRatio(k, c, m);

  const steps = Math.max(2, Math.floor(opts?.steps ?? DEFAULT_STEPS));
  const from = opts?.from ?? 0;
  const to = opts?.to ?? 1;
  const span = to - from;

  const duration = springDurationMs(params);
  const durationSeconds = duration / 1000;

  const samples: SpringSample[] = [];
  for (let i = 0; i < steps; i++) {
    const offset = i / (steps - 1);
    let value: number;
    if (i === 0) {
      value = from;
    } else if (i === steps - 1) {
      // Snap the final frame exactly to the target.
      value = to;
    } else {
      const t = offset * durationSeconds;
      const progress = springPosition(t, w0, zeta);
      value = from + span * progress;
    }
    samples.push({ offset, value });
  }

  const keyframes = opts?.property
    ? buildKeyframes(samples, opts.property, opts.format, opts.unit)
    : samples.map((s) => ({ offset: s.offset }) as Keyframe);

  return {
    samples,
    keyframes,
    // Easing is linear: the spring's shape is encoded in the samples
    // themselves, so the browser must interpolate them at constant rate.
    options: { duration, easing: "linear" },
  };
}

/**
 * Project spring {@link SpringSample}s onto a CSS property, returning
 * `Keyframe[]` ready for `element.animate`.
 *
 * @param samples  output of {@link springToWaapiKeyframes}.
 * @param property CSS property name, e.g. "transform" or "opacity".
 * @param format   optional formatter (value -> CSS string); overrides `unit`.
 * @param unit     unit suffix used when no `format` is given (default "").
 */
export function springKeyframesForProperty(
  samples: readonly SpringSample[],
  property: string,
  format?: (value: number) => string,
  unit?: string,
): Keyframe[] {
  return buildKeyframes(samples, property, format, unit);
}

function buildKeyframes(
  samples: readonly SpringSample[],
  property: string,
  format?: (value: number) => string,
  unit?: string,
): Keyframe[] {
  const suffix = unit ?? "";
  return samples.map((s) => {
    const css = format ? format(s.value) : `${s.value}${suffix}`;
    // `Keyframe` indexes string property names, so a dynamic key is valid.
    return { offset: s.offset, [property]: css } as Keyframe;
  });
}

/**
 * Spring preset shape as authored in {@link m3Springs} (a Motion `Transition`
 * augmented with the physical fields). Kept minimal so we depend only on the
 * numeric fields, not on the full Motion type.
 */
export interface SpringPresetLike {
  stiffness?: number;
  damping?: number;
  mass?: number;
}

/**
 * Extract {@link SpringParams} from an {@link m3Springs} entry (or any
 * compatible preset object) for reuse on the Lit / WAAPI side.
 *
 * Missing fields fall back to the `default.spatial` M3 preset values so the
 * result is always a usable, finite spring.
 *
 * @example
 *   springFromPreset(m3Springs.default.spatial)  // { stiffness: 220, damping: 17, mass: 1 }
 */
export function springFromPreset(preset: SpringPresetLike): SpringParams {
  return {
    stiffness: preset.stiffness != null && preset.stiffness > 0 ? preset.stiffness : 220,
    damping: preset.damping != null && preset.damping >= 0 ? preset.damping : 17,
    mass: preset.mass != null && preset.mass > 0 ? preset.mass : DEFAULT_MASS,
  };
}
