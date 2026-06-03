// Network view host — web search panel, plus a raw CLI output view.

import { useState } from "react";
import { MdIcon } from "@aphrody/m3-react";
import { PageHead } from "../../ui.tsx";
import { ToolRunner, type ToolAction } from "../../ToolRunner.tsx";
import { WebSearchPanel } from "./WebSearchPanel.tsx";

const RAW_ACTIONS: ToolAction[] = [
  {
    label: "Recherche web native",
    icon: "search",
    args: ["search"],
    prompt: { placeholder: "Requête de recherche" },
    hint: "Recherche web native sans clé ni navigateur (DuckDuckGo).",
  },
];

export function Network() {
  const [rawOpen, setRawOpen] = useState(false);

  return (
    <div className="aph-net">
      <PageHead
        title="Réseau"
        subtitle="Recherche web native (aphrody search)."
      />

      <section className="aph-net-block">
        <WebSearchPanel />
      </section>

      <section className="aph-net-raw">
        <button
          className="aph-net-raw__toggle"
          onClick={() => setRawOpen((o) => !o)}
          aria-expanded={rawOpen}
        >
          <MdIcon>terminal</MdIcon>
          <span>Vue brute (sortie CLI)</span>
          <MdIcon className="aph-net-raw__chevron">
            {rawOpen ? "expand_less" : "expand_more"}
          </MdIcon>
        </button>
        {rawOpen && (
          <ToolRunner
            title="Réseau — sortie brute"
            subtitle="Sortie texte non formatée de la commande search."
            icon="search"
            actions={RAW_ACTIONS}
          />
        )}
      </section>
    </div>
  );
}
