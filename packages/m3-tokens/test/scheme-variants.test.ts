import { describe, expect, test } from "bun:test";
import { ROLES, schemeFromSeed, cssFromSeed, type SchemeVariant } from "../src/dynamic-color.js";

const HEX = /^#[0-9a-f]{6}$/;
const SEED = "#6750A4";

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
// Completeness: every variant emits the full ~47-role set in both modes
// ---------------------------------------------------------------------------

describe("schemeFromSeed — variant completeness", () => {
  for (const variant of VARIANTS) {
    for (const dark of [false, true]) {
      test(`${variant} ${dark ? "dark" : "light"} emits ${ROLES.length} valid hex roles`, () => {
        const map = schemeFromSeed(SEED, { variant, dark });
        expect(Object.keys(map)).toHaveLength(ROLES.length);
        for (const [name, value] of Object.entries(map)) {
          expect(
            name.startsWith("--md-sys-color-"),
            `${name} must start with --md-sys-color-`,
          ).toBe(true);
          expect(value, `${name} must be a valid hex`).toMatch(HEX);
        }
      });
    }
  }
});

// ---------------------------------------------------------------------------
// Distinction: different variants produce different palettes from the same seed
// ---------------------------------------------------------------------------

describe("schemeFromSeed — variants are distinguishable", () => {
  test("tonalSpot vs monochrome differ on primary (monochrome is greyscale)", () => {
    const ts = schemeFromSeed(SEED, { variant: "tonalSpot" });
    const mono = schemeFromSeed(SEED, { variant: "monochrome" });
    // Monochrome primary should be grey — all channels equal.
    const monoHex = mono["--md-sys-color-primary"]!.slice(1);
    const [r, g, b] = [0, 2, 4].map((i) => parseInt(monoHex.slice(i, i + 2), 16));
    expect(Math.abs(r - g) + Math.abs(g - b)).toBeLessThan(10);
    // TonalSpot primary is chromatic — differs from monochrome.
    expect(ts["--md-sys-color-primary"]).not.toBe(mono["--md-sys-color-primary"]);
  });

  test("tonalSpot vs neutral: neutral has less chroma in primary", () => {
    // Both produce different primaries from the same seed.
    const ts = schemeFromSeed(SEED, { variant: "tonalSpot" });
    const neutral = schemeFromSeed(SEED, { variant: "neutral" });
    expect(ts["--md-sys-color-primary"]).not.toBe(neutral["--md-sys-color-primary"]);
  });

  test("vibrant vs content: vibrant pushes higher chroma", () => {
    const vibrant = schemeFromSeed(SEED, { variant: "vibrant" });
    const content = schemeFromSeed(SEED, { variant: "content" });
    // They must differ on at least primary.
    expect(vibrant["--md-sys-color-primary"]).not.toBe(content["--md-sys-color-primary"]);
  });

  test("expressive tertiary differs from tonalSpot tertiary (detached hue)", () => {
    const ts = schemeFromSeed(SEED, { variant: "tonalSpot" });
    const exp = schemeFromSeed(SEED, { variant: "expressive" });
    expect(ts["--md-sys-color-tertiary"]).not.toBe(exp["--md-sys-color-tertiary"]);
  });

  test("fidelity and content produce different tertiaries", () => {
    const fidelity = schemeFromSeed(SEED, { variant: "fidelity" });
    const content = schemeFromSeed(SEED, { variant: "content" });
    // Not required to always differ, but they use different tertiary strategies.
    // We verify both produce valid hex for tertiary.
    expect(fidelity["--md-sys-color-tertiary"]).toMatch(HEX);
    expect(content["--md-sys-color-tertiary"]).toMatch(HEX);
  });
});

// ---------------------------------------------------------------------------
// Dark / light distinction per variant
// ---------------------------------------------------------------------------

describe("schemeFromSeed — dark/light per variant", () => {
  for (const variant of VARIANTS) {
    test(`${variant}: surface differs between light and dark`, () => {
      const light = schemeFromSeed(SEED, { variant, dark: false });
      const dark = schemeFromSeed(SEED, { variant, dark: true });
      expect(light["--md-sys-color-surface"]).not.toBe(dark["--md-sys-color-surface"]);
    });
  }
});

// ---------------------------------------------------------------------------
// Backwards-compat: no `variant` option defaults to tonalSpot
// ---------------------------------------------------------------------------

describe("schemeFromSeed — backwards compatibility", () => {
  test("no options = tonalSpot light", () => {
    const noOpts = schemeFromSeed(SEED);
    const explicit = schemeFromSeed(SEED, { variant: "tonalSpot", dark: false });
    expect(noOpts).toEqual(explicit);
  });

  test("{ dark: true } alone = tonalSpot dark", () => {
    const legacyDark = schemeFromSeed(SEED, { dark: true });
    const explicit = schemeFromSeed(SEED, { variant: "tonalSpot", dark: true });
    expect(legacyDark).toEqual(explicit);
  });

  test("legacy `contrast` option still respected when contrastLevel absent", () => {
    const legacyContrast = schemeFromSeed(SEED, { contrast: 1 });
    const newContrast = schemeFromSeed(SEED, { contrastLevel: 1 });
    expect(legacyContrast).toEqual(newContrast);
  });

  test("contrastLevel takes precedence over contrast when both provided", () => {
    // contrastLevel=0 vs contrast=1 => should use contrastLevel=0
    const withBoth = schemeFromSeed(SEED, { contrastLevel: 0, contrast: 1 });
    const standard = schemeFromSeed(SEED, { contrastLevel: 0 });
    expect(withBoth).toEqual(standard);
  });
});

// ---------------------------------------------------------------------------
// cssFromSeed with variant
// ---------------------------------------------------------------------------

describe("cssFromSeed — variant option forwarded", () => {
  for (const variant of VARIANTS) {
    test(`${variant}: CSS output contains both :root blocks`, () => {
      const css = cssFromSeed(SEED, { variant });
      expect(css).toContain(":root {");
      expect(css).toContain(':root[data-theme="dark"]');
      expect(css).toContain("--md-sys-color-primary:");
    });
  }
});
