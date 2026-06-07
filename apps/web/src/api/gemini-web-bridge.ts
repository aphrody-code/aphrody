// SPDX-License-Identifier: Apache-2.0
//
// Native Gemini (web) bridge for the aphrody chat API.
//
// Thin transport layer over bxc's shared `GeminiSessionPool` (the single source
// of truth for Gemini Web client lifecycle, per-conversation continuity and
// "session expired" detection — see `@aphrody/bxc/google/gemini-session`). This
// file only owns the aphrody-web concerns: picking the prompt for a turn and
// re-streaming the real Gemini answer as OpenAI-compatible SSE so the existing
// Material 3 ChatView renders it natively (typing indicator + message bubbles),
// exactly like the mock models.
//
// bxc is consumed via the `@aphrody/bxc` package specifier (a node_modules
// symlink kept fresh by `scripts/link-bxc.ts`); a direct sibling-path import is
// used as a last-resort fallback so a missing link degrades to a clear error
// bubble instead of crashing the server.

import type { CompletionRequest } from "./types.ts";

/** Model id selected in the ChatView model picker to route here. */
export const GEMINI_WEB_MODEL_ID = "gemini-web";

/** Structural view of the bits of bxc's gemini-session module we depend on. */
interface GeminiSessionPool {
  hasStarted(key: string): boolean;
  generate(key: string, prompt: string, opts?: { model?: string }): Promise<string>;
  reset(key: string): void;
  drop(key: string): void;
}

interface GeminiSessionModule {
  GeminiSessionPool: new (opts?: { model?: string }) => GeminiSessionPool;
  isGeminiStaleError: (message: string) => boolean;
  GEMINI_STALE_MESSAGE: string;
}

// Lazily resolve bxc's shared session module once. Primary: the installed
// `@aphrody/bxc` specifier. Fallback: the sibling checkout path (same convention
// the standalone aphrody-gemini proxy uses).
let modulePromise: Promise<GeminiSessionModule> | null = null;
function loadSessionModule(): Promise<GeminiSessionModule> {
  if (modulePromise) return modulePromise;
  modulePromise = (async () => {
    try {
      return (await import("@aphrody/bxc/google/gemini-session")) as GeminiSessionModule;
    } catch {
      return (await import("/home/ubuntu/bxc/src/google/gemini-session.ts")) as GeminiSessionModule;
    }
  })();
  return modulePromise;
}

// One pool process-wide; keyed internally by chat id so each conversation keeps
// its own Gemini thread (continuity).
let pool: GeminiSessionPool | null = null;
async function ensurePool(): Promise<{ pool: GeminiSessionPool; mod: GeminiSessionModule }> {
  const mod = await loadSessionModule();
  if (!pool) pool = new mod.GeminiSessionPool({});
  return { pool, mod };
}

function lastUserMessage(body: CompletionRequest): string {
  for (let i = body.messages.length - 1; i >= 0; i--) {
    const m = body.messages[i];
    if (m && m.role === "user") return m.content;
  }
  return "";
}

/**
 * Prompt for the first turn of a thread. With a single message we send it as-is;
 * with prior history (e.g. a chat switched to Gemini mid-conversation) we replay
 * the transcript once so Gemini has context, then keepContext carries the rest.
 */
function firstTurnPrompt(body: CompletionRequest): string {
  const msgs = body.messages;
  if (msgs.length <= 1) return msgs[msgs.length - 1]?.content ?? "";
  const prior = msgs
    .slice(0, -1)
    .map((m) => `${m.role === "user" ? "User" : "Assistant"}: ${m.content}`)
    .join("\n");
  const last = msgs[msgs.length - 1];
  return `Conversation so far:\n${prior}\n\nUser: ${last?.content ?? ""}`;
}

const SSE_HEADERS = {
  "content-type": "text/event-stream",
  "cache-control": "no-cache",
  connection: "keep-alive",
} as const;

/**
 * Answer a "gemini-web" completion request: run a real Gemini turn via the
 * shared bxc pool, then re-stream the answer token-by-token as OpenAI-compatible
 * SSE.
 */
export function geminiWebStream(body: CompletionRequest): Response {
  const key = body.chat_id ?? "";

  const stream = new ReadableStream<Uint8Array>({
    async start(controller) {
      const enc = new TextEncoder();
      const send = (obj: unknown) =>
        controller.enqueue(enc.encode(`data: ${JSON.stringify(obj)}\n\n`));

      try {
        const { pool: sessions, mod } = await ensurePool();
        try {
          const prompt = sessions.hasStarted(key) ? lastUserMessage(body) : firstTurnPrompt(body);

          if (!prompt.trim()) {
            send({ choices: [{ delta: { content: "(message vide)" } }] });
            send({ choices: [{ delta: {}, finish_reason: "stop" }] });
            return;
          }

          // The pool keeps continuity (keepContext) and drops the thread on error.
          const text = await sessions.generate(key, prompt);

          const tokens = text.match(/\s+|\S+/g) ?? [text];
          for (const tok of tokens) {
            send({ choices: [{ delta: { content: tok } }] });
            await Bun.sleep(6);
          }
          send({
            choices: [{ delta: {}, finish_reason: "stop" }],
            usage: {
              prompt_tokens: prompt.length,
              completion_tokens: tokens.length,
              total_tokens: prompt.length + tokens.length,
            },
          });
        } catch (err) {
          const msg = err instanceof Error ? err.message : String(err);
          const friendly = mod.isGeminiStaleError(msg)
            ? mod.GEMINI_STALE_MESSAGE
            : `Erreur Gemini : ${msg}`;
          console.error("[gemini-web-bridge]", msg);
          send({ choices: [{ delta: { content: friendly } }] });
          send({ choices: [{ delta: {}, finish_reason: "stop" }] });
        }
      } catch (loadErr) {
        const msg = loadErr instanceof Error ? loadErr.message : String(loadErr);
        console.error("[gemini-web-bridge] bxc module unavailable:", msg);
        send({
          choices: [{ delta: { content: `Module Gemini (bxc) indisponible : ${msg}` } }],
        });
        send({ choices: [{ delta: {}, finish_reason: "stop" }] });
      } finally {
        controller.enqueue(enc.encode("data: [DONE]\n\n"));
        controller.close();
      }
    },
  });

  return new Response(stream, { headers: SSE_HEADERS });
}

/** Forget the Gemini thread for a chat id (used when a chat is reset/deleted). */
export function resetGeminiThread(chatId: string): void {
  pool?.reset(chatId ?? "");
}
