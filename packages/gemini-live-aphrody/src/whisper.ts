// SPDX-License-Identifier: Apache-2.0
//
// Whisper STT adapter for the whisper-gateway backend.
//
// Two transports are supported, selected at runtime:
//
//   1. Local whisper CLI (default when `OPENAI_API_KEY` is unset).
//      Shells out to a `whisper` binary (OpenAI's reference Python CLI,
//      whisper.cpp's `main`, or any drop-in) and parses the JSON output.
//      Override the binary path with `WHISPER_BINARY`.
//
//   2. OpenAI Whisper HTTP API (when `OPENAI_API_KEY` is set).
//      POSTs multipart/form-data to /v1/audio/transcriptions per the
//      OpenAI API reference.
//
// Either path returns the same `TranscriptionResult` shape so the
// runtime selector in `index.ts` does not care which transport ran.

import { spawn } from "node:child_process";
import { promises as fs } from "node:fs";
import { basename } from "node:path";

// ── Public types ─────────────────────────────────────────────────────────────

/** Normalized transcription returned by either transport. */
export interface TranscriptionResult {
  text: string;
  language?: string;
  segments?: ReadonlyArray<{
    start: number;
    end: number;
    text: string;
  }>;
}

/** Configuration accepted by [`transcribe`]. */
export interface WhisperOptions {
  /** Absolute path to the audio file. WAV/MP3/FLAC/M4A all work. */
  audioPath: string;
  /** Optional language hint (ISO 639-1, e.g. "fr"). */
  language?: string;
  /** Optional model id. CLI: "small", "medium", "large-v3". API: "whisper-1". */
  model?: string;
}

// ── Public API ───────────────────────────────────────────────────────────────

/**
 * Transcribe an audio file. Dispatches to the OpenAI HTTP API when
 * `OPENAI_API_KEY` is present, otherwise to the local Whisper CLI.
 */
export async function transcribe(
  opts: WhisperOptions,
): Promise<TranscriptionResult> {
  if (process.env["OPENAI_API_KEY"]) {
    return transcribeViaOpenAi(opts);
  }
  return transcribeViaLocalCli(opts);
}

// ── Local CLI transport ──────────────────────────────────────────────────────

/**
 * Shell out to a local `whisper` binary, request `--output_format json`,
 * read the resulting `<basename>.json` file from the working directory,
 * then normalize it into [`TranscriptionResult`].
 *
 * The CLI's output naming convention (`<basename>.json` next to the
 * audio) matches both OpenAI's reference whisper Python CLI and
 * whisper.cpp's `main -oj` flag.
 */
async function transcribeViaLocalCli(
  opts: WhisperOptions,
): Promise<TranscriptionResult> {
  const binary = process.env["WHISPER_BINARY"] ?? "whisper";
  const model = opts.model ?? process.env["WHISPER_MODEL"] ?? "small";
  const args = [opts.audioPath, "--output_format", "json", "--model", model];
  if (opts.language) {
    args.push("--language", opts.language);
  }

  await runProcess(binary, args);

  // OpenAI whisper writes <basename>.json next to the input file by
  // default; whisper.cpp obeys `-of <prefix>` and writes <prefix>.json.
  // We try the basename-next-to-audio convention first.
  const candidate = opts.audioPath.replace(/\.[^.]+$/u, ".json");
  let raw: string;
  try {
    raw = await fs.readFile(candidate, "utf8");
  } catch (cause) {
    throw new Error(
      `Whisper CLI did not produce ${candidate}. Check that ${binary} supports --output_format json.`,
      { cause },
    );
  }
  return parseWhisperJson(raw);
}

/**
 * Spawn a process and resolve once it exits 0. Rejects with stderr on
 * non-zero exit.
 */
function runProcess(command: string, args: ReadonlyArray<string>): Promise<void> {
  return new Promise((resolve, reject) => {
    const child = spawn(command, [...args], { stdio: ["ignore", "pipe", "pipe"] });
    const stderr: Buffer[] = [];
    child.stderr.on("data", (chunk: Buffer) => stderr.push(chunk));
    child.on("error", reject);
    child.on("close", (code) => {
      if (code === 0) {
        resolve();
        return;
      }
      const detail = Buffer.concat(stderr).toString("utf8").trim();
      reject(
        new Error(
          `${command} exited with code ${code}${detail ? `: ${detail}` : ""}`,
        ),
      );
    });
  });
}

/**
 * Parse the JSON shape produced by both OpenAI's reference whisper CLI
 * and whisper.cpp `-oj`. Both expose `text` at the top level; segments
 * are an array of `{ start, end, text }` (whisper.cpp uses
 * `offsets.from / offsets.to` instead — we normalize both shapes).
 */
function parseWhisperJson(raw: string): TranscriptionResult {
  const parsed = JSON.parse(raw) as Record<string, unknown>;
  const text = typeof parsed["text"] === "string" ? parsed["text"] : "";
  const language =
    typeof parsed["language"] === "string" ? parsed["language"] : undefined;

  const segmentsRaw = parsed["segments"];
  const segments = Array.isArray(segmentsRaw)
    ? segmentsRaw.map((seg) => normalizeSegment(seg as Record<string, unknown>))
    : undefined;

  return { text, language, segments };
}

function normalizeSegment(seg: Record<string, unknown>): {
  start: number;
  end: number;
  text: string;
} {
  const start = typeof seg["start"] === "number" ? seg["start"] : 0;
  const end = typeof seg["end"] === "number" ? seg["end"] : 0;
  const text = typeof seg["text"] === "string" ? seg["text"] : "";

  const offsets = seg["offsets"] as Record<string, unknown> | undefined;
  if (offsets) {
    return {
      start:
        typeof offsets["from"] === "number" ? offsets["from"] / 1000 : start,
      end: typeof offsets["to"] === "number" ? offsets["to"] / 1000 : end,
      text,
    };
  }
  return { start, end, text };
}

// ── OpenAI HTTP transport ────────────────────────────────────────────────────

/**
 * POST the audio file to OpenAI's `/v1/audio/transcriptions` endpoint
 * with `response_format=verbose_json` so segments/language are populated.
 */
async function transcribeViaOpenAi(
  opts: WhisperOptions,
): Promise<TranscriptionResult> {
  const apiKey = process.env["OPENAI_API_KEY"];
  if (!apiKey) {
    throw new Error("OPENAI_API_KEY is unset; cannot use OpenAI transport.");
  }

  const audio = await fs.readFile(opts.audioPath);
  const form = new FormData();
  form.append(
    "file",
    new Blob([audio as Uint8Array]),
    basename(opts.audioPath),
  );
  form.append("model", opts.model ?? "whisper-1");
  form.append("response_format", "verbose_json");
  if (opts.language) {
    form.append("language", opts.language);
  }

  const response = await fetch(
    "https://api.openai.com/v1/audio/transcriptions",
    {
      method: "POST",
      headers: { authorization: `Bearer ${apiKey}` },
      body: form,
    },
  );

  if (!response.ok) {
    const detail = await response.text();
    throw new Error(
      `OpenAI Whisper API failed (${response.status}): ${detail}`,
    );
  }

  const payload = (await response.json()) as Record<string, unknown>;
  const text = typeof payload["text"] === "string" ? payload["text"] : "";
  const language =
    typeof payload["language"] === "string" ? payload["language"] : undefined;
  const segmentsRaw = payload["segments"];
  const segments = Array.isArray(segmentsRaw)
    ? segmentsRaw.map((seg) => normalizeSegment(seg as Record<string, unknown>))
    : undefined;
  return { text, language, segments };
}
