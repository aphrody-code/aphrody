// SPDX-License-Identifier: Apache-2.0
import { describe, expect, test } from "bun:test";

// We hit the route handlers directly (no Next.js server) so we don't have
// to spin up a dev server inside `bun test`. The handlers are pure
// Web-standard (Request -> Response) functions.

describe("POST /api/chat", () => {
  test("rejects missing prompt with 400", async () => {
    const { POST } = await import("../app/api/chat/route");
    const req = new Request("http://localhost/api/chat", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({}),
    });
    const res = await POST(req);
    expect(res.status).toBe(400);
    const body = (await res.json()) as { error: string };
    expect(body.error).toContain("prompt required");
  });

  test("rejects non-JSON body with 400", async () => {
    const { POST } = await import("../app/api/chat/route");
    const req = new Request("http://localhost/api/chat", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: "not-json",
    });
    const res = await POST(req);
    expect(res.status).toBe(400);
  });
});

describe("POST /api/stt", () => {
  test("rejects empty form-data with 400", async () => {
    const { POST } = await import("../app/api/stt/route");
    const form = new FormData();
    const req = new Request("http://localhost/api/stt", {
      method: "POST",
      body: form,
    });
    const res = await POST(req);
    expect(res.status).toBe(400);
    const body = (await res.json()) as { error: string };
    expect(body.error).toContain("missing audio");
  });
});

describe("POST /api/tts", () => {
  test("rejects missing text with 400", async () => {
    const { POST } = await import("../app/api/tts/route");
    const req = new Request("http://localhost/api/tts", {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({}),
    });
    const res = await POST(req);
    expect(res.status).toBe(400);
    const body = (await res.json()) as { error: string };
    expect(body.error).toContain("text required");
  });
});
