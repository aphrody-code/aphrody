// SPDX-License-Identifier: Apache-2.0

/**
 * Hoisted, observable text-field state for Material Design 3 React text fields.
 *
 * This is the Web counterpart of Jetpack Compose's `TextFieldState` /
 * `rememberTextFieldState()` paradigm: a persistent state container holding the
 * current text plus the cursor/selection, mutated through atomic, transactional
 * edits (`state.edit(...)`). It is designed to drive `<md-outlined-text-field>`
 * / `<md-filled-text-field>` (wrappers `MdOutlinedTextField` /
 * `MdFilledTextField`) without spurious re-renders while preserving the caret
 * and selection inside the Lit Shadow DOM.
 *
 * SSR-safe: the module never touches `window`/`document` at import time, and the
 * hook subscribes lazily through `useSyncExternalStore`.
 *
 * @packageDocumentation
 */

import { useState, useSyncExternalStore } from "react";

/** A caret position (`start === end`) or a directed/undirected selection range. */
export interface TextFieldSelection {
  start: number;
  end: number;
}

/** The mutable buffer exposed inside an {@link WebTextFieldState.edit} block. */
export interface TextFieldEditBuffer {
  text: string;
  selection: TextFieldSelection;
}

/** A listener notified after every committed mutation. */
export type TextFieldStateListener = () => void;

function clampSelection(selection: TextFieldSelection, length: number): TextFieldSelection {
  const clamp = (n: number): number => {
    if (!Number.isFinite(n) || n < 0) {
      return 0;
    }
    return n > length ? length : Math.floor(n);
  };
  const start = clamp(selection.start);
  const end = clamp(selection.end);
  // Keep start <= end for an undirected range; callers that need direction can
  // read it back as-is when start <= end already holds.
  return start <= end ? { start, end } : { start: end, end: start };
}

/**
 * Observable, transactional text-field state container.
 *
 * Mirrors Compose `TextFieldState`: the only way to mutate text + selection
 * together is the atomic {@link edit} transaction, so a single notification is
 * dispatched per logical change (text and caret never observed mid-update).
 */
export class WebTextFieldState {
  private _text: string;
  private _selection: TextFieldSelection;
  private readonly _listeners = new Set<TextFieldStateListener>();

  constructor(initialText = "", initialSelection: TextFieldSelection = { start: 0, end: 0 }) {
    this._text = initialText;
    this._selection = clampSelection(initialSelection, initialText.length);
    // Bind so the methods can be passed directly to useSyncExternalStore /
    // addEventListener without losing `this`.
    this.subscribe = this.subscribe.bind(this);
    this.getSnapshot = this.getSnapshot.bind(this);
  }

  /** Current text value. */
  get text(): string {
    return this._text;
  }

  /** Current selection/caret, always clamped to `[0, text.length]`. */
  get selection(): TextFieldSelection {
    return this._selection;
  }

  /**
   * Stable identity snapshot for `useSyncExternalStore`. Returns the raw text;
   * because edits are transactional, text identity changes iff a commit
   * happened, which is the correct "did the store change" signal for the value.
   */
  getSnapshot(): string {
    return this._text;
  }

  /**
   * Move the caret / set the selection without changing text. Out-of-range
   * indices are clamped. Notifies subscribers only when the range changes.
   */
  setSelection(start: number, end: number = start): void {
    const next = clampSelection({ start, end }, this._text.length);
    if (next.start === this._selection.start && next.end === this._selection.end) {
      return;
    }
    this._selection = next;
    this.notify();
  }

  /**
   * Replace the whole text value, re-clamping the existing selection against
   * the new length. Prefer {@link edit} when you also need to move the caret.
   */
  setText(text: string): void {
    if (text === this._text) {
      return;
    }
    this._text = text;
    this._selection = clampSelection(this._selection, text.length);
    this.notify();
  }

  /**
   * Atomic transaction over text + selection. The block receives a mutable
   * buffer initialised from the current state; mutations are validated and
   * committed together, dispatching a single notification. If the block throws,
   * nothing is committed (the state is left untouched).
   *
   * @example
   * state.edit((buffer) => {
   *   buffer.text = buffer.text.toUpperCase();
   *   buffer.selection = { start: buffer.text.length, end: buffer.text.length };
   * });
   */
  edit(block: (buffer: TextFieldEditBuffer) => void): void {
    const buffer: TextFieldEditBuffer = {
      text: this._text,
      selection: { start: this._selection.start, end: this._selection.end },
    };
    block(buffer);
    const nextText = buffer.text;
    const nextSelection = clampSelection(buffer.selection, nextText.length);
    const changed =
      nextText !== this._text ||
      nextSelection.start !== this._selection.start ||
      nextSelection.end !== this._selection.end;
    if (!changed) {
      return;
    }
    this._text = nextText;
    this._selection = nextSelection;
    this.notify();
  }

