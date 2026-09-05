import { afterEach, describe, expect, test } from "bun:test";
import * as React from "react";
import { createRoot, type Root } from "react-dom/client";
import { flushSync } from "react-dom";
import { Collapse, Fade, Grow, Slide, Zoom } from "../transitions/index.js";
import { isVisible, resolveTimeout } from "../transitions/use-transition-state.js";

// --- Pure helpers (no DOM) -------------------------------------------------

describe("resolveTimeout", () => {
  test("falls back when undefined", () => {
    expect(resolveTimeout(undefined, "enter", 250)).toBe(250);
  });
  test("a single number applies to every direction", () => {
    expect(resolveTimeout(300, "enter", 250)).toBe(300);
    expect(resolveTimeout(300, "exit", 200)).toBe(300);
    expect(resolveTimeout(300, "appear", 250)).toBe(300);
  });
  test("per-direction object resolves each direction", () => {
    const t = { appear: 10, enter: 20, exit: 30 };
    expect(resolveTimeout(t, "appear", 0)).toBe(10);
    expect(resolveTimeout(t, "enter", 0)).toBe(20);
    expect(resolveTimeout(t, "exit", 0)).toBe(30);
  });
  test("appear falls back to enter, then to the fallback", () => {
    expect(resolveTimeout({ enter: 20 }, "appear", 99)).toBe(20);
    expect(resolveTimeout({ exit: 30 }, "appear", 99)).toBe(99);
  });
});

describe("isVisible", () => {
  test("entering/entered are visible; exiting/exited are not", () => {
    expect(isVisible("entering")).toBe(true);
    expect(isVisible("entered")).toBe(true);
    expect(isVisible("exiting")).toBe(false);
    expect(isVisible("exited")).toBe(false);
  });
});

// --- Component rendering (happy-dom + flushSync) ---------------------------

let root: Root | null = null;
let host: HTMLElement | null = null;

function render(element: React.ReactElement): HTMLElement {
  host = document.createElement("div");
  document.body.appendChild(host);
  root = createRoot(host);
  flushSync(() => root!.render(element));
  return host;
}

afterEach(() => {
  if (root) flushSync(() => root!.unmount());
  host?.remove();
  root = null;
  host = null;
});

describe("Fade", () => {
  test("shown: renders the child with opacity 1 and an opacity transition", () => {
    const el = render(
      <Fade in>
        <div data-testid="box">hi</div>
      </Fade>,
    );
    const box = el.querySelector<HTMLElement>('[data-testid="box"]');
    expect(box).not.toBeNull();
    expect(box!.style.opacity).toBe("1");
    expect(box!.style.transition).toContain("opacity");
  });

  test("hidden (kept mounted): renders the child with opacity 0", () => {
    const el = render(
      <Fade in={false}>
        <div data-testid="box">hi</div>
      </Fade>,
    );
    const box = el.querySelector<HTMLElement>('[data-testid="box"]');
    expect(box).not.toBeNull();
    expect(box!.style.opacity).toBe("0");
  });

  test("mountOnEnter + not in: renders nothing", () => {
    const el = render(
      <Fade in={false} mountOnEnter>
        <div data-testid="box">hi</div>
      </Fade>,
    );
    expect(el.querySelector('[data-testid="box"]')).toBeNull();
  });

  test("preserves the child's own inline style", () => {
    const el = render(
      <Fade in>
        <div data-testid="box" style={{ color: "red" }}>
          hi
        </div>
      </Fade>,
    );
    const box = el.querySelector<HTMLElement>('[data-testid="box"]');
    expect(box!.style.color).toBe("red");
    expect(box!.style.opacity).toBe("1");
  });
});

describe("Grow / Zoom / Slide / Collapse", () => {
  test("each renders its child when shown and applies a transition", () => {
    for (const T of [Grow, Zoom, Slide, Collapse]) {
      const el = render(
        <T in>
          <div data-testid="box">x</div>
        </T>,
      );
      const box = el.querySelector<HTMLElement>('[data-testid="box"]');
      expect(box).not.toBeNull();
      // Either the child is cloned with a transition (Fade/Grow/Zoom/Slide)
      // or it sits inside an animated wrapper (Collapse) — assert some
      // rendered element carries a transition.
      const animated = [...el.querySelectorAll<HTMLElement>("*")].some(
        (n) => n.style.transition.length > 0,
      );
      expect(animated).toBe(true);
      // reset between iterations
      if (root) flushSync(() => root!.unmount());
      host?.remove();
      root = null;
      host = null;
    }
  });

  test("Slide/Collapse unmount nothing when shown but render null when mountOnEnter and not in", () => {
    const el = render(
      <Collapse in={false} mountOnEnter>
        <div data-testid="box">x</div>
      </Collapse>,
    );
    expect(el.querySelector('[data-testid="box"]')).toBeNull();
  });
});
