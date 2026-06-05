// Sidebar: new-chat CTA, section nav, search, and the chat history grouped by
// recency (Today / Yesterday / Previous 7 days / Older), each with a delete action.

import { useMemo, useState } from "react";
import { useNavigate, useRouterState } from "@tanstack/react-router";
import {
  MdFilledTonalButton,
  MdIcon,
  MdIconButton,
  MdList,
  MdListItem,
  MdSearchBar,
} from "@aphrody/m3-react";
import { useChats, useCreateChat, useDeleteChat } from "../api/queries.ts";
import type { ChatListItem } from "../api/types.ts";

const DAY = 86_400_000;

function bucket(ts: number): string {
  const age = Date.now() - ts;
  if (age < DAY) return "Aujourd'hui";
  if (age < 2 * DAY) return "Hier";
  if (age < 7 * DAY) return "7 derniers jours";
  return "Plus anciens";
}

const NAV = [
  { to: "/", icon: "chat", label: "Chats" },
  { to: "/workspace", icon: "workspaces", label: "Espace de travail" },
  { to: "/notes", icon: "edit_note", label: "Notes" },
  { to: "/admin", icon: "admin_panel_settings", label: "Admin" },
] as const;

export function Sidebar() {
  const { data: chats = [] } = useChats();
  const createChat = useCreateChat();
  const deleteChat = useDeleteChat();
  const navigate = useNavigate();
  const path = useRouterState({ select: (s) => s.location.pathname });
  const [query, setQuery] = useState("");

  const groups = useMemo(() => {
    const filtered = chats.filter((c) => c.title.toLowerCase().includes(query.toLowerCase()));
    const order = ["Aujourd'hui", "Hier", "7 derniers jours", "Plus anciens"];
    const map = new Map<string, ChatListItem[]>();
    for (const c of filtered) {
      const k = bucket(c.updated_at);
      (map.get(k) ?? map.set(k, []).get(k)!).push(c);
    }
    return order.filter((k) => map.has(k)).map((k) => [k, map.get(k)!] as const);
  }, [chats, query]);

  const onNewChat = async () => {
    const chat = await createChat.mutateAsync({ title: "Nouveau chat" });
    void navigate({ to: "/c/$chatId", params: { chatId: chat.id } });
  };

  const onDelete = async (id: string) => {
    await deleteChat.mutateAsync(id);
    if (path === `/c/${id}`) void navigate({ to: "/" });
  };

  return (
    <>
      <div style={{ padding: "12px 12px 4px" }}>
        <MdFilledTonalButton style={{ width: "100%" }} onClick={() => void onNewChat()}>
          <MdIcon slot="icon">add</MdIcon>
          Nouveau chat
        </MdFilledTonalButton>
      </div>

      <div className="owui-row" style={{ padding: "4px 8px", gap: 2, flexWrap: "wrap" }}>
        {NAV.map((n) => {
          const active =
            n.to === "/" ? path === "/" || path.startsWith("/c/") : path.startsWith(n.to);
          return (
            <MdIconButton
              key={n.to}
              aria-label={n.label}
              onClick={() => void navigate({ to: n.to })}
              style={active ? { color: "var(--md-sys-color-primary)" } : undefined}
            >
              <MdIcon>{n.icon}</MdIcon>
            </MdIconButton>
          );
        })}
      </div>

      <div style={{ padding: "4px 12px 8px" }}>
        <MdSearchBar
          value={query}
          placeholder="Rechercher"
          onInput={(e) => setQuery((e.target as HTMLInputElement).value)}
        />
      </div>

      <nav className="owui-sidebar__list">
        {groups.length === 0 && (
          <p className="owui-muted" style={{ padding: "8px 12px" }}>
            Aucun chat pour le moment.
          </p>
        )}
        {groups.map(([label, items]) => (
          <section key={label}>
            <p
              className="owui-muted"
              style={{ margin: "10px 12px 2px", fontSize: 12, fontWeight: 600, letterSpacing: 0.4 }}
            >
              {label}
            </p>
            <MdList>
              {items.map((c) => {
                const active = path === `/c/${c.id}`;
                return (
                  <MdListItem
                    key={c.id}
                    type="button"
                    onClick={() => void navigate({ to: "/c/$chatId", params: { chatId: c.id } })}
                    style={
                      active
                        ? ({
                            "--md-list-item-container-color":
                              "var(--md-sys-color-secondary-container)",
                          } as React.CSSProperties)
                        : undefined
                    }
                  >
                    <MdIcon slot="start">{c.pinned ? "push_pin" : "chat_bubble"}</MdIcon>
                    <span
                      slot="headline"
                      style={{ overflow: "hidden", textOverflow: "ellipsis", whiteSpace: "nowrap" }}
                    >
                      {c.title}
                    </span>
                    <MdIconButton
                      slot="end"
                      aria-label="Supprimer le chat"
                      onClick={(e: React.MouseEvent) => {
                        e.stopPropagation();
                        void onDelete(c.id);
                      }}
                    >
                      <MdIcon>delete</MdIcon>
                    </MdIconButton>
                  </MdListItem>
                );
              })}
            </MdList>
          </section>
        ))}
      </nav>
    </>
  );
}
