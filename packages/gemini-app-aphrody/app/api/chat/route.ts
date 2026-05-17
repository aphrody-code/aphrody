// SPDX-License-Identifier: Apache-2.0
//
// POST /api/chat — server-only chat completion endpoint.
//
// Body : { prompt: string }
// Reply: { reply: string } | { error: string }
//
// Dispatches to `@aphrody/gemini-live-aphrody`'s ask() which itself selects
// the gemini-oauth or whisper-gateway backend at runtime from
// APHRODY_LIVE_BACKEND. If credentials are missing the SDK throws and we
// reply 503 so the client renders an explicit failure.

import { ask } from "@aphrody/gemini-live-aphrody";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

interface ChatRequestBody {
  prompt?: unknown;
}

export async function POST(req: Request): Promise<Response> {
  let body: ChatRequestBody;
  try {
    body = (await req.json()) as ChatRequestBody;
  } catch {
    return Response.json(
      { error: "request body must be JSON" },
      { status: 400 },
    );
  }
  const prompt = body.prompt;
  if (typeof prompt !== "string" || prompt.trim().length === 0) {
    return Response.json(
      { error: "prompt required (non-empty string)" },
      { status: 400 },
    );
  }
  try {
    const reply = await ask(prompt);
    return Response.json({ reply });
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return Response.json({ error: message }, { status: 503 });
  }
}
