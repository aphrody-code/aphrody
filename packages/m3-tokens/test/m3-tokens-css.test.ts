import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

// The runtime token asset consumed by Tailwind (`@theme`) and any host that
// needs the M3 families @aphrody-code/material-web does NOT project as runtime vars
// (typescale/shape/elevation/motion/state). This test pins that contract.
const css = readFileSync(fileURLToPath(new URL("../src/m3-tokens.css", import.meta.url)), "utf8");

describe("m3-tokens.css runtime asset", () => {
  test("declares the non-colour M3 token families", () => {
    for (const family of [
      "--md-sys-typescale",
      "--md-sys-shape",
      "--md-sys-elevation",
      "--md-sys-motion",
      "--md-sys-state",
    ]) {
      expect(css).toContain(family);
    }
  });

  test("exposes the variables on :root", () => {
    expect(css).toMatch(/:root\s*\{/);
  });

  test("declares a substantial, stable number of tokens", () => {
    const count = (css.match(/^\s*--md-sys-[a-z-]+\s*:/gm) ?? []).length;
    // ~37 `--md-sys-*` definitions (the documented "58 vars" also counts the
    // `@theme` Tailwind aliases). Guard against accidental gutting.
    expect(count).toBeGreaterThanOrEqual(30);
  });

  test("every declared custom property has a non-empty value", () => {
    const decls = css.match(/--md-sys-[a-z-]+\s*:[^;]*;/g) ?? [];
    expect(decls.length).toBeGreaterThan(0);
    for (const d of decls) {
      const value = d.slice(d.indexOf(":") + 1, -1).trim();
      expect(value.length).toBeGreaterThan(0);
    }
  });
});
