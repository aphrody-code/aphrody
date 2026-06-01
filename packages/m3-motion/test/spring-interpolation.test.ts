// SPDX-License-Identifier: Apache-2.0

import { expect, test, describe } from "bun:test";
import {
  springDurationMs,
  springToCssEasing,
  springToWaapiKeyframes,
  springKeyframesForProperty,
  springFromPreset,
  type SpringParams,
} from "../src/spring-interpolation.js";
import { m3Springs } from "../src/springs.js";

// ---------------------------------------------------------------------------
// Regime fixtures (zeta = c / (2*sqrt(k*m)))
// ---------------------------------------------------------------------------

// k=200, m=1 -> 2*sqrt(k*m) = 28.28...
const underDamped: SpringParams = { stiffness: 200, damping: 10, mass: 1 }; // zeta ~ 0.354
const criticallyDamped: SpringParams = { stiffness: 200, damping: 28.2842712, mass: 1 }; // zeta ~ 1
const overDamped: SpringParams = { stiffness: 200, damping: 60, mass: 1 }; // zeta ~ 2.12

function isFinite(n: number): boolean {
  return Number.isFinite(n) && !Number.isNaN(n);
}

// ---------------------------------------------------------------------------
// springDurationMs
// ---------------------------------------------------------------------------

describe("springDurationMs", () => {
  test("returns a finite value within [50, 2000] ms", () => {
    for (const p of [underDamped, criticallyDamped, overDamped]) {
      const d = springDurationMs(p);
      expect(isFinite(d)).toBe(true);
      expect(d).toBeGreaterThanOrEqual(50);
      expect(d).toBeLessThanOrEqual(2000);
    }
  });

  test("within the under-damped regime, settle time grows as damping decreases", () => {
    // k=150, m=1 -> critical damping c_crit = 2*sqrt(k*m) = 24.49.
    // All fixtures below stay under-damped (zeta < 1) where the decay rate
    // is zeta*w0, so less damping == slower settle (monotonic).
    const k = 150;
    const heavy = springDurationMs({ stiffness: k, damping: 22, mass: 1 }); // zeta ~ 0.898
    const medium = springDurationMs({ stiffness: k, damping: 14, mass: 1 }); // zeta ~ 0.572
    const light = springDurationMs({ stiffness: k, damping: 8, mass: 1 }); // zeta ~ 0.327
    expect(light).toBeGreaterThan(medium);
    expect(medium).toBeGreaterThan(heavy);
  });

  test("past critical, increasing damping slows the return (over-damped creep)", () => {
    // Physically, an over-damped spring (zeta > 1) settles SLOWER as damping
    // rises, because the slow real pole dominates the sluggish creep. This is
    // the correct, documented behaviour of springDurationMs.
    const k = 150;
    const justOver = springDurationMs({ stiffness: k, damping: 30, mass: 1 }); // zeta ~ 1.225
    const wayOver = springDurationMs({ stiffness: k, damping: 60, mass: 1 }); // zeta ~ 2.449
    expect(wayOver).toBeGreaterThan(justOver);
  });

  test("clamps an undamped spring to the maximum", () => {
    expect(springDurationMs({ stiffness: 200, damping: 0, mass: 1 })).toBe(2000);
  });

  test("clamps an extremely over-damped/stiff spring to the minimum floor", () => {
    const d = springDurationMs({ stiffness: 5000, damping: 50, mass: 0.5 });
    expect(d).toBeGreaterThanOrEqual(50);
  });

  test("defaults mass to 1 when omitted", () => {
    const withMass = springDurationMs({ stiffness: 200, damping: 12, mass: 1 });
    const noMass = springDurationMs({ stiffness: 200, damping: 12 });
    expect(noMass).toBeCloseTo(withMass, 6);
  });
});

// ---------------------------------------------------------------------------
// springToCssEasing
// ---------------------------------------------------------------------------

