// Mémoire tab: surface the agent's local memory store + the discovered memory providers (read-only, honest).

import { MdIcon } from "@aphrody/m3-react";
import { execText, useExec } from "../../../client.ts";
import { CodeOutput, Hint, Panel, Spinner } from "../../../ui.tsx";

interface Provider {
  name: string;
  desc: string;
}

const PROVIDERS: Provider[] = [
  { name: "SqliteLocal", desc: "Base locale hors-ligne (par défaut, ci-dessus)." },
  { name: "Mem0", desc: "Service HTTP — clé MEM0_API_KEY." },
  { name: "Honcho", desc: "Service HTTP (dialectic v3) — clé HONCHO_API_KEY." },
];

export function MemoryTab() {
  const { data: statRes, isLoading: loadingStat } = useExec(["memory", "stat"], ["memory", "stat"]);
  const { data: helpRes } = useExec(["memory", "help"], ["memory", "--help"]);

  const statText = execText(statRes);
  const helpText = execText(helpRes).slice(0, 1200);

  return (
    <div className="aph-settings-tab">
      <Panel title="Magasin de mémoire local" icon="database">
        <p className="aph-muted">
          L'agent conserve sa mémoire à long terme dans une base SQLite locale (
          <code>~/.aphrody/aphrody-memory.sqlite</code>). Elle est lue à chaque conversation et
          alimente le contexte.
        </p>
        {loadingStat ? (
          <Spinner label="Lecture de l'état…" />
        ) : statText ? (
          <CodeOutput text={statText} empty="État du magasin indisponible." />
        ) : (
          <Hint
            icon="info"
            title="État indisponible"
            text="La taille et la date de modification du magasin nécessitent le binaire aphrody local."
          />
        )}
      </Panel>

      <Panel title="Fournisseurs de mémoire" icon="dns">
        <p className="aph-muted">
          Le trait <code>MemoryProvider</code> permet de copier des enregistrements entre
          fournisseurs (<code>aphrody memory migrate</code>). Les fournisseurs HTTP lisent leurs
          identifiants dans l'environnement.
        </p>
        <ul className="aph-providers">
          {PROVIDERS.map((p) => (
            <li key={p.name} className="aph-provider">
              <div className="aph-row">
                <MdIcon className="aph-provider__icon">storage</MdIcon>
                <b>{p.name}</b>
              </div>
              <span className="aph-muted">{p.desc}</span>
            </li>
          ))}
        </ul>
        {helpText && <CodeOutput text={helpText} empty="" />}
      </Panel>
    </div>
  );
}
