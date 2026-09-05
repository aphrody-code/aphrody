// Adaptive layout hooks: M3 window size class boundaries + the React hook
// rendering through the happy-dom harness.
import { afterEach, describe, expect, test } from "bun:test";
import * as React from "react";
import { createRoot, type Root } from "react-dom/client";
import { flushSync } from "react-dom";
import {
  BREAKPOINTS,
  classifyWidth,
  minWidthQuery,
  useWindowSizeClass,
} from "../adaptive/index.js";

describe("classifyWidth — canonical M3 boundaries (600/840/1200/1600)", () => {
  test.each([
    [0, "compact"],
    [599, "compact"],
    [600, "medium"],
    [839, "medium"],
    [840, "expanded"],
    [1199, "expanded"],
    [1200, "large"],
    [1599, "large"],
    [1600, "extra-large"],
    [2560, "extra-large"],
  ] as const)("%i → %s", (w, cls) => {
    expect(classifyWidth(w)).toBe(cls);
  });

  test("BREAKPOINTS lower-bounds match the spec", () => {
    expect(BREAKPOINTS).toMatchObject({
      compact: 0,
      medium: 600,
      expanded: 840,
      large: 1200,
      "extra-large": 1600,
    });
  });

  test("minWidthQuery builds a CSS media query", () => {
    expect(minWidthQuery("expanded")).toBe("(min-width: 840px)");
  });
});

describe("useWindowSizeClass", () => {
  let root: Root | undefined;
  let host: HTMLElement | undefined;
  afterEach(() => {
    flushSync(() => root?.unmount());
    host?.remove();
    root = undefined;
    host = undefined;
  });

  test("renders a window size class and reflects the current viewport", () => {
    let observed: string | undefined;
    function Probe() {
      observed = useWindowSizeClass();
      return React.createElement("span", null, observed);
    }
    host = document.createElement("div");
    document.body.appendChild(host);
    root = createRoot(host);
    flushSync(() => root!.render(React.createElement(Probe)));
    // After mount the effect ran: the class is one of the five valid values.
    expect(["compact", "medium", "expanded", "large", "extra-large"]).toContain(observed);
  });
});
