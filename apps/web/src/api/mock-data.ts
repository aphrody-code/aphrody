// In-memory seed for the Bun.serve mock backend. No DB — open-webui's surface,
// faithfully shaped, just enough to drive the M3 React client end-to-end.

import type {
  AdminUser,
  BackendConfig,
  Chat,
  ChatMessage,
  KnowledgeCollection,
  Model,
  Prompt,
  SessionUser,
  Tool,
  WorkspaceModel,
} from "./types.ts";

const now = () => Date.now();

export const SESSION_USER: SessionUser = {
  id: "u-admin",
  name: "Ada Lovelace",
  email: "ada@example.com",
  role: "admin",
  profile_image_url: "/favicon.png",
  token: "mock-token",
};

// Public consumer surface: custom in-house RAG + LLMs (shenron, rpbey, …),
// not third-party hosted models. The admin/ops tooling lives in the private app.
export const MODELS: Model[] = [
  {
    id: "shenron",
    name: "Shenron",
    description: "Custom Dragon Ball RAG assistant (in-house LLM + knowledge base).",
    owned_by: "ollama",
    capabilities: { vision: true, web_search: true },
  },
  {
    id: "rpbey",
    name: "RPBey",
    description: "Custom in-house assistant tuned on the rpbey corpus.",
    owned_by: "ollama",
    capabilities: { web_search: true },
  },
  {
    id: "aphrody-rag",
    name: "aphrody RAG",
    description: "Retrieval-augmented generation over your linked knowledge collections.",
    owned_by: "ollama",
    capabilities: { web_search: true, code_interpreter: true },
  },
  {
    id: "aphrody-vision",
    name: "aphrody Vision",
    description: "Multimodal in-house model (image + text).",
    owned_by: "ollama",
    capabilities: { vision: true, image_generation: true },
  },
];

export const CONFIG: BackendConfig = {
  name: "aphrody",
  version: "0.1.0",
  default_models: ["shenron"],
  default_prompt_suggestions: [
    {
      title: ["Explain a concept", "like I'm five"],
      content: "Explain quantum entanglement like I'm five.",
    },
    { title: ["Write code", "a debounce hook"], content: "Write a typed React useDebounce hook." },
    {
      title: ["Brainstorm", "product names"],
      content: "Brainstorm 10 names for a Bun-native chat app.",
    },
    {
      title: ["Summarize", "a long article"],
      content: "Summarize the key ideas of Material Design 3.",
    },
  ],
  features: {
    auth: true,
    enable_signup: true,
    enable_web_search: true,
    enable_image_generation: true,
    enable_admin_panel: true,
  },
  oauth: { providers: { google: "Google", microsoft: "Microsoft", github: "GitHub" } },
};

function msg(
  partial: Partial<ChatMessage> & Pick<ChatMessage, "id" | "role" | "content">,
): ChatMessage {
  return {
    parentId: null,
    childrenIds: [],
    timestamp: Math.floor(now() / 1000),
    done: true,
    ...partial,
  };
}

function seedChat(id: string, title: string, turns: [string, string][], ageMin: number): Chat {
  const messages: Record<string, ChatMessage> = {};
  let parentId: string | null = null;
  let currentId: string | null = null;
  turns.forEach(([u, a], i) => {
    const uid = `${id}-u${i}`;
    const aid = `${id}-a${i}`;
    messages[uid] = msg({ id: uid, role: "user", content: u, parentId, childrenIds: [aid] });
    messages[aid] = msg({
      id: aid,
      role: "assistant",
      content: a,
      model: "shenron",
      modelName: "Shenron",
      parentId: uid,
      childrenIds: [],
      usage: { prompt_tokens: 24, completion_tokens: 80, total_tokens: 104 },
    });
    if (parentId) messages[parentId].childrenIds = [uid];
    parentId = aid;
    currentId = aid;
  });
  return {
    id,
    title,
    models: ["shenron"],
    history: { messages, currentId },
    created_at: now() - ageMin * 60_000,
    updated_at: now() - ageMin * 60_000,
  };
}

