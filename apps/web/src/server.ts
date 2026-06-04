// Bun-native server: bundles + serves the React app via an HTML import (HMR in dev),
// and exposes an OpenAI-compatible mock API — including an SSE chat-completions stream.
// No Next, no Vite, no Express. Just Bun.serve.

import index from "./index.html";
import {
  ADMIN_USERS,
  CHATS,
  CONFIG,
  fakeReply,
  KNOWLEDGE,
  MODELS,
  PROMPTS,
  SESSION_USER,
  TOOLS,
  WORKSPACE_MODELS,
} from "./api/mock-data.ts";
import type { Chat, ChatListItem, CompletionRequest } from "./api/types.ts";
import { ACCOUNT, META, runMock } from "./aphrody/server-mock.ts";
import {
  getSemanticCache,
  setSemanticCache,
  searchShenron,
  searchRpbey,
  performServerlessScrape,
  getSystemInstruction,
  streamGeminiChat,
} from "./api/rag-engine.ts";

const PORT = Number(process.env.PORT ?? 3210);

// Mutable in-memory chat store, seeded from mock data.
const chats = new Map<string, Chat>(CHATS.map((c) => [c.id, c]));

const json = (data: unknown, status = 200) =>
  new Response(JSON.stringify(data), {
    status,
    headers: { "content-type": "application/json" },
  });

function listItems(): ChatListItem[] {
  return [...chats.values()]
    .filter((c) => !c.archived)
    .toSorted((a, b) => b.updated_at - a.updated_at)
    .map((c) => ({ id: c.id, title: c.title, pinned: c.pinned, updated_at: c.updated_at }));
}

/** Helper to format standard OpenAI-compatible mock reply stream. */
function fallbackMockStream(body: CompletionRequest): Response {
  const lastUser = body.messages.toReversed().find((m) => m.role === "user");
  const text = fakeReply(lastUser?.content ?? "", body.model);
  const tokens = text.match(/\s+|\S+/g) ?? [text];

  const stream = new ReadableStream<Uint8Array>({
    async start(controller) {
      const enc = new TextEncoder();
      const send = (obj: unknown) =>
        controller.enqueue(enc.encode(`data: ${JSON.stringify(obj)}\n\n`));
      for (const tok of tokens) {
        send({ choices: [{ delta: { content: tok } }] });
        await Bun.sleep(14);
      }
      send({
        choices: [{ delta: {}, finish_reason: "stop" }],
        usage: {
          prompt_tokens: 32,
          completion_tokens: tokens.length,
          total_tokens: 32 + tokens.length,
        },
      });
      controller.enqueue(enc.encode("data: [DONE]\n\n"));
      controller.close();
    },
  });

  return new Response(stream, {
    headers: {
      "content-type": "text/event-stream",
      "cache-control": "no-cache",
      connection: "keep-alive",
    },
  });
}

/** Stream a real RAG-enhanced Gemini or local model reply. */
function completionStream(body: CompletionRequest): Response {
  const lastUser = body.messages.toReversed().find((m) => m.role === "user");
  const query = lastUser?.content ?? "";

  // If no Gemini API key, or empty query, default immediately to the mock fallback.
  if (!query || (!process.env.GEMINI_API_KEY && !process.env.GOOGLE_API_KEY)) {
    return fallbackMockStream(body);
  }

  const stream = new ReadableStream<Uint8Array>({
    async start(controller) {
      const enc = new TextEncoder();
      const send = (obj: unknown) =>
        controller.enqueue(enc.encode(`data: ${JSON.stringify(obj)}\n\n`));

      try {
        // 1. Check Redis semantic cache
        const cachedAnswer = await getSemanticCache(query, body.model);
        if (cachedAnswer) {
          const tokens = cachedAnswer.match(/\s+|\S+/g) ?? [cachedAnswer];
          for (const tok of tokens) {
            send({ choices: [{ delta: { content: tok } }] });
            await Bun.sleep(10);
          }
          send({ choices: [{ delta: {}, finish_reason: "stop" }] });
          controller.enqueue(enc.encode("data: [DONE]\n\n"));
          controller.close();
          return;
        }

        let contextMd = "";

        // 2. Perform Serverless Scraping via bxc if URLs are in the query
        const urlRegex = /https?:\/\/[^\s]+/gi;
        const urls = query.match(urlRegex);
        if (urls && urls.length > 0) {
          for (const url of urls) {
            const scrapedText = await performServerlessScrape(url);
            contextMd += `### Webpage Content: ${url}\n${scrapedText}\n\n`;
          }
        }

        // 3. Perform Vector/Hybrid RAG search based on model
        let sources: { title: string; url: string }[] = [];
        if (body.model === "shenron") {
          const hits = await searchShenron(query, 5);
          for (const hit of hits) {
            contextMd += `### Document: ${hit.title} (Type: ${hit.kind})\n${hit.content ?? hit.snippet}\n\n`;
            sources.push({ title: hit.title, url: hit.url });
          }
        } else if (body.model === "rpbey") {
          const hits = await searchRpbey(query, 5);
          for (const hit of hits) {
            contextMd += `### Document: ${hit.title} (Type: ${hit.kind})\n${hit.content ?? hit.snippet}\n\n`;
            sources.push({ title: hit.title, url: hit.url });
          }
        }

        // 4. Generate system instruction
        const systemInstruction = getSystemInstruction(body.model, contextMd);

        // 5. Query and stream Gemini
        let fullAnswer = "";
        await streamGeminiChat(systemInstruction, body.messages, (chunk) => {
          fullAnswer += chunk;
          send({ choices: [{ delta: { content: chunk } }] });
        });

        // Add source citations to output if sources exist
        if (sources.length > 0) {
          const citationHeader = "\n\n**Sources :**\n" + sources.map((s, idx) => `* [${s.title}](${s.url})`).join("\n");
          send({ choices: [{ delta: { content: citationHeader } }] });
          fullAnswer += citationHeader;
        }

        // 6. Cache the generated answer
        await setSemanticCache(query, fullAnswer, body.model);

        send({ choices: [{ delta: {}, finish_reason: "stop" }] });
      } catch (err) {
        console.error("[RAG ENGINE] Error during stream processing, falling back to mock:", err);
        // On error, write the error details as an assistant bubble
        const errMsg = `\n\n_[RAG System Error: ${(err as Error).message}. Falling back to mock.]_\n\n`;
        send({ choices: [{ delta: { content: errMsg } }] });
        
        // Fallback streaming
        const mockText = fakeReply(query, body.model);
        const tokens = mockText.match(/\s+|\S+/g) ?? [mockText];
        for (const tok of tokens) {
          send({ choices: [{ delta: { content: tok } }] });
          await Bun.sleep(10);
        }
        send({ choices: [{ delta: {}, finish_reason: "stop" }] });
      } finally {
        controller.enqueue(enc.encode("data: [DONE]\n\n"));
        controller.close();
      }
    },
  });

  return new Response(stream, {
    headers: {
      "content-type": "text/event-stream",
      "cache-control": "no-cache",
      connection: "keep-alive",
    },
  });
}


