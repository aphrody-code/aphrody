// SPDX-License-Identifier: Apache-2.0
//
// ElevenLabs TTS adapter.
//
// Mirrors the Rust adapter at `crates/aphrody-voice/src/elevenlabs.rs`:
// same env var (`ELEVENLABS_API_KEY`), same header (`xi-api-key`), same
// `POST /v1/text-to-speech/{voice_id}` endpoint, default `audio/mpeg`
// response. Used by both the gemini-oauth and whisper-gateway backends
// when voice output is required.

import { promises as fs } from "node:fs";

// ── Constants ────────────────────────────────────────────────────────────────

const ELEVENLABS_BASE_URL = "https://api.elevenlabs.io";
const DEFAULT_MODEL_ID = "eleven_multilingual_v2";
const DEFAULT_OUTPUT_MIME = "audio/mpeg";

// ── Public types ─────────────────────────────────────────────────────────────

/** Configuration for [`synthesize`]. */
export interface VoiceOptions {
  /** Text to speak. Required. */
  text: string;
  /**
   * Voice id. Defaults to `ELEVENLABS_VOICE_ID`. If neither is set the
   * call fails — there is no implicit default voice in order to keep
   * billing predictable.
   */
  voiceId?: string;
  /** Model id (defaults to `ELEVENLABS_MODEL_ID` or `eleven_multilingual_v2`). */
  modelId?: string;
  /** ElevenLabs API key (defaults to `ELEVENLABS_API_KEY`). */
  apiKey?: string;
  /** Optional voice tuning overrides. */
  voiceSettings?: {
    stability?: number;
    similarity_boost?: number;
    style?: number;
    use_speaker_boost?: boolean;
  };
}

/** Result of a synthesis call. */
export interface VoiceResult {
  /** Raw audio bytes (typically MP3). */
  audio: Uint8Array;
  /** MIME type reported by ElevenLabs (typically `audio/mpeg`). */
  mimeType: string;
  /** Voice id actually used. */
  voiceId: string;
}

// ── Public API ───────────────────────────────────────────────────────────────

/**
 * Synthesize `text` to audio bytes via ElevenLabs. Returns the audio
 * buffer + mime type so the caller can stream/play/persist as it sees
 * fit.
 */
export async function synthesize(opts: VoiceOptions): Promise<VoiceResult> {
  const apiKey = opts.apiKey ?? process.env["ELEVENLABS_API_KEY"];
  if (!apiKey) {
    throw new Error(
      "ELEVENLABS_API_KEY is unset. Provide it via env or VoiceOptions.apiKey.",
    );
  }
  const voiceId = opts.voiceId ?? process.env["ELEVENLABS_VOICE_ID"];
  if (!voiceId) {
    throw new Error(
      "ELEVENLABS_VOICE_ID is unset. Pick a voice id from /v1/voices and export it.",
    );
  }
  const modelId =
    opts.modelId ?? process.env["ELEVENLABS_MODEL_ID"] ?? DEFAULT_MODEL_ID;

  const body = {
    text: opts.text,
    model_id: modelId,
    ...(opts.voiceSettings && { voice_settings: opts.voiceSettings }),
  };

  const url = `${ELEVENLABS_BASE_URL}/v1/text-to-speech/${encodeURIComponent(voiceId)}`;
  const response = await fetch(url, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      accept: DEFAULT_OUTPUT_MIME,
      "xi-api-key": apiKey,
    },
    body: JSON.stringify(body),
  });

  if (!response.ok) {
    const detail = await response.text();
    throw new Error(
      `ElevenLabs synthesis failed (${response.status}): ${detail}`,
    );
  }

  const mimeType = response.headers.get("content-type") ?? DEFAULT_OUTPUT_MIME;
  const buffer = new Uint8Array(await response.arrayBuffer());
  return { audio: buffer, mimeType, voiceId };
}

/**
 * Convenience helper: synthesize and write the result to disk in one
 * call.
 */
export async function synthesizeToFile(
  opts: VoiceOptions,
  outputPath: string,
): Promise<VoiceResult> {
  const result = await synthesize(opts);
  await fs.writeFile(outputPath, result.audio);
  return result;
}
