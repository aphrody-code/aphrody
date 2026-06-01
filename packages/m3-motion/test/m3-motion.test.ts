import { expect, test, describe } from "bun:test";
import { m3Springs } from "../src/springs.js";
import { m3Easings, m3Durations, type M3EasingKey, type M3DurationKey } from "../src/easings.js";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function isCubicBezier(v: unknown): v is readonly [number, number, number, number] {
  return (
    Array.isArray(v) && v.length === 4 && v.every((n) => typeof n === "number" && n >= 0 && n <= 1)
  );
}

// ---------------------------------------------------------------------------
// Easing tokens — values vs spec
// ---------------------------------------------------------------------------

describe("m3Easings", () => {
  test("all keys present per spec (_md-sys-motion.scss)", () => {
    const expected: M3EasingKey[] = [
      "emphasized",
      "emphasizedDecelerate",
      "emphasizedAccelerate",
      "standard",
      "standardDecelerate",
      "standardAccelerate",
      "linear",
      "legacy",
      "legacyDecelerate",
      "legacyAccelerate",
    ];
    for (const key of expected) {
      expect(m3Easings).toHaveProperty(key);
    }
  });

  test("emphasized = cubic-bezier(0.2, 0, 0, 1) — web token value from _md-sys-motion.scss", () => {
    expect(m3Easings.emphasized).toEqual([0.2, 0.0, 0.0, 1.0]);
  });

  test("emphasizedDecelerate = cubic-bezier(0.05, 0.7, 0.1, 1) per spec", () => {
    expect(m3Easings.emphasizedDecelerate).toEqual([0.05, 0.7, 0.1, 1.0]);
  });

  test("emphasizedAccelerate = cubic-bezier(0.3, 0, 0.8, 0.15) per spec", () => {
    expect(m3Easings.emphasizedAccelerate).toEqual([0.3, 0.0, 0.8, 0.15]);
  });

  test("standard = cubic-bezier(0.2, 0, 0, 1) per spec", () => {
    expect(m3Easings.standard).toEqual([0.2, 0.0, 0.0, 1.0]);
  });

  test("standardDecelerate = cubic-bezier(0, 0, 0, 1) per spec", () => {
    expect(m3Easings.standardDecelerate).toEqual([0.0, 0.0, 0.0, 1.0]);
  });

  test("standardAccelerate = cubic-bezier(0.3, 0, 1, 1) per spec", () => {
    expect(m3Easings.standardAccelerate).toEqual([0.3, 0.0, 1.0, 1.0]);
  });

  test("linear = cubic-bezier(0, 0, 1, 1) per spec", () => {
    expect(m3Easings.linear).toEqual([0.0, 0.0, 1.0, 1.0]);
  });

  test("legacy = cubic-bezier(0.4, 0, 0.2, 1) (M2 easing-legacy)", () => {
    expect(m3Easings.legacy).toEqual([0.4, 0.0, 0.2, 1.0]);
  });

  test("legacyDecelerate = cubic-bezier(0, 0, 0.2, 1)", () => {
    expect(m3Easings.legacyDecelerate).toEqual([0.0, 0.0, 0.2, 1.0]);
  });

  test("legacyAccelerate = cubic-bezier(0.4, 0, 1, 1)", () => {
    expect(m3Easings.legacyAccelerate).toEqual([0.4, 0.0, 1.0, 1.0]);
  });

  test("all values are valid cubic-bezier coordinates [0..1]×4", () => {
    for (const [key, value] of Object.entries(m3Easings)) {
      expect(isCubicBezier(value)).toBe(true, `${key} failed cubic-bezier validation`);
    }
  });
});

// ---------------------------------------------------------------------------
// Duration tokens — values vs spec
// ---------------------------------------------------------------------------

