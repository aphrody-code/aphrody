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

/** Stream a canned reply as OpenAI-style SSE deltas. */
function completionStream(body: CompletionRequest): Response {
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

    // SPA + bundling catch-all (HTML import auto-bundles main.tsx + CSS).
    "/*": index,
  },
});

console.log(`Open WebUI · M3  →  http://localhost:${server.port}`);
