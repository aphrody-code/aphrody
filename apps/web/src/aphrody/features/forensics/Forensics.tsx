// Forensics view — reproducible, read-only forensic extraction (aphrody
// forensics + aphrody chromium). A segmented selector switches between the typed
// map / sqlite panels and the generic Chromium artefact ToolRunner.

import { useState } from "react";
import { MdIcon } from "@aphrody-code/m3-react";
import { PageHead } from "../../ui.tsx";
import { ToolRunner, type ToolAction } from "../../ToolRunner.tsx";
import { FsMapPanel } from "./FsMapPanel.tsx";
import { SqliteSchemaPanel } from "./SqliteSchemaPanel.tsx";

/** The forensics sub-views the operator can switch between. */
type ForensicsTab = "map" | "sqlite" | "chromium";

interface TabDef {
  id: ForensicsTab;
  label: string;
  icon: string;
}

const TABS: TabDef[] = [
  { id: "map", label: "Carte du système de fichiers", icon: "account_tree" },
  { id: "sqlite", label: "Schéma SQLite", icon: "database" },
  { id: "chromium", label: "Artefacts Chromium", icon: "cookie" },
];

/**
 * Chromium artefact commands. These produce free-text output (no documented JSON
 * contract), so they stay on the generic tool-runner rather than a typed panel.
 */
const CHROMIUM_ACTIONS: ToolAction[] = [
  {
    label: "Export de session Chromium",
    icon: "cookie",
    args: ["chromium", "export-session"],
    hint: "Exporte l'état de session Chromium de manière reproductible.",
  },
  {
    label: "Synchronisation Chromium",
    icon: "sync",
    args: ["chromium", "sync"],
    hint: "Synchronisation forensique des artefacts Chromium.",
  },
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
        {tab === "chromium" && (
          <ToolRunner
            title="Artefacts Chromium"
            subtitle="Export / synchronisation forensique des artefacts Chromium (sortie texte brute)"
            icon="cookie"
            actions={CHROMIUM_ACTIONS}
          />
        )}
      </section>
    </div>
  );
}
