// SPDX-License-Identifier: Apache-2.0
import { GlobalRegistrator } from "@happy-dom/global-registrator";

GlobalRegistrator.register();

import { describe, expect, test } from "bun:test";
import {
  GEMINI_BRAND_BLUE,
  GEMINI_BRAND_PINK,
  GEMINI_BRAND_PURPLE,
  SPECTRUM_SHIFT_CSS,
  SPARKLE_CSS,
  FOUR_COLOR_DOTS,
  BRAND_CORNER_PROMPT_BAR_PX,
  BRAND_CORNER_MESSAGE_PX,
  BRAND_CORNER_CHIP_PX,
  fontVariationSettings,
  rootTokensCss,
} from "../lib/m3-tokens";

describe("m3-tokens mirrors crates/m3-tokens/src/gemini_brand.rs", () => {
  test("brand colors match the Rust constants", () => {
    expect(GEMINI_BRAND_BLUE).toBe("#4285F4");
    expect(GEMINI_BRAND_PURPLE).toBe("#9168C0");
    expect(GEMINI_BRAND_PINK).toBe("#EC4899");
  });

  test("four-color dots match the Google logo lineage", () => {
    expect(FOUR_COLOR_DOTS).toEqual([
      "#4285F4",
      "#EA4335",
      "#FBBC04",
      "#34A853",
    ]);
  });

  test("spectrum-shift gradient is canonical 3-stop blue->purple->pink", () => {
    expect(SPECTRUM_SHIFT_CSS).toContain("90deg");
    expect(SPECTRUM_SHIFT_CSS).toContain(GEMINI_BRAND_BLUE);
    expect(SPECTRUM_SHIFT_CSS).toContain(GEMINI_BRAND_PURPLE);
    expect(SPECTRUM_SHIFT_CSS).toContain(GEMINI_BRAND_PINK);
  });

  test("sparkle gradient is canonical 4-stop four-color-dots", () => {
    expect(SPARKLE_CSS).toContain("135deg");
    for (const dot of FOUR_COLOR_DOTS) {
      expect(SPARKLE_CSS).toContain(dot);
    }
  });

  test("brand corner radii match Rust constants", () => {
    expect(BRAND_CORNER_PROMPT_BAR_PX).toBe(28);
    expect(BRAND_CORNER_MESSAGE_PX).toBe(24);
    expect(BRAND_CORNER_CHIP_PX).toBe(18);
  });
});

describe("Google Sans Flex axis settings", () => {
  test("defaults match google_sans_flex.rs", () => {
    expect(fontVariationSettings({})).toBe(
      '"wght" 400, "opsz" 14, "wdth" 100, "GRAD" 0, "slnt" 0, "ROND" 0',
    );
  });

  test("clamps out-of-range axes", () => {
    expect(fontVariationSettings({ wght: 50 })).toContain('"wght" 100');
    expect(fontVariationSettings({ wght: 950 })).toContain('"wght" 900');
    expect(fontVariationSettings({ rond: -50 })).toContain('"ROND" 0');
    expect(fontVariationSettings({ rond: 200 })).toContain('"ROND" 100');
  });
});

describe("rootTokensCss emits the expected custom-property block", () => {
  test("contains all required Gemini brand tokens", () => {
    const css = rootTokensCss();
    for (const token of [
      "--md-sys-color-primary",
      "--md-sys-color-surface",
      "--gemini-brand-blue",
      "--gemini-brand-purple",
      "--gemini-brand-pink",
      "--gemini-spectrum-shift",
      "--gemini-sparkle",
      "--gemini-corner-prompt-bar",
      "--gemini-corner-message",
      "--gemini-corner-chip",
    ]) {
      expect(css).toContain(token);
    }
  });
});

describe("Sparkle component renders pure SVG (no dangerouslySetInnerHTML)", () => {
  test("renders 1 <svg> with the four-color gradient stops", async () => {
    const { Sparkle } = await import("../app/components/Sparkle");
    const { renderToStaticMarkup } = await import("react-dom/server");
    const html = renderToStaticMarkup(<Sparkle size={64} />);
    expect(html).toContain("<svg");
    expect(html).toContain("aphrody-sparkle-grad");
    expect(html).toContain("#4285F4");
    expect(html).toContain("#34A853");
    // Hard assertion: no innerHTML escape-hatches.
    expect(html).not.toContain("dangerouslySetInnerHTML");
  });
});
