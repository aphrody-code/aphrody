// "/workspace" — Models / Knowledge / Prompts / Tools, each a searchable M3 card
// grid with a create CTA. Mirrors open-webui's workspace hub.

import { useMemo, useState } from "react";
import {
  MdAssistChip,
  MdFilledTonalButton,
  MdIcon,
  MdIconButton,
  MdOutlinedCard,
  MdPrimaryTab,
  MdSearchBar,
  MdTabs,
} from "@aphrody-code/m3-react";
import { Menu } from "../components/ui/Menu.tsx";
import { MdMenuItem } from "@aphrody-code/m3-react";
import { useKnowledge, usePrompts, useTools, useWorkspaceModels } from "../api/queries.ts";

interface Card {
  id: string;
  title: string;
  subtitle: string;
  icon: string;
  tags?: string[];
  badge?: string;
}

function CardGrid({ cards, createLabel }: { cards: Card[]; createLabel: string }) {
  const [q, setQ] = useState("");
  const filtered = useMemo(
    () => cards.filter((c) => (c.title + c.subtitle).toLowerCase().includes(q.toLowerCase())),
    [cards, q],
  );

  return (
    <>
      <div className="owui-spread" style={{ margin: "12px 0" }}>
        <div style={{ flex: "1 1 320px", maxWidth: 360 }}>
          <MdSearchBar
            value={q}
            placeholder="Search"
            onInput={(e) => setQ((e.target as HTMLInputElement).value)}
          />
        </div>
        <MdFilledTonalButton>
          <MdIcon slot="icon">add</MdIcon>
          {createLabel}
        </MdFilledTonalButton>
      </div>

      <div className="owui-grid">
        {filtered.map((c) => (
          <MdOutlinedCard key={c.id} style={{ padding: 16 }}>
            <div className="owui-spread">
              <div className="owui-row">
                <MdIcon style={{ color: "var(--md-sys-color-primary)" }}>{c.icon}</MdIcon>
                <strong>{c.title}</strong>
              </div>
              <Menu
                trigger={({ toggle }) => (
                  <MdIconButton aria-label="Options" onClick={toggle}>
                    <MdIcon>more_vert</MdIcon>
                  </MdIconButton>
                )}
              >
                <MdMenuItem>
                  <MdIcon slot="start">edit</MdIcon>
                  <span slot="headline">Edit</span>
                </MdMenuItem>
                <MdMenuItem>
                  <MdIcon slot="start">content_copy</MdIcon>
                  <span slot="headline">Clone</span>
                </MdMenuItem>
                <MdMenuItem>
                  <MdIcon slot="start">delete</MdIcon>
                  <span slot="headline">Delete</span>
                </MdMenuItem>
              </Menu>
            </div>
            <p className="owui-muted" style={{ margin: "6px 0 10px", fontSize: 14 }}>
              {c.subtitle}
            </p>
            <div className="owui-row" style={{ flexWrap: "wrap" }}>
              {c.badge && <MdAssistChip label={c.badge} />}
              {c.tags?.map((t) => (
                <MdAssistChip key={t} label={t} />
              ))}
            </div>
          </MdOutlinedCard>
        ))}
      </div>
    </>
  );
}

export function WorkspaceRoute() {
  const [tab, setTab] = useState(0);
  const { data: models = [] } = useWorkspaceModels();
  const { data: knowledge = [] } = useKnowledge();
  const { data: prompts = [] } = usePrompts();
  const { data: tools = [] } = useTools();

  return (
    <div className="owui-page">
      <div className="owui-page__inner">
        <h1 style={{ marginTop: 0 }}>Workspace</h1>
        <MdTabs
          activeTabIndex={tab}
          onChange={(e) =>
            setTab((e.target as unknown as { activeTabIndex: number }).activeTabIndex)
          }
        >
          <MdPrimaryTab>
            <MdIcon slot="icon">deployed_code</MdIcon>Models
          </MdPrimaryTab>
          <MdPrimaryTab>
            <MdIcon slot="icon">menu_book</MdIcon>Knowledge
          </MdPrimaryTab>
          <MdPrimaryTab>
            <MdIcon slot="icon">terminal</MdIcon>Prompts
          </MdPrimaryTab>
          <MdPrimaryTab>
            <MdIcon slot="icon">build</MdIcon>Tools
          </MdPrimaryTab>
        </MdTabs>

        {tab === 0 && (
          <CardGrid
            createLabel="Create model"
            cards={models.map((m) => ({
              id: m.id,
              title: m.name,
              subtitle: m.description,
              icon: "deployed_code",
              tags: m.tags,
              badge: m.visibility,
            }))}
          />
        )}
        {tab === 1 && (
          <CardGrid
            createLabel="Create knowledge"
            cards={knowledge.map((k) => ({
              id: k.id,
              title: k.name,
              subtitle: k.description,
              icon: "menu_book",
              badge: `${k.file_count} files`,
            }))}
          />
        )}
        {tab === 2 && (
          <CardGrid
            createLabel="Create prompt"
            cards={prompts.map((p) => ({
              id: p.id,
              title: p.title,
              subtitle: p.content,
              icon: "terminal",
              tags: p.tags,
              badge: p.command,
            }))}
          />
        )}
        {tab === 3 && (
          <CardGrid
            createLabel="Create tool"
            cards={tools.map((t) => ({
              id: t.id,
              title: t.name,
              subtitle: t.description,
              icon: "build",
              badge: t.type,
            }))}
          />
        )}
      </div>
    </div>
  );
}
