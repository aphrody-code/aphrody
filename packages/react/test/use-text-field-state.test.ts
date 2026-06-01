// SPDX-License-Identifier: Apache-2.0

import { afterEach, describe, expect, it } from "bun:test";
import { createRoot, type Root } from "react-dom/client";
import { flushSync } from "react-dom";
import { renderToStaticMarkup } from "react-dom/server";
import { createElement, type ReactElement } from "react";
import {
  bindTextFieldState,
  syncFromElement,
  useTextFieldState,
  WebTextFieldState,
  type SelectionCapableElement,
} from "../state/use-text-field-state.js";

describe("WebTextFieldState (pure logic)", () => {
  it("constructs with defaults", () => {
    const state = new WebTextFieldState();
    expect(state.text).toBe("");
    expect(state.selection).toEqual({ start: 0, end: 0 });
  });

  it("clamps the initial selection to the text length", () => {
    const state = new WebTextFieldState("abc", { start: 10, end: 99 });
    expect(state.selection).toEqual({ start: 3, end: 3 });
  });

  it("edit() updates text and selection atomically", () => {
    const state = new WebTextFieldState("ab");
    let observed: { text: string; selStart: number } | null = null;
    state.subscribe(() => {
      observed = { text: state.text, selStart: state.selection.start };
    });
    state.edit((buffer) => {
      buffer.text = "abXYZ";
      buffer.selection = { start: 5, end: 5 };
    });
    expect(state.text).toBe("abXYZ");
    expect(state.selection).toEqual({ start: 5, end: 5 });
    // The listener must never observe a half-applied state.
    expect(observed).toEqual({ text: "abXYZ", selStart: 5 });
  });

  it("edit() commits nothing when the block makes no change", () => {
    const state = new WebTextFieldState("hi", { start: 1, end: 1 });
    let calls = 0;
    state.subscribe(() => {
      calls++;
    });
    state.edit(() => {
      /* no-op */
    });
    expect(calls).toBe(0);
    expect(state.text).toBe("hi");
  });

  it("edit() leaves state untouched if the block throws", () => {
    const state = new WebTextFieldState("safe", { start: 2, end: 2 });
    expect(() => {
      state.edit((buffer) => {
        buffer.text = "corrupt";
        throw new Error("boom");
      });
    }).toThrow("boom");
    expect(state.text).toBe("safe");
    expect(state.selection).toEqual({ start: 2, end: 2 });
  });

  it("setSelection() clamps and notifies only on change", () => {
    const state = new WebTextFieldState("hello");
    let calls = 0;
    state.subscribe(() => {
      calls++;
    });
    state.setSelection(1, 3);
    expect(state.selection).toEqual({ start: 1, end: 3 });
    expect(calls).toBe(1);
    // Same range again -> no notification.
    state.setSelection(1, 3);
    expect(calls).toBe(1);
    // Out of range -> clamped to text length.
    state.setSelection(99, 100);
    expect(state.selection).toEqual({ start: 5, end: 5 });
    expect(calls).toBe(2);
  });

  it("setSelection() defaults end to start (caret)", () => {
    const state = new WebTextFieldState("hello");
    state.setSelection(2);
    expect(state.selection).toEqual({ start: 2, end: 2 });
  });

  it("replaceSelection() inserts and moves the caret", () => {
    const state = new WebTextFieldState("foobar", { start: 3, end: 6 });
    state.replaceSelection("X");
    expect(state.text).toBe("fooX");
    expect(state.selection).toEqual({ start: 4, end: 4 });
  });

  it("subscribe() returns a working unsubscribe", () => {
    const state = new WebTextFieldState();
    let calls = 0;
    const off = state.subscribe(() => {
      calls++;
    });
    state.setText("a");
    off();
    state.setText("ab");
    expect(calls).toBe(1);
  });

  it("getSnapshot() returns the current text for useSyncExternalStore", () => {
    const state = new WebTextFieldState("snap");
    expect(state.getSnapshot()).toBe("snap");
    state.setText("snap2");
    expect(state.getSnapshot()).toBe("snap2");
  });
});

