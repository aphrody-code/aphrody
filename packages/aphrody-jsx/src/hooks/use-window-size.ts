/** @license SPDX-License-Identifier: Apache-2.0 */

import { createContext, useContext, useEffect, useState } from "react";
import type { WindowSize } from "../types.ts";

// WindowSizeContext — pushed by render() whenever a `window-size` OSC frame
// arrives from the terminal. We seed it with a sensible default so components
// rendered before the first event still get usable values.
export interface WindowSizeContextValue {
  current: WindowSize;
  subscribe(listener: (size: WindowSize) => void): () => void;
}

export const WindowSizeContext = createContext<WindowSizeContextValue | null>(null);

// useWindowSize() — returns the live `{ columns, rows }` and re-renders the
// caller whenever the terminal reports a resize.
export function useWindowSize(): WindowSize {
  const ctx = useContext(WindowSizeContext);
  if (ctx === null) {
    throw new Error(
      "@aphrody/jsx: useWindowSize() called outside a render() tree",
    );
  }
  const [size, setSize] = useState<WindowSize>(ctx.current);
  useEffect(() => ctx.subscribe(setSize), [ctx]);
  return size;
}
