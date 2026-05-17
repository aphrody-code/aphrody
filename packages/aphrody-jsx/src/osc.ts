/** @license SPDX-License-Identifier: Apache-2.0 */

// OSC envelope codec for the aphrody-jsx-* sequences. Frames follow the
// terminal-spec layout:
//
//   ESC ] aphrody-jsx-<opcode> ; <fields...> BEL
//
// where ESC = \x1b and BEL = \x07. Field payloads are base64-encoded JSON
// (or raw decimal strings for window-size). The codec is symmetric: encode()
// produces a string ready to write to stdout; parseFrame() recovers the
// opcode + fields from a single envelope (used when listening for input).

import type { JsonNode, JsonPatchOp, OscOpcode } from "./types.ts";

const ESC = "\x1b";
const BEL = "\x07";
const PREFIX = `${ESC}]aphrody-jsx-`;

export interface MountFields {
  id: string;
  tree: JsonNode;
}
export interface UpdateFields {
  id: string;
  patch: JsonPatchOp[];
}
export interface UnmountFields {
  id: string;
}
export interface InputFields {
  id: string;
  input: string;
  key: Record<string, boolean>;
}
export interface WindowSizeFields {
  columns: number;
  rows: number;
}
export interface FocusFields {
  id: string;
  focused: boolean;
}

export type OscFrame =
  | { opcode: "mount"; fields: MountFields }
  | { opcode: "update"; fields: UpdateFields }
  | { opcode: "unmount"; fields: UnmountFields }
  | { opcode: "input"; fields: InputFields }
  | { opcode: "window-size"; fields: WindowSizeFields }
  | { opcode: "focus"; fields: FocusFields };

// Base64 helpers. Use Bun/Node Buffer when present; fall back to btoa/atob
// otherwise. Bun ships Buffer globally so the fast path is the default.
function b64encode(value: string): string {
  if (typeof Buffer !== "undefined") {
    return Buffer.from(value, "utf8").toString("base64");
  }
  // Browser fallback: encode as UTF-8 then btoa.
  const bytes = new TextEncoder().encode(value);
  let binary = "";
  for (let i = 0; i < bytes.length; i++) {
    binary += String.fromCharCode(bytes[i] as number);
  }
  return btoa(binary);
}

function b64decode(value: string): string {
  if (typeof Buffer !== "undefined") {
    return Buffer.from(value, "base64").toString("utf8");
  }
  const binary = atob(value);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return new TextDecoder().decode(bytes);
}

// Encode an OSC frame to its terminal-wire string form.
export function encode(frame: OscFrame): string {
  switch (frame.opcode) {
    case "mount": {
      const payload = b64encode(JSON.stringify(frame.fields.tree));
      return `${PREFIX}mount;${frame.fields.id};${payload}${BEL}`;
    }
    case "update": {
      const payload = b64encode(JSON.stringify(frame.fields.patch));
      return `${PREFIX}update;${frame.fields.id};${payload}${BEL}`;
    }
    case "unmount":
      return `${PREFIX}unmount;${frame.fields.id}${BEL}`;
    case "input": {
      const payload = b64encode(
        JSON.stringify({ input: frame.fields.input, key: frame.fields.key }),
      );
      return `${PREFIX}input;${frame.fields.id};${payload}${BEL}`;
    }
    case "window-size":
      return `${PREFIX}window-size;${frame.fields.columns};${frame.fields.rows}${BEL}`;
    case "focus":
      return `${PREFIX}focus;${frame.fields.id};${frame.fields.focused ? "true" : "false"}${BEL}`;
  }
}

// Parse a single envelope (no trailing data). Returns null on malformed input.
export function parseFrame(envelope: string): OscFrame | null {
  if (!envelope.startsWith(PREFIX) || !envelope.endsWith(BEL)) return null;
  const body = envelope.slice(PREFIX.length, -1);
  const semi = body.indexOf(";");
  const opcode = (semi === -1 ? body : body.slice(0, semi)) as OscOpcode;
  const rest = semi === -1 ? "" : body.slice(semi + 1);
  switch (opcode) {
    case "mount": {
      const [id, payload] = splitOnce(rest, ";");
      if (id === null || payload === null) return null;
      try {
        const tree = JSON.parse(b64decode(payload)) as JsonNode;
        return { opcode: "mount", fields: { id, tree } };
      } catch {
        return null;
      }
    }
    case "update": {
      const [id, payload] = splitOnce(rest, ";");
      if (id === null || payload === null) return null;
      try {
        const patch = JSON.parse(b64decode(payload)) as JsonPatchOp[];
        return { opcode: "update", fields: { id, patch } };
      } catch {
        return null;
      }
    }
    case "unmount":
      if (rest === "") return null;
      return { opcode: "unmount", fields: { id: rest } };
    case "input": {
      const [id, payload] = splitOnce(rest, ";");
      if (id === null || payload === null) return null;
      try {
        const decoded = JSON.parse(b64decode(payload)) as {
          input: string;
          key: Record<string, boolean>;
        };
        return {
          opcode: "input",
          fields: { id, input: decoded.input, key: decoded.key },
        };
      } catch {
        return null;
      }
    }
    case "window-size": {
      const [colsStr, rowsStr] = splitOnce(rest, ";");
      if (colsStr === null || rowsStr === null) return null;
      const columns = Number.parseInt(colsStr, 10);
      const rows = Number.parseInt(rowsStr, 10);
      if (!Number.isFinite(columns) || !Number.isFinite(rows)) return null;
      return { opcode: "window-size", fields: { columns, rows } };
    }
    case "focus": {
      const [id, flag] = splitOnce(rest, ";");
      if (id === null || flag === null) return null;
      return { opcode: "focus", fields: { id, focused: flag === "true" } };
    }
    default:
      return null;
  }
}

function splitOnce(value: string, sep: string): [string, string] | [null, null] {
  const idx = value.indexOf(sep);
  if (idx === -1) return [null, null];
  return [value.slice(0, idx), value.slice(idx + 1)];
}

// Streaming parser. Feed arbitrary chunks; receive OscFrames as they complete.
export class OscFrameParser {
  private buffer = "";

  push(chunk: string): OscFrame[] {
    this.buffer += chunk;
    const out: OscFrame[] = [];
    while (true) {
      const start = this.buffer.indexOf(PREFIX);
      if (start === -1) {
        // Keep at most the last few bytes in case PREFIX straddles a chunk.
        if (this.buffer.length > PREFIX.length) {
          this.buffer = this.buffer.slice(-PREFIX.length);
        }
        break;
      }
      const end = this.buffer.indexOf(BEL, start + PREFIX.length);
      if (end === -1) {
        // Wait for more bytes; discard leading garbage.
        this.buffer = this.buffer.slice(start);
        break;
      }
      const envelope = this.buffer.slice(start, end + 1);
      this.buffer = this.buffer.slice(end + 1);
      const frame = parseFrame(envelope);
      if (frame !== null) out.push(frame);
    }
    return out;
  }
}

// Sink abstraction: anything with a `write` accepting strings/bytes is OK.
export interface OscSink {
  write(chunk: Uint8Array | string): unknown;
}

export function emit(sink: OscSink, frame: OscFrame): void {
  sink.write(encode(frame));
}
