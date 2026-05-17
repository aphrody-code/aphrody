/** @license SPDX-License-Identifier: Apache-2.0 */

import { describe, expect, test } from "bun:test";
import {
  encode,
  parseFrame,
  OscFrameParser,
  type OscFrame,
} from "../src/osc.ts";
import type { JsonNode, JsonPatchOp } from "../src/types.ts";

const ESC = "\x1b";
const BEL = "\x07";

describe("osc.encode", () => {
  test("mount frame contains opcode prefix, id, and base64 payload", () => {
    const tree: JsonNode = {
      type: "Box",
      id: "1",
      props: { flexDirection: "column" },
      children: [
        {
          type: "Text",
          id: "2",
          props: { bold: true, color: "m3:primary" },
          children: [{ type: "TEXT_NODE", id: "3", value: "Hi" }],
        },
      ],
    };
    const frame: OscFrame = { opcode: "mount", fields: { id: "root", tree } };
    const wire = encode(frame);
    expect(wire.startsWith(`${ESC}]aphrody-jsx-mount;root;`)).toBe(true);
    expect(wire.endsWith(BEL)).toBe(true);
    // Round-trip back into the same frame.
    const parsed = parseFrame(wire);
    expect(parsed).toEqual(frame);
  });

  test("update frame round-trips a patch list", () => {
    const patch: JsonPatchOp[] = [
      { op: "replace-prop", id: "2", key: "bold", value: false },
      { op: "replace-text", id: "3", value: "Bye" },
      { op: "remove", id: "9" },
    ];
    const frame: OscFrame = { opcode: "update", fields: { id: "root", patch } };
    const wire = encode(frame);
    expect(wire).toContain(`${ESC}]aphrody-jsx-update;root;`);
    expect(parseFrame(wire)).toEqual(frame);
  });

  test("unmount frame has no payload", () => {
    const wire = encode({ opcode: "unmount", fields: { id: "root" } });
    expect(wire).toBe(`${ESC}]aphrody-jsx-unmount;root${BEL}`);
    expect(parseFrame(wire)).toEqual({ opcode: "unmount", fields: { id: "root" } });
  });

  test("window-size frame uses decimal fields", () => {
    const wire = encode({
      opcode: "window-size",
      fields: { columns: 120, rows: 30 },
    });
    expect(wire).toBe(`${ESC}]aphrody-jsx-window-size;120;30${BEL}`);
    expect(parseFrame(wire)).toEqual({
      opcode: "window-size",
      fields: { columns: 120, rows: 30 },
    });
  });

  test("input frame carries input string and key flags", () => {
    const frame: OscFrame = {
      opcode: "input",
      fields: {
        id: "root",
        input: "a",
        key: { upArrow: false, ctrl: true, shift: false },
      },
    };
    const parsed = parseFrame(encode(frame));
    expect(parsed?.opcode).toBe("input");
    if (parsed?.opcode === "input") {
      expect(parsed.fields.input).toBe("a");
      expect(parsed.fields.key.ctrl).toBe(true);
    }
  });

  test("focus frame encodes boolean as literal true/false", () => {
    const wire = encode({
      opcode: "focus",
      fields: { id: "btn-2", focused: true },
    });
    expect(wire).toBe(`${ESC}]aphrody-jsx-focus;btn-2;true${BEL}`);
    expect(parseFrame(wire)).toEqual({
      opcode: "focus",
      fields: { id: "btn-2", focused: true },
    });
  });
});

describe("OscFrameParser", () => {
  test("re-assembles frames across chunk boundaries", () => {
    const wire = encode({
      opcode: "window-size",
      fields: { columns: 80, rows: 24 },
    });
    const parser = new OscFrameParser();
    const half = Math.floor(wire.length / 2);
    expect(parser.push(wire.slice(0, half))).toEqual([]);
    const frames = parser.push(wire.slice(half));
    expect(frames).toHaveLength(1);
    expect(frames[0]?.opcode).toBe("window-size");
  });

  test("ignores leading garbage but recovers next valid frame", () => {
    const wire = encode({ opcode: "unmount", fields: { id: "z" } });
    const parser = new OscFrameParser();
    const frames = parser.push("noise before " + wire + "tail");
    expect(frames).toHaveLength(1);
    expect(frames[0]).toEqual({ opcode: "unmount", fields: { id: "z" } });
  });
});