  /**
   * Convenience editor: replace the current selection (or insert at the caret)
   * with `replacement`, then place the caret right after the inserted text.
   */
  replaceSelection(replacement: string): void {
    this.edit((buffer) => {
      const { start, end } = buffer.selection;
      buffer.text = buffer.text.slice(0, start) + replacement + buffer.text.slice(end);
      const caret = start + replacement.length;
      buffer.selection = { start: caret, end: caret };
    });
  }

  /**
   * Subscribe to commits. Returns an unsubscribe function. Shaped for direct
   * use as the first argument of `useSyncExternalStore`.
   */
  subscribe(listener: TextFieldStateListener): () => void {
    this._listeners.add(listener);
    return () => {
      this._listeners.delete(listener);
    };
  }

  private notify(): void {
    for (const listener of this._listeners) {
      listener();
    }
  }
}

/**
 * Hook recreating Compose's `rememberTextFieldState()`.
 *
 * The {@link WebTextFieldState} instance is created exactly once (lazy
 * `useState` initialiser) and stays stable across re-renders. The component
 * re-renders only when the committed text changes, via `useSyncExternalStore` —
 * chosen over a manual `useEffect` + `forceUpdate` because it is the React 19
 * tearing-safe primitive: it reads the snapshot synchronously during render
 * (so concurrent renders never observe a stale value) and avoids the
 * double-render that a "subscribe in effect then setState" approach incurs on
 * mount. The `getServerSnapshot` argument makes it SSR/hydration-safe.
 *
 * Because selection lives on the same instance, callers that need caret-driven
 * re-renders can subscribe to it themselves; the default snapshot intentionally
 * tracks text only, so moving the caret does not re-render the whole subtree.
 *
 * @param initialValue Initial text; caret is placed at the end of it.
 * @returns The stable, observable state container.
 */
export function useTextFieldState(initialValue = ""): WebTextFieldState {
  const [state] = useState(
    () =>
      new WebTextFieldState(initialValue, {
        start: initialValue.length,
        end: initialValue.length,
      }),
  );
  // Subscribe to text commits. The snapshot is the raw string, so React bails
  // out of re-rendering when an edit leaves the text unchanged.
  useSyncExternalStore(state.subscribe, state.getSnapshot, state.getSnapshot);
  return state;
}

/**
 * A structural view of a focusable text-entry element exposing the standard
 * selection API. Both a raw `<input>`/`<textarea>` and an `<md-*-text-field>`
 * host (which delegates these members to its inner control) satisfy it, so the
 * binding helper does not need to import the React wrapper or the Lit element.
 */
export interface SelectionCapableElement {
  value: string;
  selectionStart: number | null;
  selectionEnd: number | null;
  setSelectionRange?(start: number, end: number): void;
}

/**
 * Push the current {@link WebTextFieldState} value + selection onto a live
 * `<md-*-text-field>` (or raw input) element, preserving the caret/selection
 * inside the Lit Shadow DOM. Safe to call from a React effect after each
 * commit; it writes `value` only when it differs to avoid clobbering IME
 * composition, then restores the selection range.
 *
 * @returns `true` if anything was written, `false` if already in sync.
 */
export function bindTextFieldState(
  state: WebTextFieldState,
  element: SelectionCapableElement | null | undefined,
): boolean {
  if (!element) {
    return false;
  }
  let wrote = false;
  if (element.value !== state.text) {
    element.value = state.text;
    wrote = true;
  }
  const { start, end } = state.selection;
  if (element.selectionStart !== start || element.selectionEnd !== end) {
    if (typeof element.setSelectionRange === "function") {
      element.setSelectionRange(start, end);
    } else {
      element.selectionStart = start;
      element.selectionEnd = end;
    }
    wrote = true;
  }
  return wrote;
}

/**
 * Read a `<md-*-text-field>` (or raw input) element's current value + selection
 * back into a {@link WebTextFieldState} as a single atomic edit. Use this as the
 * `onInput`/`onSelect` handler bridge so user typing stays the source of truth.
 */
export function syncFromElement(
  state: WebTextFieldState,
  element: SelectionCapableElement | null | undefined,
): void {
  if (!element) {
    return;
  }
  const caretStart = element.selectionStart;
  const caretEnd = element.selectionEnd;
  state.edit((buffer) => {
    buffer.text = element.value;
    const start = caretStart ?? buffer.text.length;
    const end = caretEnd ?? start;
    buffer.selection = { start, end };
  });
}
