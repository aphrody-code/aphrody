/** @license SPDX-License-Identifier: Apache-2.0 */

import { createContext, useContext } from "react";

// App-level context exposed via useApp(). The render() entry point provides
// the value; consumer components use it to request a clean exit or wait for
// the renderer to finish (useful for tests).
export interface AppContextValue {
  exit(error?: Error): void;
  waitUntilExit(): Promise<void>;
}

export const AppContext = createContext<AppContextValue | null>(null);

// useApp() — `{ exit, waitUntilExit }`. Mirrors Ink's hook.
export function useApp(): AppContextValue {
  const ctx = useContext(AppContext);
  if (ctx === null) {
    throw new Error("@aphrody/jsx: useApp() called outside a render() tree");
  }
  return ctx;
}
