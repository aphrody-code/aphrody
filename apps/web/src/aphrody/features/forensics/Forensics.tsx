// Forensics view — reproducible, read-only forensic extraction (aphrody
// forensics). A segmented selector switches between the typed
// map / sqlite panels.

import { useState } from "react";
import { MdIcon } from "@aphrody/m3-react";
import { PageHead } from "../../ui.tsx";
import { FsMapPanel } from "./FsMapPanel.tsx";
import { SqliteSchemaPanel } from "./SqliteSchemaPanel.tsx";

/** The forensics sub-views the operator can switch between. */
type ForensicsTab = "map" | "sqlite";

interface TabDef {
  id: ForensicsTab;
  label: string;
  icon: string;
}

const TABS: TabDef[] = [
  { id: "map", label: "Carte du système de fichiers", icon: "account_tree" },
  { id: "sqlite", label: "Schéma SQLite", icon: "database" },
];

export function Forensics() {
  const [tab, setTab] = useState<ForensicsTab>("map");

  return (
    <div className="aph-section aph-forensics">
      <PageHead
        title="Forensics"
        subtitle="Extraction forensique reproductible (lecture seule, jamais de secrets en clair)."
      />

      <div className="aph-seg" role="tablist" aria-label="Sous-commande forensics">
        {TABS.map((t) => (
          <button
            key={t.id}
            className={`aph-seg__btn${tab === t.id ? " is-active" : ""}`}
            role="tab"
            type="button"
            aria-selected={tab === t.id}
            onClick={() => setTab(t.id)}
          >
            <MdIcon>{t.icon}</MdIcon>
            <span>{t.label}</span>
          </button>
        ))}
      </div>

      <section className="aph-forensics__panel">
        {tab === "map" && <FsMapPanel />}
        {tab === "sqlite" && <SqliteSchemaPanel />}
      </section>
    </div>
  );
}
