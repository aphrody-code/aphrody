// SPDX-License-Identifier: MIT
// Minimal zod-to-JSON-Schema converter for the MUI MCP server.
// Handles only the subset of zod types used by this server's tool schemas.

import { z } from "zod";

type JsonSchema = Record<string, unknown>;

export function zodToJsonSchema(schema: z.ZodTypeAny): JsonSchema {
  return convert(schema);
}

function convert(schema: z.ZodTypeAny): JsonSchema {
  if (schema instanceof z.ZodObject) {
    const shape = schema.shape as Record<string, z.ZodTypeAny>;
    const properties: Record<string, JsonSchema> = {};
    const required: string[] = [];

    for (const [key, value] of Object.entries(shape)) {
      properties[key] = convert(value);
      if (!(value instanceof z.ZodOptional)) {
        required.push(key);
      }
    }

    const result: JsonSchema = { type: "object", properties };
    if (required.length > 0) result["required"] = required;
    return result;
  }

  if (schema instanceof z.ZodArray) {
    return {
      type: "array",
      items: convert(schema.element),
    };
  }

  if (schema instanceof z.ZodString) {
    const out: JsonSchema = { type: "string" };
    const desc = schema.description;
    if (desc) out["description"] = desc;
    return out;
  }

  if (schema instanceof z.ZodNumber) {
    const out: JsonSchema = { type: "number" };
    const desc = schema.description;
    if (desc) out["description"] = desc;
    return out;
  }

  if (schema instanceof z.ZodBoolean) {
    const out: JsonSchema = { type: "boolean" };
    const desc = schema.description;
    if (desc) out["description"] = desc;
    return out;
  }

  if (schema instanceof z.ZodOptional) {
    return convert(schema.unwrap());
  }

  if (schema instanceof z.ZodNullable) {
    const inner = convert(schema.unwrap());
    return { anyOf: [inner, { type: "null" }] };
  }

  if (schema instanceof z.ZodEnum) {
    return { type: "string", enum: schema.options };
  }

  if (schema instanceof z.ZodLiteral) {
    return { const: schema.value };
  }

  if (schema instanceof z.ZodUnion) {
    return {
      anyOf: (schema.options as z.ZodTypeAny[]).map(convert),
    };
  }

  // Fallback: any type
  return {};
}
