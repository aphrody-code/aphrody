// Bun-native server: bundles + serves the React app via an HTML import (HMR in dev),
// and exposes an OpenAI-compatible mock API — including an SSE chat-completions stream.
// No Next, no Vite, no Express. Just Bun.serve.
import * as admin from "firebase-admin";
import { aphrodyFlow } from "./api/genkit-flow.ts";
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

// Mutable in-memory collections, seeded from mock data.
const chats = new Map<string, Chat>(CHATS.map((c) => [c.id, c]));
const adminUsers = [...ADMIN_USERS];
const workspaceModels = [...WORKSPACE_MODELS];
const knowledge = [...KNOWLEDGE];
const prompts = [...PROMPTS];
const tools = [...TOOLS];

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
          const citationHeader = "\n\n**Sources :**\n" + sources.map((s) => `* [${s.title}](${s.url})`).join("\n");
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


const EXPECTED_TOKEN = process.env.WEB_APP_TOKEN ?? "m7K2p9Q4x1R8w5Z3";

let firebaseAdminInitialized = false;
try {
  const serviceAccountPath = "/home/ubuntu/aphrody/secrets/aphrody-bot.json";
  const file = Bun.file(serviceAccountPath);
  if (await file.exists()) {
    const serviceAccount = await file.json();
    admin.initializeApp({
      credential: admin.credential.cert(serviceAccount)
    });
    firebaseAdminInitialized = true;
    console.log("Firebase Admin initialized successfully.");
  }
} catch (err) {
  console.error("Failed to initialize Firebase Admin:", err);
}

async function isAuthorized(req: Request): Promise<boolean> {
  const authHeader = req.headers.get("authorization");
  let token = "";
  if (authHeader && authHeader.startsWith("Bearer ")) {
    token = authHeader.substring(7);
  } else {
    const url = new URL(req.url);
    token = url.searchParams.get("token") || "";
  }

  if (token === EXPECTED_TOKEN) {
    return true;
  }

  if (token && firebaseAdminInitialized) {
    try {
      const decoded = await admin.auth().verifyIdToken(token);
      return !!decoded;
    } catch {
      // Token verification failed or not a firebase token
    }
  }

  return false;
}

type Handler = (req: any) => Response | Promise<Response>;

function withAuth(handler: Handler): Handler {
  return async (req) => {
    if (!(await isAuthorized(req))) {
      return json({ detail: "Unauthorized" }, 401);
    }
    return handler(req);
  };
}

