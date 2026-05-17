/** @license SPDX-License-Identifier: Apache-2.0 */

import { describe, expect, test } from "bun:test";
import { createElement } from "react";
import { render } from "../src/render.ts";
import { Box } from "../src/components/Box.tsx";
import { Text } from "../src/components/Text.tsx";
import { OscFrameParser } from "../src/osc.ts";

function captureSink(): { write(chunk: Uint8Array | string): void; chunks: string[] } {
  const chunks: string[] = [];
  return {
    chunks,
    write(chunk): void {
      chunks.push(typeof chunk === "string" ? chunk : new TextDecoder().decode(chunk));
    },
  };
}

// React's concurrent renderer commits via microtasks + the scheduler queue.
// Yielding once isn't always enough on the rerender path because the scheduler
// rebatches; double-yield so concurrent + transition lanes both drain.
async function flush(ms = 100): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, ms));
  await new Promise((resolve) => setTimeout(resolve, 0));
  await new Promise((resolve) => setTimeout(resolve, ms));
}

describe("render", () => {
  test("hello tree emits a single mount frame with the expected structure", async () => {
    const sink = captureSink();
    const element = createElement(
      Box,
      { flexDirection: "column", padding: 1 },
      createElement(Text, { bold: true, color: "primary" }, "Hello aphrody-jsx"),
    );
    const instance = render(element, { output: sink, exitOnCtrlC: false });
    await flush();

    const parser = new OscFrameParser();
    const frames = parser.push(sink.chunks.join(""));
    expect(frames.length).toBeGreaterThanOrEqual(1);

    const mount = frames.find((f) => f.opcode === "mount");
    expect(mount).toBeDefined();
    if (mount?.opcode === "mount") {
      const root = mount.fields.tree;
      expect(root.type).toBe("Root");
      if (root.type !== "TEXT_NODE") {
        expect(root.children.length).toBe(1);
        const box = root.children[0];
        if (box && box.type !== "TEXT_NODE") {
          expect(box.type).toBe("Box");
          // Padding shorthand expanded to per-edge values.
          expect(box.props.paddingTop).toBe(1);
          expect(box.props.paddingBottom).toBe(1);
          expect(box.props.paddingLeft).toBe(1);
          expect(box.props.paddingRight).toBe(1);
          expect(box.props.flexDirection).toBe("column");
          expect(box.children.length).toBe(1);
          const text = box.children[0];
          if (text && text.type !== "TEXT_NODE") {
            expect(text.type).toBe("Text");
            expect(text.props.bold).toBe(true);
            // Color tokens carry the `m3:` marker for re-resolution renderer-side.
            expect(text.props.color).toBe("m3:primary");
            const inner = text.children[0];
            expect(inner?.type).toBe("TEXT_NODE");
            if (inner?.type === "TEXT_NODE") {
              expect(inner.value).toBe("Hello aphrody-jsx");
            }
          }
        }
      }
    }

    instance.unmount();
  });

  test("rerender emits an update patch, not a fresh mount", async () => {
    const sink = captureSink();
    const instance = render(
      createElement(Text, { color: "primary" }, "first"),
      { output: sink, exitOnCtrlC: false },
    );
    await flush();
    const initialChunks = sink.chunks.length;
    instance.rerender(createElement(Text, { color: "secondary" }, "second"));
    await flush();

    const after = sink.chunks.slice(initialChunks).join("");
    const parser = new OscFrameParser();
    const frames = parser.push(after);
    expect(frames.some((f) => f.opcode === "update")).toBe(true);
    expect(frames.every((f) => f.opcode !== "mount")).toBe(true);

    instance.unmount();
  });
});
