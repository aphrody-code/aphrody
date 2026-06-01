// Settings page: Material tabs hosting Compte / Apparence / Conversation + the control-center tabs (Mémoire, Âme, Identité, Canaux, Actions, Agents) + À propos.

import { useEffect, useState } from "react";
import {
  MdFilledButton,
  MdIcon,
  MdOutlinedButton,
  MdPrimaryTab,
  MdTabs,
} from "@aphrody-code/m3-react";
import { useAccount, useMeta, useRun } from "../../client.ts";
import { CodeOutput, Hint, PageHead, Panel, StatTile } from "../../ui.tsx";
import { getState, setState, useUi } from "../../../store.ts";
import { ActionsTab } from "./tabs/ActionsTab.tsx";
import { AgentsTab } from "./tabs/AgentsTab.tsx";
import { ChannelsTab } from "./tabs/ChannelsTab.tsx";
import { MemoryTab } from "./tabs/MemoryTab.tsx";
import { PersonaTab } from "./tabs/PersonaTab.tsx";

/** Ordered tab ids, used to map the `?tab=` query param to a tab index. */
const TAB_IDS = [
  "compte",
  "apparence",
  "conversation",
  "memoire",
  "soul",
  "identity",
  "channels",
  "actions",
  "agents",
  "apropos",
] as const;
type TabId = (typeof TAB_IDS)[number];

const TAB_LABELS: Record<TabId, string> = {
  compte: "Compte",
  apparence: "Apparence",
  conversation: "Conversation",
  memoire: "Mémoire",
  soul: "Âme",
  identity: "Identité",
  channels: "Canaux",
  actions: "Actions",
  agents: "Agents",
  apropos: "À propos",
};

type ChatBackend = "agy" | "web" | "stub";

interface BackendOption {
  id: ChatBackend;
  label: string;
  desc: string;
  icon: string;
}

const BACKENDS: BackendOption[] = [
  {
    id: "agy",
    label: "Antigravity (recommandé)",
    desc: "Compte Google AI — Gemini 3.5 Flash",
    icon: "bolt",
  },
  { id: "web", label: "Gemini Web", desc: "Cookie Google signé, sans clé", icon: "language" },
  { id: "stub", label: "Hors-ligne", desc: "Réponse locale, aucun réseau", icon: "cloud_off" },
];

const BACKEND_STORAGE = "aphrody.backend";

export function Settings() {
  const [index, setIndex] = useState(0);
  const { data: account } = useAccount();
  const { data: meta } = useMeta();

  // Honour a `?tab=` deep-link (e.g. from the Dashboard quick actions).
  useEffect(() => {
    const tab = new URLSearchParams(window.location.search).get("tab") as TabId | null;
    if (tab) {
      const idx = TAB_IDS.indexOf(tab);
      if (idx >= 0) setIndex(idx);
    }
  }, []);

  function onTabChange(next: number): void {
    setIndex(next);
    // Reflect the tab in the URL so it is shareable / restorable.
    const url = new URL(window.location.href);
    url.searchParams.set("tab", TAB_IDS[next] ?? "compte");
    window.history.replaceState(null, "", url);
  }

  return (
    <div className="aph-section aph-settings">
      <PageHead title="Paramètres" subtitle="Compte, apparence et centre de contrôle de l'agent" />

      <MdTabs
        activeTabIndex={index}
        onChange={(e) =>
          onTabChange((e.target as unknown as { activeTabIndex: number }).activeTabIndex)
        }
      >
        {TAB_IDS.map((id) => (
          <MdPrimaryTab key={id}>{TAB_LABELS[id]}</MdPrimaryTab>
        ))}
      </MdTabs>

      <div className="aph-settings-body">
        {index === 0 && <CompteTab account={account} />}
        {index === 1 && <ApparenceTab />}
        {index === 2 && <ConversationTab />}
        {index === 3 && <MemoryTab />}
        {index === 4 && <PersonaTab which="soul" />}
        {index === 5 && <PersonaTab which="identity" />}
        {index === 6 && <ChannelsTab />}
        {index === 7 && <ActionsTab />}
        {index === 8 && <AgentsTab />}
        {index === 9 && <AproposTab meta={meta} />}
      </div>
    </div>
  );
}

interface AccountLike {
  connected: boolean;
  email: string;
  name: string;
  initials: string;
}

