// SPDX-License-Identifier: Apache-2.0
"use client";

import { useCallback, useState } from "react";
import type { ChatTurn } from "../app/components/MessageBubble";

export interface UseChatResult {
  turns: ReadonlyArray<ChatTurn>;
  busy: boolean;
  error: string | null;
  send(prompt: string): Promise<void>;
  reset(): void;
}

function newId(): string {
  return `${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
}

export function useChat(): UseChatResult {
  const [turns, setTurns] = useState<ReadonlyArray<ChatTurn>>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const reset = useCallback(() => {
    setTurns([]);
    setError(null);
    setBusy(false);
  }, []);

  const send = useCallback(async (prompt: string) => {
    const trimmed = prompt.trim();
    if (!trimmed) return;
    setError(null);
    const userTurn: ChatTurn = {
      id: newId(),
      role: "user",
      text: trimmed,
      ts: Date.now(),
    };
    const placeholder: ChatTurn = {
      id: newId(),
      role: "assistant",
      text: "Thinking...",
      ts: Date.now(),
      streaming: true,
    };
    setTurns((prev) => [...prev, userTurn, placeholder]);
    setBusy(true);
    try {
      const response = await fetch("/api/chat", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ prompt: trimmed }),
      });
      const payload = (await response.json()) as
        | { reply: string }
        | { error: string };
      if (!response.ok || "error" in payload) {
        const message =
          "error" in payload ? payload.error : `HTTP ${response.status}`;
        setError(message);
        setTurns((prev) =>
          prev.map((t) =>
            t.id === placeholder.id
              ? { ...t, text: `Error: ${message}`, streaming: false }
              : t,
          ),
        );
        return;
      }
      setTurns((prev) =>
        prev.map((t) =>
          t.id === placeholder.id
            ? {
                ...t,
                text: (payload as { reply: string }).reply || "(empty reply)",
                streaming: false,
                ts: Date.now(),
              }
            : t,
        ),
      );
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
      setTurns((prev) =>
        prev.map((t) =>
          t.id === placeholder.id
            ? { ...t, text: `Error: ${message}`, streaming: false }
            : t,
        ),
      );
    } finally {
      setBusy(false);
    }
  }, []);

  return { turns, busy, error, send, reset };
}
