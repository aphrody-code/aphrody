// SPDX-License-Identifier: Apache-2.0
//
// Runtime entry point for `@aphrody/gemini-live-aphrody`.
//
// Selects between two backends based on `APHRODY_LIVE_BACKEND`:
//
//   gemini-oauth     -> use the Gemini CLI's OAuth tokens (see auth.ts).
//   whisper-gateway  -> Whisper STT (see whisper.ts) + OpenAI-compatible
//                       chat completions (see gateway.ts).
//
// In both cases ElevenLabs (voice.ts) handles TTS. NO GEMINI_API_KEY is
// ever read by this module.
//
// CLI invocation:
//
//   bun run packages/gemini-live-aphrody/src/index.ts <subcommand> [...]
//
// Subcommands:
//
//   doctor                        Print resolved backend + env diagnostics.
//   ask    <prompt>               Single-turn chat against the active backend.
//   stt    <audio-path>           Transcribe a file via Whisper (any backend).
//   speak  <text> [output.mp3]    TTS via ElevenLabs (any backend).

import { authorizationHeader, resolveCredentialsPath } from "./auth.ts";
import { chatCompletion, type ChatMessage } from "./gateway.ts";
import { synthesize, synthesizeToFile } from "./voice.ts";
import { transcribe } from "./whisper.ts";

// ── Public surface ───────────────────────────────────────────────────────────

/** Supported backend identifiers. */
export type BackendId = "gemini-oauth" | "whisper-gateway";

/** Resolved runtime configuration. */
export interface RuntimeConfig {
  backend: BackendId;
  /** Path to the credentials file when backend is `gemini-oauth`. */
  credentialsPath?: string;
  /** Gateway URL when backend is `whisper-gateway`. */
  gatewayUrl?: string;
}

/**
 * Resolve the active backend from the environment. Defaults to
 * `gemini-oauth` because that path reuses the credentials a developer
 * already has from running `gemini auth login`.
 */
export function resolveRuntime(): RuntimeConfig {
  const raw = (process.env["APHRODY_LIVE_BACKEND"] ?? "gemini-oauth").toLowerCase();
  if (raw !== "gemini-oauth" && raw !== "whisper-gateway") {
    throw new Error(
      `Unknown APHRODY_LIVE_BACKEND '${raw}'. Expected 'gemini-oauth' or 'whisper-gateway'.`,
    );
  }
  if (raw === "gemini-oauth") {
    return { backend: raw, credentialsPath: resolveCredentialsPath() };
  }
  return {
    backend: raw,
    gatewayUrl: process.env["APHRODY_GATEWAY_URL"],
  };
}

// ── Backend dispatch ─────────────────────────────────────────────────────────

/**
 * Run a single-turn chat against the active backend and return the
 * assistant's reply as a string.
 */
export async function ask(prompt: string): Promise<string> {
  const runtime = resolveRuntime();
  const messages: ChatMessage[] = [{ role: "user", content: prompt }];

  if (runtime.backend === "gemini-oauth") {
    return askGemini(messages);
  }
  return askGateway(messages);
}

/**
 * Backend A: POST to Google's Cloud Code Assist endpoint with the OAuth
 * Bearer token loaded from the Gemini CLI's credentials file.
 *
 * Endpoint mirrors what the upstream live-api-web-console hits via
 * `@google/genai`, except authenticated with the user's OAuth token
 * instead of an API key. We use the v1beta `generateContent` REST
 * endpoint because the WebSocket Live API is browser-oriented and not
 * needed for a headless one-shot.
 */
async function askGemini(messages: ReadonlyArray<ChatMessage>): Promise<string> {
  const model = process.env["GEMINI_MODEL"] ?? "gemini-2.0-flash-exp";
  const auth = await authorizationHeader();

  const contents = messages.map((m) => ({
    role: m.role === "assistant" ? "model" : "user",
    parts: [{ text: m.content }],
  }));

  const url = `https://generativelanguage.googleapis.com/v1beta/models/${encodeURIComponent(model)}:generateContent`;
  const response = await fetch(url, {
    method: "POST",
    headers: {
      "content-type": "application/json",
      authorization: auth,
    },
    body: JSON.stringify({ contents }),
  });

  if (!response.ok) {
    const detail = await response.text();
    throw new Error(
      `Gemini generateContent failed (${response.status}): ${detail}`,
    );
  }

  const payload = (await response.json()) as Record<string, unknown>;
  const candidates = payload["candidates"];
  if (!Array.isArray(candidates) || candidates.length === 0) {
    return "";
  }
  const first = candidates[0] as Record<string, unknown>;
  const content = first["content"] as Record<string, unknown> | undefined;
  const parts = (content?.["parts"] as Array<Record<string, unknown>> | undefined) ?? [];
  return parts
    .map((p) => (typeof p["text"] === "string" ? p["text"] : ""))
    .join("");
}

