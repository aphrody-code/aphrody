// Headless unit smoke run by `bun test`. The DOM/component behaviour is covered
// by the real-Chromium bxc gate (`bun run test:browser`); here we only assert
// the pure, framework-free utilities that don't need a browser.
import { test, expect } from "bun:test";
import {
  defaultTransformPseudoClasses,
  getTransformedPseudoClass,
} from "../testing/transform-pseudo-classes.js";

test("getTransformedPseudoClass strips the colon and prefixes an underscore", () => {
  expect(getTransformedPseudoClass(":hover")).toBe("_hover");
  expect(getTransformedPseudoClass(":focus-visible")).toBe("_focus-visible");
});

test("defaultTransformPseudoClasses lists the interactive pseudo-classes", () => {
  expect(defaultTransformPseudoClasses).toContain(":hover");
  expect(defaultTransformPseudoClasses).toContain(":focus");
  expect(defaultTransformPseudoClasses).toContain(":active");
  expect(defaultTransformPseudoClasses.length).toBeGreaterThanOrEqual(13);
});