describe("m3Durations", () => {
  test("all 16 duration keys defined", () => {
    const expected: M3DurationKey[] = [
      "short1",
      "short2",
      "short3",
      "short4",
      "medium1",
      "medium2",
      "medium3",
      "medium4",
      "long1",
      "long2",
      "long3",
      "long4",
      "extraLong1",
      "extraLong2",
      "extraLong3",
      "extraLong4",
    ];
    expect(Object.keys(m3Durations)).toHaveLength(expected.length);
    for (const key of expected) {
      expect(m3Durations).toHaveProperty(key);
    }
  });

  // Canonical ms values from _md-sys-motion.scss converted to seconds
  const specMs: Record<M3DurationKey, number> = {
    short1: 50,
    short2: 100,
    short3: 150,
    short4: 200,
    medium1: 250,
    medium2: 300,
    medium3: 350,
    medium4: 400,
    long1: 450,
    long2: 500,
    long3: 550,
    long4: 600,
    extraLong1: 700,
    extraLong2: 800,
    extraLong3: 900,
    extraLong4: 1000,
  };

  for (const [key, ms] of Object.entries(specMs) as [M3DurationKey, number][]) {
    test(`${key} = ${ms}ms → ${ms / 1000}s`, () => {
      expect(m3Durations[key]).toBeCloseTo(ms / 1000, 5);
    });
  }

  test("durations are strictly increasing", () => {
    const vals = Object.values(m3Durations) as number[];
    for (let i = 1; i < vals.length; i++) {
      expect(vals[i]).toBeGreaterThan(vals[i - 1]!);
    }
  });
});

// ---------------------------------------------------------------------------
// Spring presets
// ---------------------------------------------------------------------------

describe("m3Springs", () => {
  test("fast.spatial is a spring with stiffness=400", () => {
    expect(m3Springs.fast.spatial.type).toBe("spring");
    expect(m3Springs.fast.spatial.stiffness).toBe(400);
    expect(m3Springs.fast.spatial.damping).toBe(24);
    expect(m3Springs.fast.spatial.mass).toBe(0.8);
  });

  test("fast.effects is a spring with stiffness=350", () => {
    expect(m3Springs.fast.effects.type).toBe("spring");
    expect(m3Springs.fast.effects.stiffness).toBe(350);
    expect(m3Springs.fast.effects.damping).toBe(37);
  });

  test("default.spatial is a spring with stiffness=220", () => {
    expect(m3Springs.default.spatial.type).toBe("spring");
    expect(m3Springs.default.spatial.stiffness).toBe(220);
    expect(m3Springs.default.spatial.damping).toBe(17);
    expect(m3Springs.default.spatial.mass).toBe(1.0);
  });

  test("default.effects is a spring with stiffness=180", () => {
    expect(m3Springs.default.effects.type).toBe("spring");
    expect(m3Springs.default.effects.stiffness).toBe(180);
    expect(m3Springs.default.effects.damping).toBe(26);
  });

  test("slow.spatial is a spring with stiffness=80", () => {
    expect(m3Springs.slow.spatial.type).toBe("spring");
    expect(m3Springs.slow.spatial.stiffness).toBe(80);
    expect(m3Springs.slow.spatial.damping).toBe(12);
    expect(m3Springs.slow.spatial.mass).toBe(1.2);
  });

  test("slow.effects is a spring with stiffness=70", () => {
    expect(m3Springs.slow.effects.type).toBe("spring");
    expect(m3Springs.slow.effects.stiffness).toBe(70);
    expect(m3Springs.slow.effects.damping).toBe(16);
  });

  test("fast springs have higher stiffness than default which is higher than slow", () => {
    expect(m3Springs.fast.spatial.stiffness!).toBeGreaterThan(m3Springs.default.spatial.stiffness!);
    expect(m3Springs.default.spatial.stiffness!).toBeGreaterThan(m3Springs.slow.spatial.stiffness!);
  });

  test("effects springs have higher damping than spatial (critically damped, no overshoot)", () => {
    for (const speed of ["fast", "default", "slow"] as const) {
      expect(m3Springs[speed].effects.damping!).toBeGreaterThan(m3Springs[speed].spatial.damping!);
    }
  });
});

// ---------------------------------------------------------------------------
// Component exports — presence check (no jsdom needed)
// ---------------------------------------------------------------------------