/** Backend B: defer to the OpenAI-compatible gateway adapter. */
async function askGateway(messages: ReadonlyArray<ChatMessage>): Promise<string> {
  const result = await chatCompletion({ messages });
  return result.text;
}

// ── CLI ──────────────────────────────────────────────────────────────────────

/**
 * Backend description used by the `doctor` subcommand.
 */
function backendTable(): string {
  return [
    "backend         auth-source                        model                              stt           voice",
    "gemini-oauth    ~/.gemini/oauth_creds.json (CLI)    GEMINI_MODEL (default flash-exp)   n/a           ElevenLabs",
    "whisper-gateway APHRODY_GATEWAY_TOKEN (Bearer)      APHRODY_GATEWAY_MODEL              Whisper (CLI  ElevenLabs",
    "                                                                                       or OpenAI)",
  ].join("\n");
}

async function cliMain(argv: ReadonlyArray<string>): Promise<number> {
  const [subcommand, ...rest] = argv;

  switch (subcommand) {
    case undefined:
    case "help":
    case "-h":
    case "--help": {
      process.stdout.write(usage());
      return 0;
    }

    case "doctor": {
      const runtime = resolveRuntime();
      process.stdout.write(`backend: ${runtime.backend}\n`);
      if (runtime.credentialsPath) {
        process.stdout.write(`credentials: ${runtime.credentialsPath}\n`);
      }
      if (runtime.gatewayUrl) {
        process.stdout.write(`gateway: ${runtime.gatewayUrl}\n`);
      }
      process.stdout.write(`\n${backendTable()}\n`);
      return 0;
    }

    case "ask": {
      const prompt = rest.join(" ").trim();
      if (!prompt) {
        process.stderr.write("ask: prompt required\n");
        return 2;
      }
      const reply = await ask(prompt);
      process.stdout.write(`${reply}\n`);
      return 0;
    }

    case "stt": {
      const audioPath = rest[0];
      if (!audioPath) {
        process.stderr.write("stt: audio path required\n");
        return 2;
      }
      const result = await transcribe({ audioPath });
      process.stdout.write(`${result.text}\n`);
      return 0;
    }

    case "speak": {
      const text = rest[0];
      const output = rest[1];
      if (!text) {
        process.stderr.write("speak: text required\n");
        return 2;
      }
      if (output) {
        const result = await synthesizeToFile({ text }, output);
        process.stdout.write(`wrote ${result.audio.byteLength} bytes (${result.mimeType}) to ${output}\n`);
      } else {
        const result = await synthesize({ text });
        process.stdout.write(`synthesized ${result.audio.byteLength} bytes (${result.mimeType})\n`);
      }
      return 0;
    }

    default: {
      process.stderr.write(`unknown subcommand: ${subcommand}\n\n${usage()}`);
      return 2;
    }
  }
}

function usage(): string {
  return [
    "Usage: bun run packages/gemini-live-aphrody/src/index.ts <subcommand> [args]",
    "",
    "Subcommands:",
    "  doctor                        Print resolved backend + env diagnostics.",
    "  ask    <prompt>               Single-turn chat against the active backend.",
    "  stt    <audio-path>           Transcribe a file via Whisper.",
    "  speak  <text> [output.mp3]    TTS via ElevenLabs.",
    "",
    "Select backend with APHRODY_LIVE_BACKEND=gemini-oauth | whisper-gateway.",
    "",
  ].join("\n");
}

// Re-exports so consumers can import the adapters directly.
export { authorizationHeader, loadCredentials, getAccessToken, resolveCredentialsPath } from "./auth.ts";
export { chatCompletion } from "./gateway.ts";
export type { ChatMessage, ChatRequest, ChatResponse, GatewayConfig } from "./gateway.ts";
export { transcribe } from "./whisper.ts";
export type { TranscriptionResult, WhisperOptions } from "./whisper.ts";
export { synthesize, synthesizeToFile } from "./voice.ts";
export type { VoiceOptions, VoiceResult } from "./voice.ts";

// Entry point — only run the CLI when this module is the program entry,
// not when imported as a library.
const isDirectInvocation =
  typeof Bun !== "undefined" && import.meta.path === Bun.main;
if (isDirectInvocation) {
  cliMain(process.argv.slice(2))
    .then((code) => process.exit(code))
    .catch((err: unknown) => {
      const message = err instanceof Error ? err.message : String(err);
      process.stderr.write(`fatal: ${message}\n`);
      process.exit(1);
    });
}