export const CHATS: Chat[] = [
  seedChat(
    "c-welcome",
    "Welcome to Open WebUI M3",
    [
      [
        "What is this app?",
        "This is a full Material Design 3 rebuild of Open WebUI, written in React and served Bun-natively on TanStack Router + TanStack Query. Every control you see is a `<md-*>` web component wrapped for React.",
      ],
    ],
    5,
  ),
  seedChat(
    "c-bun",
    "Bun-native serving",
    [
      [
        "How is it served?",
        "`Bun.serve` hosts the bundle and a mock OpenAI-compatible API; `bun build ./src/index.html` produces the production bundle. No Next, no Vite.",
      ],
    ],
    90,
  ),
  seedChat(
    "c-tanstack",
    "TanStack stack",
    [
      [
        "Which router?",
        "TanStack Router drives navigation; TanStack Query owns all server state (chats, models, config) with optimistic mutations.",
      ],
    ],
    1500,
  ),
];

export const ADMIN_USERS: AdminUser[] = [
  {
    id: "u-admin",
    name: "Ada Lovelace",
    email: "ada@example.com",
    role: "admin",
    last_active_at: now(),
    created_at: now() - 86_400_000 * 120,
  },
  {
    id: "u-2",
    name: "Alan Turing",
    email: "alan@example.com",
    role: "user",
    last_active_at: now() - 3_600_000,
    created_at: now() - 86_400_000 * 60,
  },
  {
    id: "u-3",
    name: "Grace Hopper",
    email: "grace@example.com",
    role: "user",
    last_active_at: now() - 7_200_000,
    created_at: now() - 86_400_000 * 30,
  },
  {
    id: "u-4",
    name: "Katherine Johnson",
    email: "katherine@example.com",
    role: "pending",
    last_active_at: now() - 86_400_000,
    created_at: now() - 86_400_000 * 2,
  },
];

export const WORKSPACE_MODELS: WorkspaceModel[] = [
  {
    id: "wm-1",
    name: "Research Assistant",
    description: "Web-search tuned, cites sources.",
    base_model_id: "gpt-4o",
    tags: ["research", "search"],
    visibility: "public",
    created_at: now() - 86_400_000 * 10,
  },
  {
    id: "wm-2",
    name: "Code Reviewer",
    description: "Strict TS/Rust reviewer persona.",
    base_model_id: "qwen2.5-coder:7b",
    tags: ["code"],
    visibility: "private",
    created_at: now() - 86_400_000 * 4,
  },
];

export const KNOWLEDGE: KnowledgeCollection[] = [
  {
    id: "k-1",
    name: "Product Docs",
    description: "Internal handbook + specs.",
    file_count: 42,
    created_at: now() - 86_400_000 * 20,
  },
  {
    id: "k-2",
    name: "M3 Guidelines",
    description: "Material Design 3 reference.",
    file_count: 8,
    created_at: now() - 86_400_000 * 6,
  },
];

export const PROMPTS: Prompt[] = [
  {
    id: "p-1",
    command: "/summarize",
    title: "Summarize",
    content: "Summarize the following text in 3 bullets:\n{{text}}",
    tags: ["writing"],
    created_at: now() - 86_400_000 * 3,
  },
  {
    id: "p-2",
    command: "/translate",
    title: "Translate to French",
    content: "Translate to French, keep tone:\n{{text}}",
    tags: ["i18n"],
    created_at: now() - 86_400_000,
  },
];

export const TOOLS: Tool[] = [
  {
    id: "t-1",
    name: "Weather",
    description: "Fetch current weather by city.",
    type: "custom",
    created_at: now() - 86_400_000 * 8,
  },
  {
    id: "t-2",
    name: "GitHub API",
    description: "OpenAPI bridge to GitHub.",
    type: "openapi",
    created_at: now() - 86_400_000 * 2,
  },
];

/** Canned assistant reply — streamed token-by-token by the mock completion endpoint. */
export function fakeReply(prompt: string, model: string): string {
  const trimmed = prompt.trim().slice(0, 160);
  return [
    `Here's a Material 3 mock response from **${model}**.`,
    ``,
    `You asked: _"${trimmed}"_`,
    ``,
    `In the real Open WebUI this would stream from Ollama or an OpenAI-compatible endpoint. Here it streams from a \`Bun.serve\` SSE handler so you can see the M3 message bubbles, the typing indicator, and the auto-scroll behaviour exactly as the client renders them.`,
    ``,
    "```ts",
    "const reply = await fetch('/api/chat/completions', {",
    "  method: 'POST',",
    "  body: JSON.stringify({ model, messages, stream: true }),",
    "});",
    "```",
    ``,
    "Try editing your message, regenerating, or switching models from the header.",
  ].join("\n");
}
