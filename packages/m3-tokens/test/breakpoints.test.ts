import { describe, expect, test } from "bun:test";
import {
  BREAKPOINTS,
  classifyWidth,
  marginFor,
  mediaQuery,
  navigationFor,
  panesFor,
  breakpointsCss,
} from "../src/breakpoints.js";

describe("M3 breakpoints", () => {
  test.each([
    [599, "compact"],
    [600, "medium"],
    [840, "expanded"],
    [1200, "large"],
    [1600, "extra-large"],
  ] as const)("classifyWidth(%i) === %s", (w, c) => expect(classifyWidth(w)).toBe(c));

  test("canonical lower bounds", () =>
    expect([
      BREAKPOINTS.medium,
      BREAKPOINTS.expanded,
      BREAKPOINTS.large,
      BREAKPOINTS["extra-large"],
    ]).toEqual([600, 840, 1200, 1600]));

  test("margins 16/24", () => {
    expect(marginFor("compact")).toBe(16);
    expect(marginFor("expanded")).toBe(24);
  });

  test("adaptive navigation per class", () => {
    expect(navigationFor("compact")).toBe("bar");
    expect(navigationFor("medium")).toBe("rail");
    expect(navigationFor("large")).toBe("rail-expanded");
  });

  test("pane counts 1/1/2/2/3", () =>
    expect(
      ["compact", "medium", "expanded", "large", "extra-large"].map((c) => panesFor(c as never)),
    ).toEqual([1, 1, 2, 2, 3]));

  test("mediaQuery + breakpointsCss", () => {
    expect(mediaQuery("expanded")).toBe("(min-width: 840px)");
    const css = breakpointsCss();
    expect(css).toContain("@custom-media --md-expanded (min-width: 840px)");
    expect(css).toContain("--md-breakpoint-large: 1200px");
  });
});
