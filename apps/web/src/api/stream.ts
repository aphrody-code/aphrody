// SSE chat-completion parser. Mirrors open-webui's createOpenAITextStream:
// reads the fetch ReadableStream, splits on blank-line SSE frames, yields deltas.

import type { CompletionRequest, StreamUpdate } from "./types.ts";
import { auth } from "./client.ts";

interface OpenAIChunk {
  choices?: { delta?: { content?: string }; finish_reason?: string | null }[];
  usage?: StreamUpdate["usage"];
}

export async function* streamCompletion(
  body: CompletionRequest,
  signal?: AbortSignal,
): AsyncGenerator<StreamUpdate> {
  const res = await fetch("/api/chat/completions", {
    method: "POST",
    headers: {
      "content-type": "application/json",
      ...(auth.token ? { authorization: `Bearer ${auth.token}` } : {}),
    },
    body: JSON.stringify(body),
    signal,
  });

  if (!res.ok || !res.body) {
    yield { done: true, value: "", error: `HTTP ${res.status}` };
    return;
  }

  const reader = res.body.getReader();
  const decoder = new TextDecoder();
  let buffer = "";

  while (true) {
    const { done, value } = await reader.read();
    if (done) break;
    buffer += decoder.decode(value, { stream: true });

    const frames = buffer.split("\n\n");
    buffer = frames.pop() ?? "";

    for (const frame of frames) {
      const line = frame.trim();
      if (!line.startsWith("data:")) continue;
      const payload = line.slice(5).trim();
      if (payload === "[DONE]") {
        yield { done: true, value: "" };
        return;
      }
      try {
        const chunk = JSON.parse(payload) as OpenAIChunk;
        const delta = chunk.choices?.[0]?.delta?.content ?? "";
        if (delta) yield { done: false, value: delta };
        if (chunk.usage) yield { done: false, value: "", usage: chunk.usage };
      } catch {
        // partial / non-JSON keep-alive frame — ignore
      }
    }
  }
  yield { done: true, value: "" };
}
