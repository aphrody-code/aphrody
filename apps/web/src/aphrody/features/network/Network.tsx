// Network view host — typed DNS recon + web search panels via M3 tabs, plus a raw CLI output view.

import { useState } from "react";
import { MdIcon, MdPrimaryTab, MdTabs } from "@aphrody-code/m3-react";
import { PageHead } from "../../ui.tsx";
import { ToolRunner, type ToolAction } from "../../ToolRunner.tsx";
import { DnsReconPanel } from "./DnsReconPanel.tsx";
import { WebSearchPanel } from "./WebSearchPanel.tsx";

const RAW_ACTIONS: ToolAction[] = [
  {
    label: "Reconnaissance DNS",
    icon: "dns",
    args: ["dns"],
    prompt: { placeholder: "Domaine (ex. example.com)" },
    hint: "Résolution DNS OSINT agressive d'un domaine.",
  },
  {
    label: "Recherche web native",
    icon: "search",
    args: ["search"],
    prompt: { placeholder: "Requête de recherche" },
    hint: "Recherche web native sans clé ni navigateur (DuckDuckGo).",
  },
];

export function Network() {
  const [tab, setTab] = useState(0);
  const [rawOpen, setRawOpen] = useState(false);

  return (
    <div className="aph-net">
      <PageHead
        title="Réseau"
        subtitle="Reconnaissance DNS OSINT et recherche web native (aphrody dns / search)."
      />

      <MdTabs
        className="aph-net-tabs"
        activeTabIndex={tab}
        onChange={(e) => setTab((e.target as unknown as { activeTabIndex: number }).activeTabIndex)}
      >
        <MdPrimaryTab>
          <MdIcon slot="icon">dns</MdIcon>
          Reconnaissance DNS
        </MdPrimaryTab>
        <MdPrimaryTab>
          <MdIcon slot="icon">search</MdIcon>
          Recherche web
        </MdPrimaryTab>
      </MdTabs>

      <section className="aph-net-block">
        {tab === 0 ? <DnsReconPanel /> : <WebSearchPanel />}
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
            subtitle="Sortie texte non formatée des commandes dns / search."
            icon="dns"
            actions={RAW_ACTIONS}
          />
        )}
      </section>
    </div>
  );
}
