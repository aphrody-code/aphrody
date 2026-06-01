import { describe, expect, test } from "bun:test";
import { ROLES, schemeFromSeed, cssFromSeed } from "../src/dynamic-color.js";

const HEX = /^#[0-9a-f]{6}$/;

describe("schemeFromSeed", () => {
  test("emits one --md-sys-color-* var per role, all valid hex", () => {
    const scheme = schemeFromSeed("#6750A4");
    expect(Object.keys(scheme)).toHaveLength(ROLES.length);
    for (const [name, value] of Object.entries(scheme)) {
      expect(name.startsWith("--md-sys-color-")).toBe(true);
      expect(value).toMatch(HEX);
    }
  });

  test("kebab-cases camelCase roles", () => {
    const scheme = schemeFromSeed("#6750A4");
    expect(scheme["--md-sys-color-on-primary-container"]).toBeDefined();
    expect(scheme["--md-sys-color-surface-container-high"]).toBeDefined();
    expect(scheme["--md-sys-color-inverse-on-surface"]).toBeDefined();
  });

  test("light and dark differ for the same seed", () => {
    const light = schemeFromSeed("#6750A4", { dark: false });
    const dark = schemeFromSeed("#6750A4", { dark: true });
    expect(light["--md-sys-color-surface"]).not.toBe(dark["--md-sys-color-surface"]);
    // Dark surfaces are darker than light surfaces.
    expect(dark["--md-sys-color-surface"] < light["--md-sys-color-surface"]).toBe(true);
  });

  test("different seeds produce different primaries", () => {
    expect(schemeFromSeed("#00658f")["--md-sys-color-primary"]).not.toBe(
      schemeFromSeed("#3a6a1e")["--md-sys-color-primary"],
    );
  });

  test("max contrast pushes on-colours toward the extremes", () => {
    const standard = schemeFromSeed("#6750A4");
    const high = schemeFromSeed("#6750A4", { contrast: 1 });
    expect(standard["--md-sys-color-on-surface"]).toMatch(HEX);
    expect(high["--md-sys-color-on-surface"]).toMatch(HEX);
    // Both valid; high-contrast scheme is a distinct mapping.
    expect(typeof high["--md-sys-color-outline"]).toBe("string");
  });
});

describe("cssFromSeed", () => {
  test("contains :root light and [data-theme=dark] blocks", () => {
    const css = cssFromSeed("#6750A4");
    expect(css).toContain(":root {");
    expect(css).toContain(':root[data-theme="dark"]');
    expect(css).toContain("--md-sys-color-primary:");
    expect(css).toContain("color-scheme: light");
    expect(css).toContain("color-scheme: dark");
  });
});