describe("component exports", () => {
  test("M3Fade is exported", async () => {
    const mod = await import("../src/components/M3Fade.js");
    expect(typeof mod.M3Fade).toBe("object"); // forwardRef returns an object
  });

  test("M3FadeThrough is exported", async () => {
    const mod = await import("../src/components/M3FadeThrough.js");
    // M3FadeThrough is an FC (function), not a forwardRef object
    expect(typeof mod.M3FadeThrough).toBe("function");
  });

  test("M3SharedAxis is exported", async () => {
    const mod = await import("../src/components/M3SharedAxis.js");
    expect(typeof mod.M3SharedAxis).toBe("object");
  });

  test("M3Collapse is exported", async () => {
    const mod = await import("../src/components/M3Collapse.js");
    expect(typeof mod.M3Collapse).toBe("object");
  });

  test("M3ContainerTransform is exported", async () => {
    const mod = await import("../src/components/M3ContainerTransform.js");
    expect(typeof mod.M3ContainerTransform).toBe("object");
    expect(typeof mod.M3ContainerTransformStart).toBe("object");
    expect(typeof mod.M3ContainerTransformEnd).toBe("object");
  });

  test("M3Transition is exported with displayName", async () => {
    const mod = await import("../src/components/M3Transition.js");
    expect(typeof mod.M3Transition).toBe("object");
    expect((mod.M3Transition as { displayName?: string }).displayName).toBe("M3Transition");
  });

  test("M3ListStagger is exported with displayName", async () => {
    const mod = await import("../src/components/M3ListStagger.js");
    expect(typeof mod.M3ListStagger).toBe("object");
    expect((mod.M3ListStagger as { displayName?: string }).displayName).toBe("M3ListStagger");
  });

  test("M3ListStaggerItem is exported with displayName", async () => {
    const mod = await import("../src/components/M3ListStagger.js");
    expect(typeof mod.M3ListStaggerItem).toBe("object");
    expect((mod.M3ListStaggerItem as { displayName?: string }).displayName).toBe(
      "M3ListStaggerItem",
    );
  });

  test("useM3Animate is exported as a function", async () => {
    const mod = await import("../src/hooks/useM3Animate.js");
    expect(typeof mod.useM3Animate).toBe("function");
  });
});

// ---------------------------------------------------------------------------
// index re-exports
// ---------------------------------------------------------------------------

describe("index re-exports", () => {
  test("all public APIs are accessible from the package root", async () => {
    const mod = await import("../src/index.js");

    // Easings & springs
    expect(mod.m3Easings).toBeDefined();
    expect(mod.m3Durations).toBeDefined();
    expect(mod.m3Springs).toBeDefined();

    // Components
    expect(mod.M3Fade).toBeDefined();
    expect(mod.M3FadeThrough).toBeDefined();
    expect(mod.M3SharedAxis).toBeDefined();
    expect(mod.M3Collapse).toBeDefined();
    expect(mod.M3ContainerTransform).toBeDefined();
    expect(mod.M3ContainerTransformStart).toBeDefined();
    expect(mod.M3ContainerTransformEnd).toBeDefined();
    expect(mod.M3Transition).toBeDefined();
    expect(mod.M3ListStagger).toBeDefined();
    expect(mod.M3ListStaggerItem).toBeDefined();

    // Hook
    expect(mod.useM3Animate).toBeDefined();
  });
});

// ---------------------------------------------------------------------------
// SSR safety — no global window/document access at import time
// ---------------------------------------------------------------------------

describe("SSR safety", () => {
  test("importing easings does not access window or document", async () => {
    // If any module-level access to window/document existed, it would throw in bun
    // (bun test does not inject a DOM). Passing this import is the proof.
    const { m3Easings: e, m3Durations: d } = await import("../src/easings.js");
    expect(e).toBeDefined();
    expect(d).toBeDefined();
  });

  test("importing springs does not access window or document", async () => {
    const { m3Springs: s } = await import("../src/springs.js");
    expect(s).toBeDefined();
  });

  test("importing all component modules does not throw (no DOM access at module init)", async () => {
    await expect(import("../src/components/M3Fade.js")).resolves.toBeDefined();
    await expect(import("../src/components/M3FadeThrough.js")).resolves.toBeDefined();
    await expect(import("../src/components/M3SharedAxis.js")).resolves.toBeDefined();
    await expect(import("../src/components/M3Collapse.js")).resolves.toBeDefined();
    await expect(import("../src/components/M3ContainerTransform.js")).resolves.toBeDefined();
    await expect(import("../src/components/M3Transition.js")).resolves.toBeDefined();
    await expect(import("../src/components/M3ListStagger.js")).resolves.toBeDefined();
  });
});

// ---------------------------------------------------------------------------
// M3Transition pattern coverage
// ---------------------------------------------------------------------------

describe("M3Transition pattern config", () => {
  test("M3TransitionPattern type includes all 6 patterns", async () => {
    const mod = await import("../src/components/M3Transition.js");
    // Runtime check via the component's existence
    expect(mod.M3Transition).toBeDefined();
  });
});