function CompteTab({ account }: { account: AccountLike | undefined }) {
  const runMutation = useRun();
  const [output, setOutput] = useState("");

  async function refresh(): Promise<void> {
    const res = await runMutation.mutateAsync(["antigravity", "whoami"]);
    setOutput(res.code === 0 ? res.stdout : res.stderr);
  }

  return (
    <div className="aph-settings-tab">
      <Panel title="Compte" icon="account_circle">
        {account?.connected ? (
          <>
            <div className="aph-row aph-settings-account">
              <div className="aph-settings-avatar">{account.initials}</div>
              <div className="aph-settings-who">
                <b>{account.name}</b>
                <span className="aph-muted">{account.email}</span>
              </div>
              <span className="aph-settings-badge aph-row">
                <MdIcon>check_circle</MdIcon>
                Connecté
              </span>
            </div>
            <p className="aph-muted">
              Compte Google lié à aphrody (token agy / cookie Google). Les conversations utilisent
              ce compte via Gemini.
            </p>
          </>
        ) : (
          <>
            <div className="aph-row aph-settings-account">
              <div className="aph-settings-avatar aph-settings-avatar--off">
                <MdIcon>person_off</MdIcon>
              </div>
              <div className="aph-settings-who">
                <b>Non connecté</b>
                <span className="aph-muted">Aucune session Google active.</span>
              </div>
            </div>
            <p className="aph-muted">
              Lancez l'authentification Antigravity (agy) en arrière-plan pour régénérer la session,
              puis actualisez.
            </p>
          </>
        )}
        <div className="aph-settings-row aph-settings-actions">
          <MdFilledButton onClick={() => void refresh()} disabled={runMutation.isPending}>
            <MdIcon slot="icon">{runMutation.isPending ? "progress_activity" : "refresh"}</MdIcon>
            {runMutation.isPending ? "Vérification…" : "Actualiser le compte"}
          </MdFilledButton>
        </div>
        {output && <CodeOutput text={output} empty="" />}
      </Panel>
    </div>
  );
}

function ApparenceTab() {
  const { themeMode } = useUi();

  function setTheme(mode: "dark" | "light"): void {
    if (getState().themeMode !== mode) setState({ themeMode: mode });
  }

  return (
    <div className="aph-settings-tab">
      <Panel title="Apparence" icon="palette">
        <p className="aph-muted">Thème</p>
        <div className="aph-settings-row">
          <MdFilledButton onClick={() => setTheme("dark")} disabled={themeMode === "dark"}>
            <MdIcon slot="icon">dark_mode</MdIcon>
            Sombre
          </MdFilledButton>
          <MdOutlinedButton onClick={() => setTheme("light")} disabled={themeMode === "light"}>
            <MdIcon slot="icon">light_mode</MdIcon>
            Clair
          </MdOutlinedButton>
        </div>
      </Panel>
    </div>
  );
}

function ConversationTab() {
  const [backend, setBackend] = useState<ChatBackend>("agy");
  const runMutation = useRun();

  useEffect(() => {
    const saved = localStorage.getItem(BACKEND_STORAGE) as ChatBackend | null;
    if (saved) setBackend(saved);
  }, []);

  function choose(id: ChatBackend): void {
    setBackend(id);
    localStorage.setItem(BACKEND_STORAGE, id);
    void runMutation.mutateAsync(["config", "set", "backend", id]);
  }

  return (
    <div className="aph-settings-tab">
      <Panel title="Conversation" icon="chat">
        <p className="aph-muted">Backend Gemini</p>
        <div className="aph-settings-options">
          {BACKENDS.map((o) => (
            <button
              key={o.id}
              type="button"
              className={`aph-settings-option${backend === o.id ? " aph-settings-option--sel" : ""}`}
              onClick={() => choose(o.id)}
            >
              <MdIcon className="aph-settings-option__icon">{o.icon}</MdIcon>
              <span className="aph-settings-option__text">
                <b>{o.label}</b>
                <span className="aph-muted">{o.desc}</span>
              </span>
              {backend === o.id && <MdIcon className="aph-settings-option__check">check</MdIcon>}
            </button>
          ))}
        </div>
      </Panel>
    </div>
  );
}

interface MetaLike {
  app_version: string;
  target_os: string;
  target_arch: string;
  family: string;
}

function AproposTab({ meta }: { meta: MetaLike | undefined }) {
  return (
    <div className="aph-settings-tab">
      <Panel title="À propos" icon="info">
        <div className="aph-row aph-settings-about">
          <span className="aph-settings-logo">
            <MdIcon>auto_awesome</MdIcon>
          </span>
          <div>
            <b>aphrody</b>
            <p className="aph-muted">Agent autonome — assistant IA propulsé par Gemini.</p>
          </div>
        </div>
        {meta ? (
          <div className="aph-grid">
            <StatTile icon="sell" label="Version" value={meta.app_version} />
            <StatTile icon="computer" label="Système" value={meta.target_os} />
            <StatTile icon="memory" label="Architecture" value={meta.target_arch} />
            <StatTile icon="category" label="Famille" value={meta.family} />
          </div>
        ) : (
          <Hint icon="info" title="Métadonnées indisponibles" />
        )}
        <p className="aph-muted aph-settings-note">
          React 19 + Material 3 (@aphrody-code/m3-react), propulsé en local par le binaire aphrody.
        </p>
      </Panel>
    </div>
  );
}
