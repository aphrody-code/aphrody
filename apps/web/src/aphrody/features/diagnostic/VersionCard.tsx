// Version + système : carte M3 croisant les méta de l'hôte (useMeta) avec la sortie de `aphrody version`.

import { useState } from "react";
import { MdCircularProgress, MdIcon, MdIconButton, MdOutlinedCard } from "@aphrody/m3-react";
import { execText, useMeta, useRun } from "../../client.ts";

/** Une ligne clé/valeur affichée dans la carte (valeur monospace optionnelle). */
interface KvRow {
  label: string;
  value: string;
  mono?: boolean;
}

export function VersionCard() {
  const meta = useMeta();
  const run = useRun();
  const [text, setText] = useState("");
  const [code, setCode] = useState<number | null>(null);

  const load = () => {
    run.mutate(["version"], {
      onSuccess: (res) => {
        setCode(res.code);
        setText(execText(res));
      },
    });
  };

  // Charge la version au premier rendu (équivalent du ngOnInit).
  if (code === null && !run.isPending && run.isIdle) load();

  const m = meta.data;
  const rows: KvRow[] = m
    ? [
        { label: "Système hôte", value: m.target_os },
        { label: "Architecture", value: m.target_arch, mono: true },
        { label: "Famille", value: m.family },
        { label: "Version de l'app", value: m.app_version, mono: true },
      ]
    : [];

  return (
    <MdOutlinedCard className="aph-panel aph-vcard">
      <header className="aph-panel__head">
        <span className="aph-row">
          <MdIcon className="aph-panel__icon">info</MdIcon>
          <h2 className="aph-panel__title">Version et système</h2>
        </span>
        <span className="aph-row">
          {run.isPending && <MdCircularProgress indeterminate aria-label="Chargement" />}
          <MdIconButton aria-label="Rafraîchir la version" disabled={run.isPending} onClick={load}>
            <MdIcon>refresh</MdIcon>
          </MdIconButton>
        </span>
      </header>

      <div className="aph-panel__body">
        <div className="aph-vcard__banner">
          <div className="aph-vcard__banner-text">
            <span className="aph-vcard__banner-label">Version</span>
            <b className="aph-vcard__banner-num">{m?.app_version ?? "—"}</b>
          </div>
          {m && <span className="aph-vcard__banner-chip">{m.family}</span>}
        </div>

        {rows.length > 0 && (
          <div className="aph-kv aph-vcard__kv">
            {rows.map((row) => (
              <div className="aph-vcard__row" key={row.label}>
                <span className="aph-kv__k">{row.label}</span>
                <span className={row.mono ? "aph-vcard__mono" : undefined}>{row.value}</span>
              </div>
            ))}
          </div>
        )}

        <div className="aph-vcard__out">
          <div className="aph-vcard__out-head">
            <MdIcon>terminal</MdIcon>
            <span className="aph-output__cmd">aphrody version</span>
            {code !== null && (
              <span className={`aph-code ${code === 0 ? "aph-code--ok" : "aph-code--err"}`}>
                code {code}
              </span>
            )}
          </div>
          <pre className="aph-vcard__out-body">
            {run.isPending ? "Exécution en cours…" : text || "La version s'affichera ici."}
          </pre>
        </div>
      </div>
    </MdOutlinedCard>
  );
}
