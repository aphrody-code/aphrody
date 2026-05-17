// SPDX-License-Identifier: Apache-2.0
//
// POST /api/stt — server-only Whisper transcription endpoint.
//
// Accepts multipart/form-data with a single `audio` field (any WAV/MP3/
// WebM/OGG payload). Writes the bytes to a temp file, hands them to
// `transcribe()` from `@aphrody/gemini-live-aphrody`, returns the text.

import { promises as fs } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

import { transcribe } from "@aphrody/gemini-live-aphrody";

export const runtime = "nodejs";
export const dynamic = "force-dynamic";

const EXT_BY_MIME: Record<string, string> = {
  "audio/webm": "webm",
  "audio/ogg": "ogg",
  "audio/wav": "wav",
  "audio/x-wav": "wav",
  "audio/mpeg": "mp3",
  "audio/mp4": "m4a",
  "audio/flac": "flac",
};

export async function POST(req: Request): Promise<Response> {
  let form: FormData;
  try {
    form = await req.formData();
  } catch {
    return Response.json(
      { error: "request must be multipart/form-data" },
      { status: 400 },
    );
  }

  const file = form.get("audio");
  if (!(file instanceof Blob)) {
    return Response.json(
      { error: "missing audio field (must be a file/Blob)" },
      { status: 400 },
    );
  }
  if (file.size === 0) {
    return Response.json({ error: "audio file is empty" }, { status: 400 });
  }

  const ext = EXT_BY_MIME[file.type] ?? "webm";
  const filename = `aphrody-stt-${Date.now()}-${Math.random().toString(36).slice(2, 8)}.${ext}`;
  const path = join(tmpdir(), filename);

  let cleanup = true;
  try {
    const buffer = new Uint8Array(await file.arrayBuffer());
    await fs.writeFile(path, buffer);
    const language = form.get("language");
    const result = await transcribe({
      audioPath: path,
      ...(typeof language === "string" && language.length > 0
        ? { language }
        : {}),
    });
    return Response.json({
      text: result.text,
      language: result.language,
    });
  } catch (err) {
    const message = err instanceof Error ? err.message : String(err);
    return Response.json({ error: message }, { status: 503 });
  } finally {
    if (cleanup) {
      void fs.unlink(path).catch(() => {});
    }
  }
}
