// SPDX-License-Identifier: Apache-2.0
"use client";

import { useEffect, useRef, type JSX } from "react";
import { MessageBubble, type ChatTurn } from "./MessageBubble";

interface ConversationPaneProps {
  turns: ReadonlyArray<ChatTurn>;
}

export function ConversationPane({ turns }: ConversationPaneProps): JSX.Element {
  const bottomRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth", block: "end" });
  }, [turns.length]);

  return (
    <div className="conversation" aria-live="polite">
      {turns.map((t) => (
        <MessageBubble key={t.id} turn={t} />
      ))}
      <div ref={bottomRef} />
    </div>
  );
}
