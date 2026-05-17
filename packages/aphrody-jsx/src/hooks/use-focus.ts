/** @license SPDX-License-Identifier: Apache-2.0 */

import { createContext, useContext, useEffect, useId, useState } from "react";

// FocusContext lets the focus manager track an ordered list of focusable
// nodes and route focus events to the correct one. The id used here is a
// React-internal id (useId), distinct from the host-instance id used in OSC
// frames — they don't need to align because focus routing happens before
// the OSC layer is involved.
export interface FocusContextValue {
  register(id: string, autoFocus: boolean): void;
  unregister(id: string): void;
  isFocused(id: string): boolean;
  subscribe(listener: () => void): () => void;
  focusNext(): void;
  focusPrevious(): void;
  focus(id: string): void;
  enableFocus(): void;
  disableFocus(): void;
}

export const FocusContext = createContext<FocusContextValue | null>(null);

export interface UseFocusOptions {
  autoFocus?: boolean;
  isActive?: boolean;
}

export interface UseFocusReturn {
  isFocused: boolean;
}

// useFocus({ autoFocus }) — opts the calling node into the focus ring.
export function useFocus(options: UseFocusOptions = {}): UseFocusReturn {
  const { autoFocus = false, isActive = true } = options;
  const ctx = useContext(FocusContext);
  const id = useId();
  const [focused, setFocused] = useState(false);

  useEffect(() => {
    if (ctx === null || !isActive) return undefined;
    ctx.register(id, autoFocus);
    setFocused(ctx.isFocused(id));
    const unsub = ctx.subscribe(() => setFocused(ctx.isFocused(id)));
    return () => {
      unsub();
      ctx.unregister(id);
    };
  }, [ctx, id, autoFocus, isActive]);

  return { isFocused: focused };
}
