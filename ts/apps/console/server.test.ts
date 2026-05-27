// SPDX-License-Identifier: Apache-2.0
//! API tests for the aphrody console — exercise the real native-backed
//! handlers (no port bound). Skipped if the aphrody-ffi cdylib is not built
//! (the @aphrody-code/native import `dlopen`s it at load).

import { describe, expect, test } from "bun:test";

type ServerModule = typeof import("./src/server.ts");

let server: ServerModule | null = null;
try {
  server = await import("./src/server.ts");
} catch {
  server = null;
}

const suite = server ? describe : describe.skip;

suite("aphrody console API", () => {
  test("GET /api/version returns the real version", async () => {
    const mod = server as ServerModule;
    const json = (await mod.versionApi().json()) as { version: string };
    expect(json.version.length).toBeGreaterThan(0);
  });

  test("POST /api/run runs a real command and captures output", async () => {
    const mod = server as ServerModule;
    const req = new Request("http://localhost/api/run", {
      method: "POST",
      body: JSON.stringify({ args: ["version", "--json"] }),
    });
    const json = (await (await mod.runApi(req)).json()) as { code: number; stdout: string };
    expect(json.code).toBe(0);
    const parsed = JSON.parse(json.stdout) as { version: string };
    expect(parsed.version.length).toBeGreaterThan(0);
  });

  test("POST /api/run rejects a non-array args payload", async () => {
    const mod = server as ServerModule;
    const req = new Request("http://localhost/api/run", {
      method: "POST",
      body: JSON.stringify({ args: "not-an-array" }),
    });
    expect((await mod.runApi(req)).status).toBe(400);
  });
});