const server = Bun.serve({
  port: PORT,
  development: process.env.NODE_ENV !== "production",
  routes: {
    "/api/config": withAuth(() => json(CONFIG)),

    "/api/auths/signin": {
      POST: withAuth(async (req) => {
        const { email } = (await req.json().catch(() => ({}))) as { email?: string };
        return json({ ...SESSION_USER, email: email || SESSION_USER.email });
      }),
    },
    "/api/auths/signup": {
      POST: withAuth(async (req) => {
        const body = (await req.json().catch(() => ({}))) as { name?: string; email?: string };
        return json({
          ...SESSION_USER,
          name: body.name || SESSION_USER.name,
          email: body.email || SESSION_USER.email,
        });
      }),
    },
    "/api/auths": withAuth(() => json(SESSION_USER)),

    "/api/models": withAuth(() => json(MODELS)),

    "/api/chats": withAuth(() => json(listItems())),
    "/api/chats/new": {
      POST: withAuth(async (req) => {
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
      }),
    },
    "/api/chats/:id": {
      GET: withAuth((req) => {
        const chat = chats.get(req.params.id);
        return chat ? json(chat) : json({ detail: "Not found" }, 404);
      }),
      POST: withAuth(async (req) => {
        const patch = (await req.json().catch(() => ({}))) as Partial<Chat>;
        const existing = chats.get(req.params.id);
        if (!existing) return json({ detail: "Not found" }, 404);
        const updated: Chat = { ...existing, ...patch, id: existing.id, updated_at: Date.now() };
        chats.set(existing.id, updated);
        return json(updated);
      }),
      DELETE: withAuth((req) => {
        chats.delete(req.params.id);
        return json({ ok: true });
      }),
    },

    "/api/chat/completions": {
      POST: withAuth(async (req) => {
        const body = (await req.json()) as CompletionRequest;
        return completionStream(body);
      }),
    },

    "/api/genkit": {
      POST: withAuth(async (req) => {
        try {
          const body = (await req.json()) as { prompt: string };
          const result = await aphrodyFlow({ prompt: body.prompt });
          return json(result);
        } catch (err: any) {
          console.error("Genkit flow run failed:", err);
          return json({ error: err.message || "Failed to execute Genkit flow" }, 500);
        }
      }),
    },

    "/api/users": withAuth(() => json(adminUsers)),
    "/api/users/new": {
      POST: withAuth(async (req) => {
        const body = (await req.json()) as { name: string; email: string; role: any };
        const user = {
          id: `u-${Bun.randomUUIDv7()}`,
          name: body.name || "Nouveau",
          email: body.email || "new@example.com",
          role: body.role || "user",
          last_active_at: Date.now(),
          created_at: Date.now(),
        };
        adminUsers.push(user);
        return json(user);
      }),
    },
    "/api/users/:id": {
      DELETE: withAuth((req) => {
        const idx = adminUsers.findIndex((u) => u.id === req.params.id);
        if (idx !== -1) {
          adminUsers.splice(idx, 1);
          return json({ ok: true });
        }
        return json({ detail: "Not found" }, 404);
      }),
    },

    "/api/workspace/models": withAuth(() => json(workspaceModels)),
    "/api/workspace/models/new": {
      POST: withAuth(async (req) => {
        const body = (await req.json()) as Partial<WorkspaceModel>;
        const model: WorkspaceModel = {
          id: `wm-${Bun.randomUUIDv7()}`,
          name: body.name || "Nouveau modèle",
          description: body.description || "",
          base_model_id: body.base_model_id || "gpt-4o",
          tags: body.tags || [],
          visibility: body.visibility || "public",
          created_at: Date.now(),
        };
        workspaceModels.push(model);
        return json(model);
      }),
    },
    "/api/workspace/models/:id": {
      DELETE: withAuth((req) => {
        const idx = workspaceModels.findIndex((m) => m.id === req.params.id);
        if (idx !== -1) {
          workspaceModels.splice(idx, 1);
          return json({ ok: true });
        }
        return json({ detail: "Not found" }, 404);
      }),
    },

    "/api/workspace/knowledge": withAuth(() => json(knowledge)),
    "/api/workspace/knowledge/new": {
      POST: withAuth(async (req) => {
        const body = (await req.json()) as Partial<KnowledgeCollection>;
        const collection: KnowledgeCollection = {
          id: `k-${Bun.randomUUIDv7()}`,
          name: body.name || "Nouvelle collection",
          description: body.description || "",
          file_count: body.file_count || 0,
          created_at: Date.now(),
        };
        knowledge.push(collection);
        return json(collection);
      }),
    },
    "/api/workspace/knowledge/:id": {
      DELETE: withAuth((req) => {
        const idx = knowledge.findIndex((k) => k.id === req.params.id);
        if (idx !== -1) {
          knowledge.splice(idx, 1);
          return json({ ok: true });
        }
        return json({ detail: "Not found" }, 404);
      }),
    },

    "/api/workspace/prompts": withAuth(() => json(prompts)),
    "/api/workspace/prompts/new": {
      POST: withAuth(async (req) => {
        const body = (await req.json()) as Partial<Prompt>;
        const prompt: Prompt = {
          id: `p-${Bun.randomUUIDv7()}`,
          command: body.command || "/new",
          title: body.title || "Nouveau prompt",
          content: body.content || "",
          tags: body.tags || [],
          created_at: Date.now(),
        };
        prompts.push(prompt);
        return json(prompt);
      }),
    },
    "/api/workspace/prompts/:id": {
      DELETE: withAuth((req) => {
        const idx = prompts.findIndex((p) => p.id === req.params.id);
        if (idx !== -1) {
          prompts.splice(idx, 1);
          return json({ ok: true });
        }
        return json({ detail: "Not found" }, 404);
      }),
    },

    "/api/workspace/tools": withAuth(() => json(tools)),
    "/api/workspace/tools/new": {
      POST: withAuth(async (req) => {
        const body = (await req.json()) as Partial<Tool>;
        const tool: Tool = {
          id: `t-${Bun.randomUUIDv7()}`,
          name: body.name || "Nouvel outil",
          description: body.description || "",
          type: body.type || "custom",
          created_at: Date.now(),
        };
        tools.push(tool);
        return json(tool);
      }),
    },
    "/api/workspace/tools/:id": {
      DELETE: withAuth((req) => {
        const idx = tools.findIndex((t) => t.id === req.params.id);
        if (idx !== -1) {
          tools.splice(idx, 1);
          return json({ ok: true });
        }
        return json({ detail: "Not found" }, 404);
      }),
    },

    // aphrody desktop port: CLI bridge + host metadata + linked account.
    "/api/run": {
      POST: withAuth(async (req) => {
        const { args } = (await req.json().catch(() => ({ args: [] }))) as { args?: string[] };
        if (!Array.isArray(args)) return json({ code: 1, stdout: "", stderr: "Invalid args" }, 400);

        try {
          const binPath = "/home/ubuntu/.local/bin/aphrody";
          const isPythonCmd = args[0] === "image" || args[0] === "blender";
          const proc = isPythonCmd
            ? Bun.spawn(["uv", "--directory", "py", "run", "aphrody", ...args], {
                stdout: "pipe",
                stderr: "pipe",
              })
            : Bun.spawn([binPath, ...args], {
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
      }),
    },
    "/api/meta": withAuth(() => json(META)),
    "/api/account": withAuth(() => json(ACCOUNT)),

    "/api/auths/google": {
      POST: async (req) => {
        let googleProfile: any = {};
        try {
          const binPath = "/home/ubuntu/.local/bin/aphrody";
          const proc = Bun.spawn([binPath, "antigravity", "whoami", "--json"], {
            stdout: "pipe",
            stderr: "pipe",
          });
          const stdout = await new Response(proc.stdout).text();
          googleProfile = JSON.parse(stdout);
        } catch (err) {
          console.error("Failed to run whoami, fallback profile:", err);
          googleProfile = {
            email: "contact@aphrody-code.dev",
            name: "Yohan Pierre",
            picture: "https://lh3.googleusercontent.com/a/ACg8ocLkl45UDyENhglg8S50w0TGnCAl9I9TxV59fbJwfP--R2qO63EfnA=s96-c",
            given_name: "Yohan",
            family_name: "Pierre",
            id: "101187968385849974848"
          };
        }

        const cookies = req.headers.get("cookie") ?? "";

        let serviceAccount: any = {};
        try {
          serviceAccount = await Bun.file("/home/ubuntu/aphrody/secrets/aphrody-bot.json").json();
        } catch {}

        let clientSecret: any = {};
        try {
          clientSecret = await Bun.file("/home/ubuntu/aphrody/secrets/client_secret_468000409790-oubhlpdp9rfb569vre9l1ikpdq4lc3ru.apps.googleusercontent.com.json").json();
        } catch {}

        const mixedData = {
          connected: true,
          user: {
            id: googleProfile.id || "u-google",
            name: googleProfile.name || "Yohan Pierre",
            email: googleProfile.email || "contact@aphrody-code.dev",
            profile_image_url: googleProfile.picture || "/favicon.png",
            role: "admin",
            token: EXPECTED_TOKEN,
          },
          mix: {
            google_api_key: process.env.GOOGLE_API_KEY || "AIzaSyCDn0U6iX_J6bF0mGarEYvYx2dKrQ_XRrQ",
            gcp_service_account: serviceAccount.client_email || process.env.GCP_SERVICE_ACCOUNT || "",
            client_id: clientSecret.installed?.client_id || "",
            project_id: serviceAccount.project_id || "aphrody",
            cookies: cookies,
            scopes: ["whoami", "gemini-api", "cloud-platform"],
          }
        };

        return json(mixedData);
      }
    },

    "/assets/*": {
      GET: async (req) => {
        const path = (req.params as any)["*"];
        const file = Bun.file(`/home/ubuntu/aphrody/assets/${path}`);
        if (await file.exists()) {
          return new Response(file);
        }
        return new Response("Not found", { status: 404 });
      }
    },

    // SPA + bundling catch-all (HTML import auto-bundles main.tsx + CSS).
    "/*": index,
  },
});

console.log(`Open WebUI · M3  →  http://localhost:${server.port}`);

