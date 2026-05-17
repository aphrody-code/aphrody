/** @license SPDX-License-Identifier: Apache-2.0 */

import { createContext, useContext, useEffect } from "react";
import type { InputHandler, InputKey } from "../types.ts";

// InputContext — pub/sub fed by render(). Each `aphrody-jsx-input` OSC frame
// arriving from the terminal is converted into a (string, InputKey) pair and
// fanned out to every subscribed useInput() handler.
export interface InputContextValue {
  subscribe(handler: InputHandler): () => void;
  isRawModeSupported: boolean;
}

export const InputContext = createContext<InputContextValue | null>(null);

// Constructs a fresh InputKey with every flag set to false. The frame parser
// overrides individual flags as needed before invoking subscribers.
export function emptyKey(): InputKey {
  return {
    upArrow: false,
    downArrow: false,
    leftArrow: false,
    rightArrow: false,
    return: false,
    escape: false,
    tab: false,
    backspace: false,
    delete: false,
    pageUp: false,
    pageDown: false,
    home: false,
    end: false,
    ctrl: false,
    shift: false,
    meta: false,
  };
}

// useInput(handler) — invoke `handler(input, key)` for every keypress the
// terminal forwards to our mount region.
export function useInput(handler: InputHandler, options?: { isActive?: boolean }): void {
  const ctx = useContext(InputContext);
  const active = options?.isActive ?? true;
  useEffect(() => {
    if (!active) return undefined;
    if (ctx === null) return undefined;
    return ctx.subscribe(handler);
  }, [ctx, handler, active]);
}
