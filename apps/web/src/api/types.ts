// Core data shapes, distilled from open-webui's src/lib/apis + stores.
// Shared by the typed client (browser) and the Bun.serve mock backend (node).

export type Role = "user" | "assistant" | "system";
export type UserRole = "admin" | "user" | "pending";

export interface SessionUser {
  id: string;
  name: string;
  email: string;
  role: UserRole;
  profile_image_url: string;
  token: string;
}

export interface Model {
  id: string;
  name: string;
  description?: string;
  owned_by: "ollama" | "openai" | "arena";
  capabilities?: {
    vision?: boolean;
    web_search?: boolean;
    image_generation?: boolean;
    code_interpreter?: boolean;
  };
}

export interface MessageFile {
  type: string;
  name: string;
  url: string;
}

export interface ChatMessage {
  id: string;
  parentId: string | null;
  childrenIds: string[];
  role: Role;
  content: string;
  model?: string;
  modelName?: string;
  timestamp: number;
  done?: boolean;
  error?: string;
  files?: MessageFile[];
  usage?: {
    prompt_tokens: number;
    completion_tokens: number;
    total_tokens: number;
  };
}

export interface ChatHistory {
  messages: Record<string, ChatMessage>;
  currentId: string | null;
}

export interface Chat {
  id: string;
  title: string;
  models: string[];
  history: ChatHistory;
  pinned?: boolean;
  archived?: boolean;
  created_at: number;
  updated_at: number;
}

/** Lightweight list-item projection of a chat for the sidebar. */
export interface ChatListItem {
  id: string;
  title: string;
  pinned?: boolean;
  updated_at: number;
}

export interface WorkspaceModel {
  id: string;
  name: string;
  description: string;
  base_model_id: string;
  tags: string[];
  visibility: "public" | "private";
  created_at: number;
}

export interface KnowledgeCollection {
  id: string;
  name: string;
  description: string;
  file_count: number;
  created_at: number;
}

export interface Prompt {
  id: string;
  command: string;
  title: string;
  content: string;
  tags: string[];
  created_at: number;
}

export interface Tool {
  id: string;
  name: string;
  description: string;
  type: "custom" | "openapi";
  created_at: number;
}

export interface AdminUser {
  id: string;
  name: string;
  email: string;
  role: UserRole;
  last_active_at: number;
  created_at: number;
}

export interface PromptSuggestion {
  title: [string, string];
  content: string;
}

export interface BackendConfig {
  name: string;
  version: string;
  default_models: string[];
  default_prompt_suggestions: PromptSuggestion[];
  features: {
    auth: boolean;
    enable_signup: boolean;
    enable_web_search: boolean;
    enable_image_generation: boolean;
    enable_admin_panel: boolean;
  };
  oauth: { providers: Record<string, string> };
}

/** OpenAI-compatible chat-completion request body. */
export interface CompletionRequest {
  model: string;
  messages: { role: Role; content: string }[];
  stream: true;
  temperature?: number;
}

/** Parsed SSE delta, mirrors open-webui's TextStreamUpdate. */
export interface StreamUpdate {
  done: boolean;
  value: string;
  usage?: ChatMessage["usage"];
  error?: string;
}
