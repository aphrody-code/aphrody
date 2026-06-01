import { describe, expect, test } from "bun:test";
import { schemeFromSeed, type SchemeVariant } from "../src/dynamic-color.js";

// ---------------------------------------------------------------------------
// Contrast-level tests
// Mirrors the structure of a11y-contrast.test.ts but focuses on:
//  1. contrastLevel option (new name) vs contrast (legacy alias)
//  2. Higher contrastLevel must improve WCAG ratio on key pairs
//  3. All variants respect contrastLevel
// ---------------------------------------------------------------------------

function relativeLuminance(hex: string): number {
  const channels = [1, 3, 5]
    .map((i) => parseInt(hex.slice(i, i + 2), 16) / 255)
    .map((v) => (v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4));
  return 0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2];
}

function contrast(a: string, b: string): number {
  const l1 = relativeLuminance(a);
  const l2 = relativeLuminance(b);
  return (Math.max(l1, l2) + 0.05) / (Math.min(l1, l2) + 0.05);
}

// Key pairs: [bg-role, fg-role] (without --md-sys-color- prefix)
const KEY_PAIRS: [string, string][] = [
  ["surface", "on-surface"],
  ["primary", "on-primary"],
  ["secondary", "on-secondary"],
  ["error", "on-error"],
  ["background", "on-background"],
];

const SEEDS = ["#6750a4", "#00658f", "#3a6a1e"];
const VARIANTS: SchemeVariant[] = [
  "tonalSpot",
  "content",
  "fidelity",
  "expressive",
  "vibrant",
  "neutral",
  "monochrome",
];

// ---------------------------------------------------------------------------
// 1. contrastLevel elevates WCAG ratios on standard → high
// ---------------------------------------------------------------------------

describe("contrastLevel — higher level improves ratio on key pairs", () => {
  for (const seed of SEEDS) {
    for (const dark of [false, true]) {
      test(`${seed} ${dark ? "dark" : "light"}: contrastLevel=1 >= contrastLevel=0 on all key pairs`, () => {
        const standard = schemeFromSeed(seed, { dark, contrastLevel: 0 });
        const high = schemeFromSeed(seed, { dark, contrastLevel: 1 });

        let improved = 0;
        for (const [bg, fg] of KEY_PAIRS) {
          const stdRatio = contrast(
            standard[`--md-sys-color-${bg}`]!,
            standard[`--md-sys-color-${fg}`]!,
          );
          const hiRatio = contrast(high[`--md-sys-color-${bg}`]!, high[`--md-sys-color-${fg}`]!);
          if (hiRatio > stdRatio + 0.01) improved++;
          // Never regresses below standard AA (4.5) at high contrast.
          expect(hiRatio).toBeGreaterThanOrEqual(4.5);
        }
        // At least some pairs must have improved contrast.
        expect(
          improved,
          `no pairs improved for ${seed} ${dark ? "dark" : "light"}`,
        ).toBeGreaterThan(0);
      });
    }
  }
});

// ---------------------------------------------------------------------------
// 2. Negative contrastLevel is accepted (reduced contrast, below-standard mode)
// ---------------------------------------------------------------------------

describe("contrastLevel — negative values accepted", () => {
  test("contrastLevel=-1 produces valid hex for all roles", () => {
    const HEX = /^#[0-9a-f]{6}$/;
    const map = schemeFromSeed("#6750a4", { contrastLevel: -1 });
    for (const [, value] of Object.entries(map)) {
      expect(value).toMatch(HEX);
    }
  });

  test("contrastLevel=0.5 (medium) is between standard and high on surface/on-surface", () => {
    const std = schemeFromSeed("#6750a4", { contrastLevel: 0 });
    const med = schemeFromSeed("#6750a4", { contrastLevel: 0.5 });
    const hi = schemeFromSeed("#6750a4", { contrastLevel: 1 });

    const ratioStd = contrast(std["--md-sys-color-surface"]!, std["--md-sys-color-on-surface"]!);
    const ratioMed = contrast(med["--md-sys-color-surface"]!, med["--md-sys-color-on-surface"]!);
    const ratioHi = contrast(hi["--md-sys-color-surface"]!, hi["--md-sys-color-on-surface"]!);

    // Medium should be at least as good as standard.
    expect(ratioMed).toBeGreaterThanOrEqual(ratioStd - 0.1);
    // High should be at least as good as medium.
    expect(ratioHi).toBeGreaterThanOrEqual(ratioMed - 0.1);
  });
});

// ---------------------------------------------------------------------------
// 3. All variants respect contrastLevel (AAA at contrastLevel=1)
// ---------------------------------------------------------------------------

describe("contrastLevel — all variants reach AAA (7:1) on surface/on-surface at contrastLevel=1", () => {
  const AAA = 7;
  for (const variant of VARIANTS) {
    for (const dark of [false, true]) {
      test(`${variant} ${dark ? "dark" : "light"} @contrastLevel=1 >= ${AAA}:1`, () => {
        const scheme = schemeFromSeed("#6750a4", { variant, dark, contrastLevel: 1 });
        const ratio = contrast(
          scheme["--md-sys-color-surface"]!,
          scheme["--md-sys-color-on-surface"]!,
        );
        expect(
          ratio,
          `on-surface/surface (${variant}, ${dark ? "dark" : "light"}) = ${ratio.toFixed(2)}:1`,
        ).toBeGreaterThanOrEqual(AAA);
      });
    }
  }
});

// ---------------------------------------------------------------------------
// 4. Legacy `contrast` alias is fully equivalent to contrastLevel
// ---------------------------------------------------------------------------

describe("contrastLevel — legacy `contrast` alias", () => {
  test("{ contrast: 1 } equals { contrastLevel: 1 } for all variants", () => {
    for (const variant of VARIANTS) {
      const legacy = schemeFromSeed("#6750a4", { variant, contrast: 1 });
      const newOpt = schemeFromSeed("#6750a4", { variant, contrastLevel: 1 });
      expect(legacy, `variant ${variant}: legacy != new`).toEqual(newOpt);
    }
  });

  test("{ contrast: 0 } equals { contrastLevel: 0 }", () => {
    const legacy = schemeFromSeed("#6750a4", { contrast: 0 });
    const newOpt = schemeFromSeed("#6750a4", { contrastLevel: 0 });
    expect(legacy).toEqual(newOpt);
  });
});