describe("bindTextFieldState / syncFromElement", () => {
  function makeElement(
    value = "",
  ): SelectionCapableElement & { setSelectionRange(s: number, e: number): void } {
    return {
      value,
      selectionStart: 0,
      selectionEnd: 0,
      setSelectionRange(start: number, end: number) {
        this.selectionStart = start;
        this.selectionEnd = end;
      },
    };
  }

  it("writes value + selection onto the element", () => {
    const state = new WebTextFieldState("hello", { start: 5, end: 5 });
    const el = makeElement();
    const wrote = bindTextFieldState(state, el);
    expect(wrote).toBe(true);
    expect(el.value).toBe("hello");
    expect(el.selectionStart).toBe(5);
    expect(el.selectionEnd).toBe(5);
  });

  it("is a no-op when element already matches", () => {
    const state = new WebTextFieldState("hi", { start: 2, end: 2 });
    const el = makeElement("hi");
    el.selectionStart = 2;
    el.selectionEnd = 2;
    expect(bindTextFieldState(state, el)).toBe(false);
  });

  it("tolerates a null element", () => {
    const state = new WebTextFieldState();
    expect(bindTextFieldState(state, null)).toBe(false);
    expect(() => syncFromElement(state, undefined)).not.toThrow();
  });

  it("syncFromElement() reads value + caret back atomically", () => {
    const state = new WebTextFieldState();
    const el = makeElement("typed");
    el.selectionStart = 5;
    el.selectionEnd = 5;
    syncFromElement(state, el);
    expect(state.text).toBe("typed");
    expect(state.selection).toEqual({ start: 5, end: 5 });
  });
});

describe("useTextFieldState (SSR render path)", () => {
  it("is SSR-safe and seeds initial value + end caret", () => {
    const captured: { state?: WebTextFieldState } = {};
    function Probe() {
      const state = useTextFieldState("init");
      captured.state = state;
      return createElement("output", null, state.text);
    }
    // renderToStaticMarkup exercises the getServerSnapshot path (no DOM).
    const markup = renderToStaticMarkup(createElement(Probe));
    expect(markup).toContain("init");
    expect(captured.state?.text).toBe("init");
    expect(captured.state?.selection).toEqual({ start: 4, end: 4 });
  });
});

// Client render path: drive React with flushSync + happy-dom, matching the
// package's existing harness (adaptive/transitions tests). The setup.ts preload
// sets IS_REACT_ACT_ENVIRONMENT = false, so we do NOT use act().
describe("useTextFieldState (client render path)", () => {
  let root: Root | null = null;
  let container: HTMLElement | null = null;

  afterEach(() => {
    if (root) {
      flushSync(() => root?.unmount());
      root = null;
    }
    container?.remove();
    container = null;
  });

  function mount(element: ReactElement): void {
    container = document.createElement("div");
    document.body.appendChild(container);
    root = createRoot(container);
    flushSync(() => root?.render(element));
  }

  it("returns a stable instance and re-renders on committed text changes", () => {
    let renders = 0;
    const instances: WebTextFieldState[] = [];
    const captured: { state?: WebTextFieldState } = {};

    function Probe() {
      renders++;
      const state = useTextFieldState("init");
      instances.push(state);
      captured.state = state;
      return createElement("output", null, state.text);
    }

    mount(createElement(Probe));
    expect(captured.state?.text).toBe("init");
    expect(captured.state?.selection).toEqual({ start: 4, end: 4 });
    expect(container?.querySelector("output")?.textContent).toBe("init");

    const rendersAfterMount = renders;
    flushSync(() => {
      captured.state?.setText("changed");
    });
    expect(container?.querySelector("output")?.textContent).toBe("changed");
    expect(renders).toBeGreaterThan(rendersAfterMount);
    // Same instance across renders.
    expect(new Set(instances).size).toBe(1);
  });

  it("does not re-render when an edit leaves the text unchanged", () => {
    let renders = 0;
    const captured: { state?: WebTextFieldState } = {};

    function Probe() {
      renders++;
      const state = useTextFieldState("abc");
      captured.state = state;
      return createElement("output", null, state.text);
    }

    mount(createElement(Probe));
    const baseline = renders;
    // Moving only the caret does not change the text snapshot -> no re-render.
    flushSync(() => {
      captured.state?.setSelection(1, 2);
    });
    expect(renders).toBe(baseline);
    expect(captured.state?.selection).toEqual({ start: 1, end: 2 });
  });
});
