// One message row: avatar + M3 bubble. Assistant content is Markdown-rendered and
// carries Copy / Regenerate actions; user messages carry Copy / Edit (in-place).

import { useState } from "react";
import { MdIcon, MdIconButton, MdOutlinedTextField, MdTextButton } from "@aphrody/m3-react";
import { Markdown } from "./Markdown.tsx";
import type { ChatMessage } from "../../api/types.ts";

function Avatar({ role }: { role: ChatMessage["role"] }) {
  const assistant = role === "assistant";
  return (
    <span
      aria-hidden
      style={{
        flex: "0 0 auto",
        width: 32,
        height: 32,
        borderRadius: "50%",
        display: "grid",
        placeItems: "center",
        background: assistant ? "var(--md-sys-color-primary)" : "var(--md-sys-color-tertiary)",
        color: assistant ? "var(--md-sys-color-on-primary)" : "var(--md-sys-color-on-tertiary)",
      }}
    >
      <MdIcon style={{ fontSize: 20 } as React.CSSProperties}>
        {assistant ? "smart_toy" : "person"}
      </MdIcon>
    </span>
  );
}

export function MessageBubble({
  message,
  streaming,
  onRegenerate,
  onEdit,
}: {
  message: ChatMessage;
  streaming: boolean;
  onRegenerate: () => void;
  onEdit: (content: string) => void;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(message.content);
  const isUser = message.role === "user";
  const empty = message.content.length === 0;

  return (
    <div className="owui-msg" data-role={message.role}>
      <Avatar role={message.role} />
      <div
        style={{
          minWidth: 0,
          display: "flex",
          flexDirection: "column",
          alignItems: isUser ? "flex-end" : "flex-start",
        }}
      >
        <div className="owui-msg__bubble">
          {editing ? (
            <div className="owui-stack" style={{ minWidth: 280 }}>
              <MdOutlinedTextField
                type="textarea"
                rows={3}
                value={draft}
                onInput={(e) => setDraft((e.target as HTMLInputElement).value)}
                style={{ width: "100%" }}
              />
              <div className="owui-row" style={{ justifyContent: "flex-end" }}>
                <MdTextButton
                  onClick={() => {
                    setEditing(false);
                    setDraft(message.content);
                  }}
                >
                  Annuler
                </MdTextButton>
                <MdTextButton
                  onClick={() => {
                    setEditing(false);
                    onEdit(draft);
                  }}
                >
                  Enregistrer et envoyer
                </MdTextButton>
              </div>
            </div>
          ) : isUser ? (
            message.content
          ) : empty && streaming ? (
            <span className="owui-typing" />
          ) : (
            <Markdown source={message.content} />
          )}
        </div>

        {!editing && (
          <div className="owui-msg__actions">
            <MdIconButton
              aria-label="Copy"
              onClick={() => void navigator.clipboard?.writeText(message.content)}
            >
              <MdIcon>content_copy</MdIcon>
            </MdIconButton>
            {isUser ? (
              <MdIconButton
                aria-label="Edit"
                disabled={streaming}
                onClick={() => {
                  setDraft(message.content);
                  setEditing(true);
                }}
              >
                <MdIcon>edit</MdIcon>
              </MdIconButton>
            ) : (
              message.done && (
                <MdIconButton aria-label="Regenerate" disabled={streaming} onClick={onRegenerate}>
                  <MdIcon>refresh</MdIcon>
                </MdIconButton>
              )
            )}
            {message.usage && (
              <span
                className="owui-muted"
                style={{ alignSelf: "center", fontSize: 11, marginInlineStart: 6 }}
              >
                {message.usage.total_tokens} tok
              </span>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
