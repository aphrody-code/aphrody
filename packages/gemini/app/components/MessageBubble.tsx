// SPDX-License-Identifier: Apache-2.0
"use client";

import type { JSX } from "react";

export type Role = "user" | "assistant";

export interface ChatTurn {
  id: string;
  role: Role;
  text: string;
  ts: number;
  streaming?: boolean;
}

interface MessageBubbleProps {
  turn: ChatTurn;
}

function formatHm(ts: number): string {
  const d = new Date(ts);
  const hh = d.getHours().toString().padStart(2, "0");
  const mm = d.getMinutes().toString().padStart(2, "0");
  return `${hh}:${mm}`;
}

export function MessageBubble({ turn }: MessageBubbleProps): JSX.Element {
  const classes = [
    "message",
    turn.role === "user" ? "message--user" : "message--assistant",
    turn.streaming ? "message--streaming" : null,
  ]
    .filter((s): s is string => Boolean(s))
    .join(" ");
  return (
    <article
      className={classes}
      aria-roledescription={turn.role === "user" ? "user message" : "assistant response"}
    >
      <div className="message__body">{turn.text}</div>
      <time className="message__ts">
        {formatHm(turn.ts)}
        {turn.streaming ? " · streaming..." : ""}
      </time>
    </article>
  );
}
