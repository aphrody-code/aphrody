// "/admin" — Users (real md-table: sort + filter + pagination + add-user dialog),
// Settings (connection toggles), Evaluations (leaderboard). Admin-panel surface.

import { useMemo, useState } from "react";
import {
  MdDialog,
  MdFilledButton,
  MdIcon,
  MdList,
  MdListItem,
  MdOutlinedSelect,
  MdOutlinedTextField,
  MdPrimaryTab,
  MdSelectOption,
  MdSwitch,
  MdTable,
  MdTabs,
  MdTextButton,
} from "@aphrody/m3-react";
import { useAdminUsers } from "../api/queries.ts";

const COLUMNS = [
  { key: "name", label: "Nom", sortable: true, filter: "text" as const },
  { key: "email", label: "Email", sortable: true, filter: "text" as const },
  { key: "role", label: "Rôle", sortable: true, filter: "text" as const },
  { key: "last_active", label: "Dernière activité", sortable: true },
];

function ago(ts: number): string {
  const m = Math.floor((Date.now() - ts) / 60_000);
  if (m < 1) return "à l'instant";
  if (m < 60) return `il y a ${m}m`;
  const h = Math.floor(m / 60);
  if (h < 24) return `il y a ${h}h`;
  return `il y a ${Math.floor(h / 24)}j`;
}

function UsersTab() {
  const { data: users = [] } = useAdminUsers();
  const [adding, setAdding] = useState(false);
  const rows = useMemo<Record<string, unknown>[]>(
    () =>
      users.map((u) => ({
        id: u.id,
        name: u.name,
        email: u.email,
        role: u.role,
        last_active: ago(u.last_active_at),
      })),
    [users],
  );

  return (
    <>
      <div className="owui-spread" style={{ margin: "12px 0" }}>
        <span className="owui-muted">{users.length} utilisateurs</span>
        <MdFilledButton onClick={() => setAdding(true)}>
          <MdIcon slot="icon">person_add</MdIcon>
          Ajouter un utilisateur
        </MdFilledButton>
      </div>

      <div className="owui-scrollx">
        <MdTable columns={COLUMNS} rows={rows} filterable paginated pageSize={10} />
      </div>

      <MdDialog open={adding} onClosed={() => setAdding(false)}>
        <div slot="headline">Ajouter un utilisateur</div>
        <form slot="content" method="dialog" className="owui-stack">
          <MdOutlinedTextField label="Nom" />
          <MdOutlinedTextField label="Email" type="email" />
          <MdOutlinedTextField label="Mot de passe" type="password" />
          <MdOutlinedSelect label="Rôle" value="user">
            <MdSelectOption value="user">
              <span slot="headline">Utilisateur</span>
            </MdSelectOption>
            <MdSelectOption value="admin">
              <span slot="headline">Administrateur</span>
            </MdSelectOption>
            <MdSelectOption value="pending">
              <span slot="headline">En attente</span>
            </MdSelectOption>
          </MdOutlinedSelect>
        </form>
        <div slot="actions">
          <MdTextButton onClick={() => setAdding(false)}>Annuler</MdTextButton>
          <MdFilledButton onClick={() => setAdding(false)}>Créer</MdFilledButton>
        </div>
      </MdDialog>
    </>
  );
}

function SettingsTab() {
  const items = [
    { icon: "lan", label: "API Ollama", value: "http://localhost:11434" },
    { icon: "key", label: "API OpenAI", value: "configurée" },
    { icon: "travel_explore", label: "Recherche Web", value: "activée" },
    { icon: "image", label: "Génération d'images", value: "activée" },
  ];
  return (
    <div style={{ paddingTop: 8 }}>
      {items.map((it) => (
        <div
          key={it.label}
          className="owui-spread"
          style={{
            padding: "12px 0",
            borderBottom: "1px solid var(--md-sys-color-outline-variant)",
          }}
        >
          <span className="owui-row">
            <MdIcon>{it.icon}</MdIcon>
            {it.label}
          </span>
          <span className="owui-row">
            <span className="owui-muted" style={{ fontSize: 13 }}>
              {it.value}
            </span>
            <MdSwitch selected />
          </span>
        </div>
      ))}
    </div>
  );
}

function EvaluationsTab() {
  const board = [
    { model: "GPT-4o", score: 1284 },
    { model: "Llama 3.2", score: 1187 },
    { model: "Mistral Nemo", score: 1102 },
    { model: "Qwen2.5 Coder", score: 1064 },
  ];
  return (
    <MdList>
      {board.map((b, i) => (
        <MdListItem key={b.model}>
          <span slot="start" style={{ width: 28, textAlign: "center", fontWeight: 700 }}>
            {i + 1}
          </span>
          <span slot="headline">{b.model}</span>
          <span slot="end" className="owui-muted">
            {b.score} ELO
          </span>
        </MdListItem>
      ))}
    </MdList>
  );
}

export function AdminRoute() {
  const [tab, setTab] = useState(0);
  return (
    <div className="owui-page">
      <div className="owui-page__inner">
        <h1 style={{ marginTop: 0 }}>Administration</h1>
        <MdTabs
          activeTabIndex={tab}
          onChange={(e) =>
            setTab((e.target as unknown as { activeTabIndex: number }).activeTabIndex)
          }
        >
          <MdPrimaryTab>
            <MdIcon slot="icon">group</MdIcon>Utilisateurs
          </MdPrimaryTab>
          <MdPrimaryTab>
            <MdIcon slot="icon">settings</MdIcon>Paramètres
          </MdPrimaryTab>
          <MdPrimaryTab>
            <MdIcon slot="icon">leaderboard</MdIcon>Évaluations
          </MdPrimaryTab>
        </MdTabs>
        {tab === 0 && <UsersTab />}
        {tab === 1 && <SettingsTab />}
        {tab === 2 && <EvaluationsTab />}
      </div>
    </div>
  );
}
