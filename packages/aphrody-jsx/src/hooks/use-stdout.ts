/** @license SPDX-License-Identifier: Apache-2.0 */

import { createContext, useContext } from "react";
import type { OscSink } from "../osc.ts";

// StdoutContext exposes the raw OSC sink so advanced components can write
// passthrough bytes (escape sequences, prompts) bypassing the reconciler.
export const StdoutContext = createContext<OscSink | null>(null);

export interface StdoutHandle {
  write(chunk: Uint8Array | string): void;
}

// useStdout() — returns a small handle whose write() forwards to the sink.
export function useStdout(): StdoutHandle {
  const sink = useContext(StdoutContext);
  if (sink === null) {
    throw new Error("@aphrody/jsx: useStdout() called outside a render() tree");
  }
  return {
    write(chunk: Uint8Array | string): void {
      sink.write(chunk);
    },
  };
}
