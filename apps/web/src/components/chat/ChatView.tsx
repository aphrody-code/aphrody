// The chat screen: model header, scrolling message list (auto-stick to bottom),
// empty-state prompt suggestions, and the composer. The heart of open-webui.

import { useEffect, useLayoutEffect, useRef, useState } from "react";
import { MdIcon, MdMenuItem, MdCarousel, MdCarouselItem } from "@aphrody/m3-react";
import { ModelSelector } from "./ModelSelector.tsx";
import { MessageBubble } from "./MessageBubble.tsx";
import { Composer } from "./Composer.tsx";
import { Menu } from "../ui/Menu.tsx";
import { useChatSession } from "./useChatSession.ts";
import { useConfig, useModels } from "../../api/queries.ts";
import type { Chat } from "../../api/types.ts";

export function ChatView({ chat }: { chat: Chat | undefined }) {
  const { data: config } = useConfig();
  const { data: models = [] } = useModels();
  const session = useChatSession(chat);
  const { messages, streaming } = session;

  const [model, setModel] = useState("");
  const [prompt, setPrompt] = useState("");
  const scrollerRef = useRef<HTMLDivElement>(null);
  const stick = useRef(true);

  useEffect(() => {
    const fallback = chat?.models?.[0] ?? config?.default_models?.[0] ?? models[0]?.id ?? "";
    setModel((m) => m || fallback);
  }, [chat?.id, config, models]);

  // Auto-scroll: keep pinned to bottom unless the user scrolled up.
  useLayoutEffect(() => {
    const el = scrollerRef.current;
    if (el && stick.current) el.scrollTop = el.scrollHeight;
  }, [messages]);

  const onScroll = () => {
    const el = scrollerRef.current;
    if (!el) return;
    stick.current = el.scrollHeight - el.scrollTop - el.clientHeight < 80;
  };

  const submit = (text: string) => {
    stick.current = true;
    setPrompt("");
    void session.send(text, model);
  };

  return (
    <div className="owui-chat">
      <div
        className="owui-spread"
        style={{
          padding: "6px 16px",
          borderBottom: "1px solid var(--md-sys-color-outline-variant)",
        }}
      >
        <ModelSelector value={model} onChange={setModel} />
        <span
          className="owui-muted"
          style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}
        >
          {chat?.title}
        </span>
        <Menu
          trigger={({ toggle }) => (
            <button
              onClick={toggle}
              aria-label="Chat options"
              style={{
                border: 0,
                background: "transparent",
                cursor: "pointer",
                color: "inherit",
                padding: 6,
              }}
            >
              <MdIcon>more_vert</MdIcon>
            </button>
          )}
        >
          <MdMenuItem>
            <MdIcon slot="start">share</MdIcon>
            <span slot="headline">Partager</span>
          </MdMenuItem>
          <MdMenuItem>
            <MdIcon slot="start">archive</MdIcon>
            <span slot="headline">Archiver</span>
          </MdMenuItem>
          <MdMenuItem>
            <MdIcon slot="start">download</MdIcon>
            <span slot="headline">Télécharger</span>
          </MdMenuItem>
        </Menu>
      </div>

      <div className="owui-messages" ref={scrollerRef} onScroll={onScroll}>
        {messages.length === 0 ? (
          <div className="owui-empty">
            <MdIcon
              style={{ fontSize: 48, color: "var(--md-sys-color-primary)" } as React.CSSProperties}
            >
              forum
            </MdIcon>
            <div>
              <h2 style={{ margin: 0 }}>Comment puis-je vous aider aujourd'hui ?</h2>
              <p className="owui-muted" style={{ margin: "4px 0 0" }}>
                Choisissez une suggestion ou saisissez votre propre message.
              </p>
            </div>
            <div style={{ maxWidth: "600px", width: "100%", marginTop: 20 }}>
              <MdCarousel layout="multi-browse" showArrows={true}>
                {(config?.default_prompt_suggestions ?? []).map((s, i) => (
                  <MdCarouselItem key={i} size="medium" style={{ width: "200px", height: "100px", cursor: "pointer" }} onClick={() => submit(s.content)}>
                    <div
                      style={{
                        padding: 12,
                        borderRadius: 16,
                        background: "var(--md-sys-color-surface-container-high)",
                        border: "1px solid var(--md-sys-color-outline-variant)",
                        height: "100%",
                        display: "flex",
                        flexDirection: "column",
                        justifyContent: "space-between",
                        boxSizing: "border-box",
                        userSelect: "none"
                      }}
                    >
                      <div style={{ fontWeight: "bold", fontSize: 13, color: "var(--md-sys-color-on-surface)", overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}>
                        {s.title[0]}
                      </div>
                      <div style={{ fontSize: 12, color: "var(--md-sys-color-on-surface-variant)", display: "-webkit-box", WebkitLineClamp: 2, WebkitBoxOrient: "vertical", overflow: "hidden" }}>
                        {s.title[1]}
                      </div>
                    </div>
                  </MdCarouselItem>
                ))}
              </MdCarousel>
            </div>
          </div>
        ) : (
          <div className="owui-messages__inner">
            {messages.map((m) => (
              <MessageBubble
                key={m.id}
                message={m}
                streaming={streaming}
                onRegenerate={() => void session.regenerate(model)}
                onEdit={(content) => void session.editUser(m.id, content, model)}
              />
            ))}
          </div>
        )}
      </div>

      <Composer
        value={prompt}
        onValue={setPrompt}
        streaming={streaming}
        onStop={session.stop}
        onSend={() => submit(prompt)}
      />
    </div>
  );
}
