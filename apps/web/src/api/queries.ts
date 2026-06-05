import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { api } from "./client.ts";
import type {
  Chat,
  WorkspaceModel,
  KnowledgeCollection,
  Prompt,
  Tool,
  AdminUser,
} from "./types.ts";

export const qk = {
  config: ["config"] as const,
  models: ["models"] as const,
  chats: ["chats"] as const,
  chat: (id: string) => ["chat", id] as const,
  adminUsers: ["admin", "users"] as const,
  wsModels: ["workspace", "models"] as const,
  knowledge: ["workspace", "knowledge"] as const,
  prompts: ["workspace", "prompts"] as const,
  tools: ["workspace", "tools"] as const,
};

export const useConfig = () =>
  useQuery({ queryKey: qk.config, queryFn: api.getConfig, staleTime: Infinity });
export const useModels = () =>
  useQuery({ queryKey: qk.models, queryFn: api.getModels, staleTime: 60_000 });
export const useChats = () => useQuery({ queryKey: qk.chats, queryFn: api.getChats });

export const useChat = (id: string | undefined) =>
  useQuery({ queryKey: qk.chat(id ?? ""), queryFn: () => api.getChat(id!), enabled: !!id });

export const useAdminUsers = () =>
  useQuery({ queryKey: qk.adminUsers, queryFn: api.getAdminUsers });
export const useWorkspaceModels = () =>
  useQuery({ queryKey: qk.wsModels, queryFn: api.getWorkspaceModels });
export const useKnowledge = () => useQuery({ queryKey: qk.knowledge, queryFn: api.getKnowledge });
export const usePrompts = () => useQuery({ queryKey: qk.prompts, queryFn: api.getPrompts });
export const useTools = () => useQuery({ queryKey: qk.tools, queryFn: api.getTools });

export function useCreateChat() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (chat: Partial<Chat>) => api.createChat(chat),
    onSuccess: () => void qc.invalidateQueries({ queryKey: qk.chats }),
  });
}

export function useUpdateChat() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: ({ id, patch }: { id: string; patch: Partial<Chat> }) => api.updateChat(id, patch),
    onSuccess: (chat) => {
      qc.setQueryData(qk.chat(chat.id), chat);
      void qc.invalidateQueries({ queryKey: qk.chats });
    },
  });
}

export function useDeleteChat() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.deleteChat(id),
    onSuccess: () => void qc.invalidateQueries({ queryKey: qk.chats }),
  });
}

export function useCreateWorkspaceModel() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (data: Partial<WorkspaceModel>) => api.createWorkspaceModel(data),
    onSuccess: () => void qc.invalidateQueries({ queryKey: qk.wsModels }),
  });
}

export function useDeleteWorkspaceModel() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.deleteWorkspaceModel(id),
    onSuccess: () => void qc.invalidateQueries({ queryKey: qk.wsModels }),
  });
}

export function useCreateKnowledge() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (data: Partial<KnowledgeCollection>) => api.createKnowledge(data),
    onSuccess: () => void qc.invalidateQueries({ queryKey: qk.knowledge }),
  });
}

export function useDeleteKnowledge() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.deleteKnowledge(id),
    onSuccess: () => void qc.invalidateQueries({ queryKey: qk.knowledge }),
  });
}

export function useCreatePrompt() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (data: Partial<Prompt>) => api.createPrompt(data),
    onSuccess: () => void qc.invalidateQueries({ queryKey: qk.prompts }),
  });
}

export function useDeletePrompt() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.deletePrompt(id),
    onSuccess: () => void qc.invalidateQueries({ queryKey: qk.prompts }),
  });
}

export function useCreateTool() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (data: Partial<Tool>) => api.createTool(data),
    onSuccess: () => void qc.invalidateQueries({ queryKey: qk.tools }),
  });
}

export function useDeleteTool() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.deleteTool(id),
    onSuccess: () => void qc.invalidateQueries({ queryKey: qk.tools }),
  });
}

export function useCreateAdminUser() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (data: { name: string; email: string; role: string }) => api.createAdminUser(data),
    onSuccess: () => void qc.invalidateQueries({ queryKey: qk.adminUsers }),
  });
}

export function useDeleteAdminUser() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (id: string) => api.deleteAdminUser(id),
    onSuccess: () => void qc.invalidateQueries({ queryKey: qk.adminUsers }),
  });
}
