// SPDX-License-Identifier: Apache-2.0
//
// POST /api/tts — server-only ElevenLabs synthesis endpoint.
//
// Body : { text: string, voiceId?: string }
// Reply: audio/mpeg byte stream (raw MP3) | JSON error
//
// Streams the audio bytes back as `audio/mpeg` so the client `<audio>`
// element can play it directly without an intermediate Blob hop. If the
// ElevenLabs key/voice are missing we reply 503 explicitly.

import { synthesize } from "../../../core/index.ts";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

interface TtsRequestBody {
  text?: unknown;
  voiceId?: unknown;
}

export async function POST(req: Request): Promise<Response> {
  let body: TtsRequestBody;
  try {
    body = (await req.json()) as TtsRequestBody;
  } catch {
    return Response.json(
      { error: "request body must be JSON" },
      { status: 400 },
    );
  }
  const text = body.text;
  if (typeof text !== "string" || text.trim().length === 0) {
    return Response.json(
      { error: "text required (non-empty string)" },
      { status: 400 },
    );
  }
  const voiceId = typeof body.voiceId === "string" ? body.voiceId : undefined;

  try {
    const result = await synthesize({
      text,
      ...(voiceId !== undefined ? { voiceId } : {}),
    });
    return new Response(result.audio as BodyInit, {
      status: 200,
      headers: {
        "content-type": result.mimeType,
        "cache-control": "no-store",
        "x-aphrody-voice-id": result.voiceId,
      },
    });
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return Response.json({ error: message }, { status: 503 });
  }
}
