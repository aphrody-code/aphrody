import { describe, expect, test } from "bun:test";
import { schemeFromSeed } from "../src/dynamic-color.js";

// Accessibility guarantee: the Material You scheme must keep every text/icon
// "on-" role legible against its background. WCAG 2.1 AA requires ≥ 4.5:1 for
// normal text (≥ 3:1 for large text / UI). We assert AA (4.5) for the common
// foreground↔background role pairs across a spread of seeds and both modes.

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

// [background, foreground] role pairs that carry text or icons.
const PAIRS: [string, string][] = [
  ["primary", "on-primary"],
  ["secondary", "on-secondary"],
  ["tertiary", "on-tertiary"],
  ["error", "on-error"],
  ["surface", "on-surface"],
  ["surface", "on-surface-variant"],
  ["background", "on-background"],
  ["primary-container", "on-primary-container"],
  ["secondary-container", "on-secondary-container"],
  ["tertiary-container", "on-tertiary-container"],
  ["error-container", "on-error-container"],
  ["inverse-surface", "inverse-on-surface"],
];

const SEEDS = ["#6750a4", "#00658f", "#3a6a1e", "#9a4521", "#b3261e", "#ffd700"];
const AA = 4.5;
const AAA = 7;

describe("dynamic colour — WCAG AA contrast (standard)", () => {
  for (const seed of SEEDS) {
    for (const dark of [false, true]) {
      test(`${seed} ${dark ? "dark" : "light"}: every on-* pair ≥ ${AA}:1`, () => {
        const scheme = schemeFromSeed(seed, { dark });
        for (const [bg, fg] of PAIRS) {
          const ratio = contrast(scheme[`--md-sys-color-${bg}`], scheme[`--md-sys-color-${fg}`]);
          expect(
            ratio,
            `${fg} on ${bg} (${dark ? "dark" : "light"}, seed ${seed}) = ${ratio.toFixed(2)}:1`,
          ).toBeGreaterThanOrEqual(AA);
        }
      });
    }
  }
});

describe("dynamic colour — WCAG AAA contrast (high-contrast mode)", () => {
  for (const seed of SEEDS) {
    for (const dark of [false, true]) {
      test(`${seed} ${dark ? "dark" : "light"} @contrast=1: every on-* pair ≥ ${AAA}:1`, () => {
        const scheme = schemeFromSeed(seed, { dark, contrast: 1 });
        for (const [bg, fg] of PAIRS) {
          const ratio = contrast(scheme[`--md-sys-color-${bg}`], scheme[`--md-sys-color-${fg}`]);
          expect(
            ratio,
            `${fg} on ${bg} (${dark ? "dark" : "light"}, seed ${seed}) = ${ratio.toFixed(2)}:1`,
          ).toBeGreaterThanOrEqual(AAA);
        }
      });
    }
  }
});
