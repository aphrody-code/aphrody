/** @license SPDX-License-Identifier: Apache-2.0 */

// Tree serializer + diff engine. Layout (taffy / Yoga-equivalent) runs
// terminal-side in Rust; this file is responsible for converting our
// reconciler's host instances into the JSON shape expected by the renderer
// and for computing minimal patches when props or children change.

import type { JsonElement, JsonNode, JsonPatchOp } from "./types.ts";
import { encodeColor } from "./m3-tokens.ts";

// Host instance representation used by the reconciler. Mirrors the JSON tree
// shape but keeps a mutable parent link so we can walk upward during commits.
export interface HostInstance {
  kind: "element";
  type: JsonElement["type"];
  id: string;
  props: Record<string, unknown>;
  children: HostNode[];
  parent: HostInstance | null;
}

export interface HostTextInstance {
  kind: "text";
  id: string;
  value: string;
  parent: HostInstance | null;
}

export type HostNode = HostInstance | HostTextInstance;

let NEXT_ID = 0;
export function freshId(): string {
  NEXT_ID += 1;
  return String(NEXT_ID);
}

// Test-only: reset the id counter so snapshot tests are deterministic.
export function resetIds(): void {
  NEXT_ID = 0;
}

// Normalize props before serialization. Colors get the `m3:` prefix when they
// match a known token; numeric props pass through verbatim.
export function normalizeProps(
  props: Record<string, unknown>,
): Record<string, unknown> {
  const out: Record<string, unknown> = {};
  for (const [key, raw] of Object.entries(props)) {
    if (key === "children") continue;
    if (raw === undefined || raw === null || raw === false) continue;
    if (key === "color" || key === "backgroundColor" || key === "borderColor") {
      const encoded = encodeColor(String(raw));
      if (encoded !== undefined) out[key] = encoded;
      continue;
    }
    out[key] = raw;
  }
  return out;
}

// Serialize a host instance to the wire JSON shape.
export function serialize(node: HostNode): JsonNode {
  if (node.kind === "text") {
    return { type: "TEXT_NODE", id: node.id, value: node.value };
  }
  return {
    type: node.type,
    id: node.id,
    props: normalizeProps(node.props),
    children: node.children.map(serialize),
  };
}

// Compute a minimal patch list between two prop bags. Only used for element
// instances; text instances use `replace-text`.
export function diffProps(
  id: string,
  prev: Record<string, unknown>,
  next: Record<string, unknown>,
): JsonPatchOp[] {
  const prevNorm = normalizeProps(prev);
  const nextNorm = normalizeProps(next);
  const ops: JsonPatchOp[] = [];
  for (const key of Object.keys(prevNorm)) {
    if (!(key in nextNorm)) ops.push({ op: "remove-prop", id, key });
  }
  for (const [key, value] of Object.entries(nextNorm)) {
    if (!Object.is(prevNorm[key], value)) {
      ops.push({ op: "replace-prop", id, key, value });
    }
  }
  return ops;
}
