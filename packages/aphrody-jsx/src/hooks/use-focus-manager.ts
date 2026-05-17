/** @license SPDX-License-Identifier: Apache-2.0 */

import { useContext } from "react";
import { FocusContext } from "./use-focus.ts";

export interface FocusManager {
  focus(id: string): void;
  focusNext(): void;
  focusPrevious(): void;
  enableFocus(): void;
  disableFocus(): void;
}

// useFocusManager() — imperative handle exposed to components that need to
// drive the focus ring programmatically (e.g. a form wizard's "Next" button).
export function useFocusManager(): FocusManager {
  const ctx = useContext(FocusContext);
  if (ctx === null) {
    throw new Error(
      "@aphrody/jsx: useFocusManager() called outside a render() tree",
    );
  }
  return {
    focus: ctx.focus,
    focusNext: ctx.focusNext,
    focusPrevious: ctx.focusPrevious,
    enableFocus: ctx.enableFocus,
    disableFocus: ctx.disableFocus,
  };
}