describe("springToCssEasing", () => {
  const bezierRe =
    /^cubic-bezier\(\s*(-?\d*\.?\d+)\s*,\s*(-?\d*\.?\d+)\s*,\s*(-?\d*\.?\d+)\s*,\s*(-?\d*\.?\d+)\s*\)$/;

  test("returns a syntactically valid cubic-bezier for all regimes", () => {
    for (const p of [underDamped, criticallyDamped, overDamped]) {
      const e = springToCssEasing(p);
      expect(e.startsWith("cubic-bezier(")).toBe(true);
      expect(bezierRe.test(e)).toBe(true);
    }
  });

  test("bouncy (under-damped) springs fall back to the M3 emphasized decelerate", () => {
    // zeta < 0.85 -> documented fallback (overshoot is impossible in cubic-bezier).
    expect(springToCssEasing(underDamped)).toBe("cubic-bezier(0.05, 0.7, 0.1, 1)");
  });

  test("x control points stay within the valid [0,1] range", () => {
    for (const p of [underDamped, criticallyDamped, overDamped]) {
      const m = bezierRe.exec(springToCssEasing(p));
      expect(m).not.toBeNull();
      const x1 = Number(m![1]);
      const x2 = Number(m![3]);
      expect(x1).toBeGreaterThanOrEqual(0);
      expect(x1).toBeLessThanOrEqual(1);
      expect(x2).toBeGreaterThanOrEqual(0);
      expect(x2).toBeLessThanOrEqual(1);
    }
  });
});

// ---------------------------------------------------------------------------
// springToWaapiKeyframes
// ---------------------------------------------------------------------------

describe("springToWaapiKeyframes", () => {
  test("first sample is at offset 0 == from, last is at offset 1 == to", () => {
    const { samples } = springToWaapiKeyframes(underDamped, { from: 100, to: 0, steps: 40 });
    expect(samples[0]!.offset).toBe(0);
    expect(samples[0]!.value).toBe(100);
    const last = samples[samples.length - 1]!;
    expect(last.offset).toBe(1);
    expect(last.value).toBe(0);
  });

  test("default from/to is 0 -> 1 and reaches close to 1 by the end", () => {
    const { samples } = springToWaapiKeyframes(criticallyDamped, { steps: 80 });
    expect(samples[0]!.value).toBe(0);
    expect(samples[samples.length - 1]!.value).toBeCloseTo(1, 6);
    // penultimate sample should already be near the target (within tolerance)
    const penult = samples[samples.length - 2]!;
    expect(Math.abs(penult.value - 1)).toBeLessThan(0.1);
  });

  test("under-damped spring overshoots the target at least once", () => {
    const { samples } = springToWaapiKeyframes(underDamped, { from: 0, to: 1, steps: 120 });
    // Some interior sample must exceed `to` (1) — the bounce.
    const interior = samples.slice(1, samples.length - 1);
    const overshoots = interior.some((s) => s.value > 1.001);
    expect(overshoots).toBe(true);
  });

  test("critically/over-damped springs do NOT overshoot", () => {
    for (const p of [criticallyDamped, overDamped]) {
      const { samples } = springToWaapiKeyframes(p, { from: 0, to: 1, steps: 120 });
      const maxValue = Math.max(...samples.map((s) => s.value));
      expect(maxValue).toBeLessThanOrEqual(1 + 1e-6);
    }
  });

  test("no NaN/Infinity in samples across all three regimes", () => {
    for (const p of [underDamped, criticallyDamped, overDamped]) {
      const { samples, options } = springToWaapiKeyframes(p, { steps: 60 });
      for (const s of samples) {
        expect(isFinite(s.offset)).toBe(true);
        expect(isFinite(s.value)).toBe(true);
      }
      expect(isFinite(options.duration)).toBe(true);
      expect(options.easing).toBe("linear");
    }
  });

  test("offsets are strictly increasing from 0 to 1", () => {
    const { samples } = springToWaapiKeyframes(overDamped, { steps: 30 });
    for (let i = 1; i < samples.length; i++) {
      expect(samples[i]!.offset).toBeGreaterThan(samples[i - 1]!.offset);
    }
  });

  test("respects a custom step count (min 2 enforced)", () => {
    expect(springToWaapiKeyframes(underDamped, { steps: 10 }).samples).toHaveLength(10);
    expect(springToWaapiKeyframes(underDamped, { steps: 1 }).samples).toHaveLength(2);
  });

  test("emits property keyframes when a property + format is supplied", () => {
    const { keyframes } = springToWaapiKeyframes(underDamped, {
      from: 100,
      to: 0,
      steps: 20,
      property: "transform",
      format: (v) => `translateX(${v}px)`,
    });
    expect(keyframes).toHaveLength(20);
    const first = keyframes[0] as { offset: number; transform: string };
    expect(first.offset).toBe(0);
    expect(first.transform).toBe("translateX(100px)");
    const last = keyframes[keyframes.length - 1] as { offset: number; transform: string };
    expect(last.transform).toBe("translateX(0px)");
  });

  test("emits offset-only keyframes when no property is supplied", () => {
    const { keyframes } = springToWaapiKeyframes(criticallyDamped, { steps: 5 });
    expect(keyframes).toHaveLength(5);
    expect(Object.keys(keyframes[0] as object)).toEqual(["offset"]);
  });

  test("unit suffix path produces plain numeric+unit strings", () => {
    const { keyframes } = springToWaapiKeyframes(criticallyDamped, {
      steps: 4,
      property: "opacity",
    });
    const last = keyframes[keyframes.length - 1] as { opacity: string };
    expect(last.opacity).toBe("1");
  });
});

