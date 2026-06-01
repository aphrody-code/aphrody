// Vue diagnostic : bandeau de méta hôte, tableau de bord doctor, carte version, et sortie texte brute repliable (ToolRunner).

import { useState } from "react";
import { MdIcon } from "@aphrody-code/m3-react";
import { useMeta } from "../../client.ts";
import { ToolRunner, type ToolAction } from "../../ToolRunner.tsx";
import { DoctorDashboard } from "./DoctorDashboard.tsx";
import { VersionCard } from "./VersionCard.tsx";

const RAW_ACTIONS: ToolAction[] = [
  {
    label: "Diagnostic complet (doctor)",
    icon: "monitor_heart",
    args: ["doctor"],
    hint: "Environnement + A2A + supply-chain (sortie texte).",
  },
  {
    label: "Diagnostic JSON",
    icon: "data_object",
    args: ["doctor", "--json"],
    hint: "Même diagnostic, au format JSON structuré.",
  },
  {
    label: "Version et état",
    icon: "info",
    args: ["version"],
    hint: "Version du binaire et état du système.",
  },
  {
    label: "Version JSON",
    icon: "data_object",
    args: ["version", "--json"],
    hint: "Objet JSON unique avec la version.",
  },
];

export function Diagnostic() {
  const meta = useMeta();
  const [showRaw, setShowRaw] = useState(false);
  const m = meta.data;

  return (
    <div className="aph-diag">
      <div className="aph-diag__meta">
        {m && (
          <>
            <span className="aph-diag__pill">app {m.app_version}</span>
            <span className="aph-diag__pill">{m.target_os}</span>
            <span className="aph-diag__pill">{m.target_arch}</span>
            <span className="aph-diag__pill">{m.family}</span>
            <span className="aph-diag__pill aph-diag__pill--warn">mode navigateur</span>
          </>
        )}
      </div>

      <DoctorDashboard />

      <div className="aph-diag__section">
        <VersionCard />
      </div>

      <div className="aph-diag__section aph-diag__raw">
        <button
          className="aph-diag__toggle"
          aria-expanded={showRaw}
          onClick={() => setShowRaw((v) => !v)}
        >
          <span>Sortie texte brute (doctor / version)</span>
          <MdIcon className={`aph-diag__chevron${showRaw ? " is-open" : ""}`}>expand_more</MdIcon>
        </button>
        {showRaw && (
          <ToolRunner
            title="Sortie texte"
            subtitle="Diagnostic et version au format texte / JSON, tels que produits par le binaire aphrody."
            icon="terminal"
            actions={RAW_ACTIONS}
          />
        )}
      </div>
    </div>
  );
}
