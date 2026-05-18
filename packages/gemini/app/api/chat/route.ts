// SPDX-License-Identifier: Apache-2.0
//
// POST /api/chat — server-only chat completion endpoint.
//
// Body : { prompt: string }
// Reply: Stream of Server-Sent Events (SSE) or plain text.
//
// Dispatches to `@aphrody/gemini-live-aphrody`'s askStream() which itself selects
// the gemini-oauth or whisper-gateway backend at runtime from
// APHRODY_LIVE_BACKEND. If credentials are missing the SDK throws and we
// reply 503 so the client renders an explicit failure.

import { askStream } from "../../../core/index.ts";

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
    const stream = await askStream(prompt);
    
    const readable = new ReadableStream({
      async start(controller) {
        try {
          for await (const chunk of stream) {
            controller.enqueue(new TextEncoder().encode(chunk));
          }
        } catch (e) {
          controller.error(e);
        } finally {
          controller.close();
        }
      }
    });

    return new Response(readable, {
      headers: {
        "Content-Type": "text/plain; charset=utf-8",
        "Transfer-Encoding": "chunked"
      }
    });
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return Response.json({ error: message }, { status: 503 });
  }
}