// ---------------------------------------------------------------------------
// springKeyframesForProperty
// ---------------------------------------------------------------------------

describe("springKeyframesForProperty", () => {
  test("projects samples onto a CSS property with a unit suffix", () => {
    const { samples } = springToWaapiKeyframes(criticallyDamped, { from: 0, to: 24, steps: 6 });
    const kf = springKeyframesForProperty(samples, "width", undefined, "px");
    expect(kf).toHaveLength(6);
    const last = kf[kf.length - 1] as { width: string };
    expect(last.width).toBe("24px");
  });
});

// ---------------------------------------------------------------------------
// springFromPreset
// ---------------------------------------------------------------------------

describe("springFromPreset", () => {
  test("extracts stiffness/damping/mass from an m3Springs entry", () => {
    const p = springFromPreset(m3Springs.default.spatial);
    expect(p.stiffness).toBe(220);
    expect(p.damping).toBe(17);
    expect(p.mass).toBe(1.0);
  });

  test("extracts the fast.effects preset", () => {
    const p = springFromPreset(m3Springs.fast.effects);
    expect(p.stiffness).toBe(350);
    expect(p.damping).toBe(37);
    expect(p.mass).toBe(0.8);
  });

  test("falls back to default.spatial values for an empty preset", () => {
    const p = springFromPreset({});
    expect(p.stiffness).toBe(220);
    expect(p.damping).toBe(17);
    expect(p.mass).toBe(1);
  });

  test("round-trips a preset through the full WAAPI pipeline without NaN", () => {
    const params = springFromPreset(m3Springs.default.spatial);
    const { samples, options } = springToWaapiKeyframes(params, { steps: 40 });
    expect(samples.every((s) => isFinite(s.value))).toBe(true);
    expect(isFinite(options.duration)).toBe(true);
  });
});

// ---------------------------------------------------------------------------
// index re-export
// ---------------------------------------------------------------------------

describe("index re-exports spring-interpolation", () => {
  test("public functions are reachable from the package root", async () => {
    const mod = await import("../src/index.js");
    expect(typeof mod.springDurationMs).toBe("function");
    expect(typeof mod.springToCssEasing).toBe("function");
    expect(typeof mod.springToWaapiKeyframes).toBe("function");
    expect(typeof mod.springKeyframesForProperty).toBe("function");
    expect(typeof mod.springFromPreset).toBe("function");
  });
});
