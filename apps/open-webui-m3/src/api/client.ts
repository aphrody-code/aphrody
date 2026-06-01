// Thin typed fetch client. open-webui sends `Authorization: Bearer <token>`;
// the mock backend ignores it, but we keep the convention so the surface is real.

import type {
  AdminUser,
  BackendConfig,
  Chat,
  ChatListItem,
  KnowledgeCollection,
  Model,
  Prompt,
  SessionUser,
  Tool,
  WorkspaceModel,
} from "./types.ts";

const TOKEN_KEY = "owui-m3-token";

export const auth = {
  get token(): string | null {
    return localStorage.getItem(TOKEN_KEY);
  },
  set(token: string) {
    localStorage.setItem(TOKEN_KEY, token);
  },
  clear() {
    localStorage.removeItem(TOKEN_KEY);
  },
};

async function http<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(path, {
    ...init,
    headers: {
      "content-type": "application/json",
      ...(auth.token ? { authorization: `Bearer ${auth.token}` } : {}),
      ...init?.headers,
    },
  });
  if (!res.ok) {
    const detail = await res.json().catch(() => ({ detail: res.statusText }));
    throw new Error((detail as { detail?: string }).detail ?? `HTTP ${res.status}`);
  }
  return res.json() as Promise<T>;
}

export const api = {
  getConfig: () => http<BackendConfig>("/api/config"),

  signIn: (email: string, password: string) =>
    http<SessionUser>("/api/auths/signin", {
      method: "POST",
      body: JSON.stringify({ email, password }),
    }),
  signUp: (name: string, email: string, password: string) =>
    http<SessionUser>("/api/auths/signup", {
      method: "POST",
      body: JSON.stringify({ name, email, password }),
    }),
  getSession: () => http<SessionUser>("/api/auths"),

  getModels: () => http<Model[]>("/api/models"),

  getChats: () => http<ChatListItem[]>("/api/chats"),
  getChat: (id: string) => http<Chat>(`/api/chats/${id}`),
  createChat: (chat: Partial<Chat>) =>
    http<Chat>("/api/chats/new", { method: "POST", body: JSON.stringify(chat) }),
  updateChat: (id: string, patch: Partial<Chat>) =>
    http<Chat>(`/api/chats/${id}`, { method: "POST", body: JSON.stringify(patch) }),
  deleteChat: (id: string) => http<{ ok: boolean }>(`/api/chats/${id}`, { method: "DELETE" }),

  getAdminUsers: () => http<AdminUser[]>("/api/users"),
  getWorkspaceModels: () => http<WorkspaceModel[]>("/api/workspace/models"),
  getKnowledge: () => http<KnowledgeCollection[]>("/api/workspace/knowledge"),
  getPrompts: () => http<Prompt[]>("/api/workspace/prompts"),
  getTools: () => http<Tool[]>("/api/workspace/tools"),
};
