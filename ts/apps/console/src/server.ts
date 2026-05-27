// SPDX-License-Identifier: Apache-2.0
//! Bun server for the aphrody console.
//
// Serves the M3 frontend and a small JSON API backed by @aphrody-code/native
// (the whole aphrody CLI, in-process via bun:ffi). The API handlers are
// exported so they can be unit-tested without binding a port; the static
// frontend and `Bun.serve` only run when this file is the entry point.

import { runCaptured, version } from "@aphrody-code/native";

/** Default port; override with PORT. */
export const DEFAULT_PORT = 4317;

/**
 * `POST /api/run` — body `{ args: string[] }` (command arguments, no argv[0]).
 * Returns the captured `{ code, stdout, stderr }` of the real command.
 */
export async function runApi(req: Request): Promise<Response> {
  let body: unknown;
  try {
    body = await req.json();
  } catch {
    return Response.json({ error: "invalid JSON body" }, { status: 400 });
  }
  const args = (body as { args?: unknown }).args;
  if (!Array.isArray(args) || !args.every((arg) => typeof arg === "string")) {
    return Response.json({ error: "args must be an array of strings" }, { status: 400 });
  }
  return Response.json(runCaptured(args as string[]));
}

/** `GET /api/version` — `{ version }` of the loaded native library. */
export function versionApi(): Response {
  return Response.json({ version: version() });
}

if (import.meta.main) {
  const { default: index } = await import("./index.html");
  const port = Number(process.env.PORT ?? DEFAULT_PORT);
  const server = Bun.serve({
    port,
    development: true,
    routes: {
      "/": index,
      "/api/version": versionApi,
      "/api/run": { POST: runApi },
    },
  });
  // Warm the native runtime + context so the first user request is not the
  // cold call (pays the one-time dlopen + runtime spawn + context build now).
  runCaptured(["version", "--json"]);
  console.log(`aphrody console listening on ${server.url}`);
}
