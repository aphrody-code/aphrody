// Owns one chat's message tree, the streaming lifecycle, and persistence.
// Mirrors open-webui's history model ({ messages: {id->msg}, currentId }) and its
// linear render path (walk parents back from currentId). Streams via the SSE client.

import { useCallback, useEffect, useRef, useState } from "react";
import { streamCompletion } from "../../api/stream.ts";
import { useUpdateChat } from "../../api/queries.ts";
import type { Chat, ChatHistory, ChatMessage } from "../../api/types.ts";

const uid = () => (crypto.randomUUID ? crypto.randomUUID() : `${Date.now()}-${Math.random()}`);
const nowSec = () => Math.floor(Date.now() / 1000);

/** Linear path of messages from root to currentId (the visible conversation). */
export function linearMessages(history: ChatHistory): ChatMessage[] {
  const out: ChatMessage[] = [];
  let id = history.currentId;
  while (id) {
    const m = history.messages[id];
    if (!m) break;
    out.unshift(m);
    id = m.parentId;
  }
  return out;
}

export interface ChatSession {
  history: ChatHistory;
  messages: ChatMessage[];
  streaming: boolean;
  send: (prompt: string, model: string) => Promise<void>;
  regenerate: (model: string) => Promise<void>;
  editUser: (id: string, content: string, model: string) => Promise<void>;
  stop: () => void;
}

export function useChatSession(chat: Chat | undefined): ChatSession {
  const [history, setHistory] = useState<ChatHistory>({ messages: {}, currentId: null });
  const [streaming, setStreaming] = useState(false);
  const abortRef = useRef<AbortController | null>(null);
  const update = useUpdateChat();

  // Adopt server history when the chat loads / changes.
  useEffect(() => {
    if (chat) setHistory(chat.history ?? { messages: {}, currentId: null });
  }, [chat?.id]);

  const persist = useCallback(
    (next: ChatHistory, title?: string) => {
      if (!chat) return;
      void update.mutateAsync({
        id: chat.id,
        patch: { history: next, ...(title ? { title } : {}) },
      });
    },
    [chat?.id],
  );

  const runStream = useCallback(
    async (working: ChatHistory, assistantId: string, model: string, titleSeed?: string) => {
      const path = linearMessages(working).map((m) => ({ role: m.role, content: m.content }));
      const controller = new AbortController();
      abortRef.current = controller;
      setStreaming(true);

      let acc = "";
      let usage: ChatMessage["usage"];
      try {
        for await (const upd of streamCompletion(
          { model, messages: path, stream: true },
          controller.signal,
        )) {
          if (upd.usage) usage = upd.usage;
          if (upd.value) {
            acc += upd.value;
            setHistory((h) => ({
              ...h,
              messages: {
                ...h.messages,
                [assistantId]: { ...h.messages[assistantId], content: acc },
              },
            }));
          }
          if (upd.done) break;
        }
      } catch {
        /* aborted */
      }

      setHistory((h) => {
        const finalised: ChatHistory = {
          ...h,
          messages: {
            ...h.messages,
            [assistantId]: { ...h.messages[assistantId], content: acc, done: true, usage },
          },
        };
        persist(finalised, titleSeed);
        return finalised;
      });
      setStreaming(false);
      abortRef.current = null;
    },
    [persist],
  );

  const send = useCallback(
    async (prompt: string, model: string) => {
      const text = prompt.trim();
      if (!text || streaming) return;

      const userId = uid();
      const assistantId = uid();
      const parentId = history.currentId;

      const userMsg: ChatMessage = {
        id: userId,
        parentId,
        childrenIds: [assistantId],
        role: "user",
        content: text,
        timestamp: nowSec(),
        done: true,
      };
      const assistantMsg: ChatMessage = {
        id: assistantId,
        parentId: userId,
        childrenIds: [],
        role: "assistant",
        content: "",
        model,
        timestamp: nowSec(),
        done: false,
      };

      const messages = { ...history.messages };
      if (parentId && messages[parentId]) {
        messages[parentId] = {
          ...messages[parentId],
          childrenIds: [...messages[parentId].childrenIds, userId],
        };
      }
      messages[userId] = userMsg;
      messages[assistantId] = assistantMsg;
      const working: ChatHistory = { messages, currentId: assistantId };
      setHistory(working);

      const isFirst = !parentId;
      await runStream(working, assistantId, model, isFirst ? text.slice(0, 48) : undefined);
    },
    [history, streaming, runStream],
  );

  const regenerate = useCallback(
    async (model: string) => {
      if (streaming) return;
      const current = history.currentId ? history.messages[history.currentId] : null;
      if (!current || current.role !== "assistant" || !current.parentId) return;

      const assistantId = uid();
      const userParent = current.parentId;
      const fresh: ChatMessage = {
        id: assistantId,
        parentId: userParent,
        childrenIds: [],
        role: "assistant",
        content: "",
        model,
        timestamp: nowSec(),
        done: false,
      };
      const messages = { ...history.messages };
      messages[userParent] = {
        ...messages[userParent],
        childrenIds: [...messages[userParent].childrenIds, assistantId],
      };
      messages[assistantId] = fresh;
      const working: ChatHistory = { messages, currentId: assistantId };
      setHistory(working);
      await runStream(working, assistantId, model);
    },
    [history, streaming, runStream],
  );

  const editUser = useCallback(
    async (id: string, content: string, model: string) => {
      if (streaming) return;
      const original = history.messages[id];
      if (!original || original.role !== "user") return;

      const userId = uid();
      const assistantId = uid();
      const edited: ChatMessage = {
        ...original,
        id: userId,
        content: content.trim(),
        childrenIds: [assistantId],
        timestamp: nowSec(),
      };
      const assistantMsg: ChatMessage = {
        id: assistantId,
        parentId: userId,
        childrenIds: [],
        role: "assistant",
        content: "",
        model,
        timestamp: nowSec(),
        done: false,
      };
      const messages = { ...history.messages };
      if (original.parentId && messages[original.parentId]) {
        messages[original.parentId] = {
          ...messages[original.parentId],
          childrenIds: [...messages[original.parentId].childrenIds, userId],
        };
      }
      messages[userId] = edited;
      messages[assistantId] = assistantMsg;
      const working: ChatHistory = { messages, currentId: assistantId };
      setHistory(working);
      await runStream(working, assistantId, model);
    },
    [history, streaming, runStream],
  );

  const stop = useCallback(() => abortRef.current?.abort(), []);

  return {
    history,
    messages: linearMessages(history),
    streaming,
    send,
    regenerate,
    editUser,
    stop,
  };
}
