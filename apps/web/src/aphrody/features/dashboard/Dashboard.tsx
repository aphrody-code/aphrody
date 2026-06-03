// Accueil dashboard (React port of Angular DashboardComponent): account banner, real-state stat tiles (doctor/version/meta/mcp) + quick actions.

import { useNavigate } from "@tanstack/react-router";
import { MdFilledTonalButton, MdIcon } from "@aphrody/m3-react";
import { execJson, execText, useAccount, useExec, useMeta } from "../../client.ts";
import { PageHead, Panel, StatTile } from "../../ui.tsx";

interface QuickAction {
  label: string;
  icon: string;
  seg: string;
}

const QUICK_ACTIONS: QuickAction[] = [
  { label: "Ouvrir l'assistant", icon: "chat_bubble", seg: "assistant" },
  { label: "Voir les outils MCP", icon: "hub", seg: "mcp" },
  { label: "Parcourir les skills", icon: "extension", seg: "skills" },
  { label: "Diagnostic", icon: "stethoscope", seg: "diagnostic" },
  { label: "Paramètres", icon: "settings", seg: "settings" },
];

const OK_TONE = "#34a853";
const WARN_TONE = "#fbbc04";

/** Best-effort count of MCP tools from the `mcp list --json` payload. */
function mcpToolCount(text: string): number | null {
  const j = execJson<{ tools?: unknown[] }>({ code: 0, stdout: text, stderr: "" });
  return Array.isArray(j?.tools) ? j.tools.length : null;
}

export function Dashboard() {
  const navigate = useNavigate();
  const go = (seg: string) => void navigate({ to: "/a/$section", params: { section: seg } });

  const { data: account } = useAccount();
  const { data: meta } = useMeta();

  const doctor = useExec(["doctor"], ["doctor"]);
  const version = useExec(["version"], ["version"]);
  const mcp = useExec(["mcp", "list"], ["mcp", "list", "--json"]);

  const doctorText = execText(doctor.data).trim();
  const doctorOk = doctor.data?.code === 0;
  const versionText = execText(version.data).trim() || meta?.app_version || "—";
  const toolCount = mcp.data ? mcpToolCount(mcp.data.stdout) : null;

  const refreshAll = () => {
    void doctor.refetch();
    void version.refetch();
    void mcp.refetch();
  };

  const anyLoading = doctor.isLoading || version.isLoading || mcp.isLoading;

  return (
    <div className="aph-dashboard">
      <PageHead
        title="Accueil"
        subtitle="Centre de contrôle — état réel des agents, du compte et des extensions."
        actions={
          <MdFilledTonalButton onClick={refreshAll} disabled={anyLoading}>
            <MdIcon slot="icon">refresh</MdIcon>
            Actualiser
          </MdFilledTonalButton>
        }
      />

      {/* Account banner */}
      <Panel title="Compte" icon="account_circle">
        {account?.connected ? (
          <div className="aph-banner">
            <div className="aph-banner__avatar">{account.initials}</div>
            <div className="aph-banner__text">
              <b>{account.name}</b>
              <span>{account.email}</span>
            </div>
            <span className="aph-badge aph-badge--ok">
              <MdIcon>check_circle</MdIcon>
              Connecté
            </span>
          </div>
        ) : (
          <div className="aph-banner">
            <div className="aph-banner__avatar aph-banner__avatar--off">
              <MdIcon>person_off</MdIcon>
            </div>
            <div className="aph-banner__text">
              <b>Non connecté</b>
              <span>Aucune session Google active.</span>
            </div>
          </div>
        )}
      </Panel>

      {/* Stat tiles — real resolved state */}
      <h2 className="aph-section-title">État du système</h2>
      <div className="aph-stat-grid">
        <StatTile
          icon="stethoscope"
          label="Diagnostic"
          value={doctor.isLoading ? "Détection…" : doctorOk ? "Sain" : "Attention"}
          tone={doctor.isLoading ? undefined : doctorOk ? OK_TONE : WARN_TONE}
        />
        <StatTile
          icon="info"
          label="Version aphrody"
          value={version.isLoading ? "Détection…" : versionText}
        />
        <StatTile
          icon="hub"
          label="Outils MCP"
          value={
            mcp.isLoading
              ? "Détection…"
              : toolCount !== null
                ? `${toolCount} exposé(s)`
                : "Injoignable"
          }
          tone={mcp.isLoading ? undefined : toolCount !== null ? OK_TONE : WARN_TONE}
        />
        <StatTile
          icon="computer"
          label="Plateforme"
          value={meta ? `${meta.target_os} · ${meta.target_arch}` : "—"}
        />
      </div>

      {/* Doctor detail */}
      <Panel title="Rapport de diagnostic" icon="monitor_heart">
        <pre className="aph-dashboard__doctor">
          {doctor.isLoading
            ? "Exécution de `aphrody doctor`…"
            : doctorText || "Aucune sortie de diagnostic."}
        </pre>
      </Panel>

      {/* Quick actions */}
      <h2 className="aph-section-title">Actions rapides</h2>
      <div className="aph-quick-grid">
        {QUICK_ACTIONS.map((a) => (
          <MdFilledTonalButton key={a.seg} className="aph-quick-action" onClick={() => go(a.seg)}>
            <MdIcon slot="icon">{a.icon}</MdIcon>
            {a.label}
          </MdFilledTonalButton>
        ))}
      </div>
    </div>
  );
}
