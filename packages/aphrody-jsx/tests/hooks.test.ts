/** @license SPDX-License-Identifier: Apache-2.0 */

import { describe, expect, test } from "bun:test";
import { createElement, useEffect } from "react";
import { render } from "../src/render.ts";
import { useApp } from "../src/hooks/use-app.ts";
import { useInput } from "../src/hooks/use-input.ts";
import { encode } from "../src/osc.ts";

function captureSink(): { write(chunk: Uint8Array | string): void; chunks: string[] } {
  const chunks: string[] = [];
  return {
    chunks,
    write(chunk): void {
      chunks.push(typeof chunk === "string" ? chunk : new TextDecoder().decode(chunk));
    },
  };
}

// Build an async-iterable input source that waits for React to mount before
// emitting a single OSC input frame, then closes.
async function* singleInputFrame(input: string, delayMs = 50): AsyncIterable<string> {
  await new Promise((resolve) => setTimeout(resolve, delayMs));
  yield encode({
    opcode: "input",
    fields: {
      id: "root",
      input,
      key: {},
    },
  });
}

describe("useInput", () => {
  test("invokes handler when an aphrody-jsx-input OSC frame arrives", async () => {
    const sink = captureSink();
    const received: string[] = [];
    let appRef: ReturnType<typeof useApp> | null = null;

    function Probe(): null {
      const app = useApp();
      appRef = app;
      useInput((input) => {
        received.push(input);
        app.exit();
      });
      useEffect(() => {
        /* keep alive */
      }, []);
      return null;
    }

    const instance = render(createElement(Probe), {
      output: sink,
      input: singleInputFrame("q"),
      exitOnCtrlC: false,
    });

    // Wait for the exit triggered inside the handler.
    await Promise.race([
      instance.waitUntilExit(),
      new Promise<void>((resolve) => setTimeout(resolve, 500)),
    ]);

    expect(received).toEqual(["q"]);
    expect(appRef).not.toBeNull();
  });
});

describe("useApp", () => {
  test("exit() resolves waitUntilExit()", async () => {
    const sink = captureSink();
    function Auto(): null {
      const app = useApp();
      useEffect(() => {
        app.exit();
      }, [app]);
      return null;
    }
    const instance = render(createElement(Auto), {
      output: sink,
      exitOnCtrlC: false,
    });
    await instance.waitUntilExit();
    expect(true).toBe(true);
  });
});