const server = Bun.serve({
  port: PORT,
  development: process.env.NODE_ENV !== "production",
  routes: {
    "/api/config": () => json(CONFIG),

    "/api/auths/signin": {
      POST: async (req) => {
        const { email } = (await req.json().catch(() => ({}))) as { email?: string };
        return json({ ...SESSION_USER, email: email || SESSION_USER.email });
      },
    },
    "/api/auths/signup": {
      POST: async (req) => {
        const body = (await req.json().catch(() => ({}))) as { name?: string; email?: string };
        return json({
          ...SESSION_USER,
          name: body.name || SESSION_USER.name,
          email: body.email || SESSION_USER.email,
        });
      },
    },
    "/api/auths": () => json(SESSION_USER),

    "/api/models": () => json(MODELS),

    "/api/chats": () => json(listItems()),
    "/api/chats/new": {
      POST: async (req) => {
        const incoming = (await req.json().catch(() => ({}))) as Partial<Chat>;
        const id = incoming.id ?? `c-${Bun.randomUUIDv7()}`;
        const chat: Chat = {
          id,
          title: incoming.title ?? "New Chat",
          models: incoming.models ?? CONFIG.default_models,
          history: incoming.history ?? { messages: {}, currentId: null },
          created_at: Date.now(),
          updated_at: Date.now(),
        };
        chats.set(id, chat);
        return json(chat);
      },
    },
    "/api/chats/:id": {
      GET: (req) => {
        const chat = chats.get(req.params.id);
        return chat ? json(chat) : json({ detail: "Not found" }, 404);
      },
      POST: async (req) => {
        const patch = (await req.json().catch(() => ({}))) as Partial<Chat>;
        const existing = chats.get(req.params.id);
        if (!existing) return json({ detail: "Not found" }, 404);
        const updated: Chat = { ...existing, ...patch, id: existing.id, updated_at: Date.now() };
        chats.set(existing.id, updated);
        return json(updated);
      },
      DELETE: (req) => {
        chats.delete(req.params.id);
        return json({ ok: true });
      },
    },

    "/api/chat/completions": {
      POST: async (req) => {
        const body = (await req.json()) as CompletionRequest;
        return completionStream(body);
      },
    },

    "/api/users": () => json(ADMIN_USERS),
    "/api/workspace/models": () => json(WORKSPACE_MODELS),
    "/api/workspace/knowledge": () => json(KNOWLEDGE),
    "/api/workspace/prompts": () => json(PROMPTS),
    "/api/workspace/tools": () => json(TOOLS),

    // aphrody desktop port: CLI bridge + host metadata + linked account.
    "/api/run": {
      POST: async (req) => {
        const { args } = (await req.json().catch(() => ({ args: [] }))) as { args?: string[] };
        if (!Array.isArray(args)) return json({ code: 1, stdout: "", stderr: "Invalid args" }, 400);

        try {
          const binPath = "/home/ubuntu/.local/bin/aphrody";
          const proc = Bun.spawn([binPath, ...args], {
            stdout: "pipe",
            stderr: "pipe",
          });
          const stdout = await new Response(proc.stdout).text();
          const stderr = await new Response(proc.stderr).text();
          const code = await proc.exited;
          return json({ code, stdout, stderr });
        } catch (err) {
          console.error("Failed to run real aphrody binary, falling back to mock:", err);
          return json(runMock(args));
        }
      },
    },
    "/api/meta": () => json(META),
    "/api/account": () => json(ACCOUNT),

    // SPA + bundling catch-all (HTML import auto-bundles main.tsx + CSS).
    "/*": index,
  },
});

console.log(`Open WebUI · M3  →  http://localhost:${server.port}`);
