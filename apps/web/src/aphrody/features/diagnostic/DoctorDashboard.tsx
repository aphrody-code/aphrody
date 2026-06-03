// Tableau de bord diagnostic : exécute `aphrody doctor` et transforme chaque ligne de vérification en ligne d'état (ok/warn/error).

import { useMemo, useState } from "react";
import { MdCircularProgress, MdIcon, MdIconButton, MdOutlinedCard } from "@aphrody/m3-react";
import { execText, useRun } from "../../client.ts";

/** Tonalité d'un état, qui pilote la couleur de la puce et de l'icône. */
type Health = "ok" | "warn" | "err";

/** Une vérification analysée depuis la sortie texte de `aphrody doctor`. */
interface Check {
  health: Health;
  /** Nom de la vérification (premier mot après le marqueur de statut). */
  name: string;
  /** Détail restant sur la ligne. */
  detail: string;
}

/** Décompte de fin de sortie (ligne « 5 ok · 1 warning · 0 errors »). */
interface Summary {
  ok: number;
  warn: number;
  err: number;
}

/** Résultat complet d'une analyse de la sortie `doctor`. */
interface Parsed {
  checks: Check[];
  summary: Summary | null;
}

const META: Record<Health, { icon: string; label: string; color: string }> = {
  ok: { icon: "check_circle", label: "OK", color: "var(--md-sys-color-primary)" },
  warn: { icon: "warning", label: "Avertissement", color: "var(--md-sys-color-tertiary)" },
  err: { icon: "error", label: "Erreur", color: "var(--md-sys-color-error)" },
};

/** Associe un marqueur `[xxx]` à une tonalité d'état. */
function tagToHealth(tag: string): Health | null {
  const t = tag.toLowerCase();
  if (t === "ok" || t === "pass") return "ok";
  if (t === "warn" || t === "warning") return "warn";
  if (t === "err" || t === "error" || t === "fail") return "err";
  return null;
}

/**
 * Analyse la sortie texte de `aphrody doctor`. Chaque ligne de la forme
 * `  [ok]   nom   détail` devient une vérification ; la ligne de synthèse
 * `N ok · N warning · N errors` alimente le décompte. Robuste aux espacements.
 */
function parseDoctor(text: string): Parsed {
  const checks: Check[] = [];
  let summary: Summary | null = null;

  for (const raw of text.split("\n")) {
    const line = raw.trim();
    if (!line) continue;

    const tag = line.match(/^\[(\w+)\]\s+(.*)$/);
    if (tag) {
      const health = tagToHealth(tag[1]);
      if (health) {
        const rest = tag[2].trim();
        const split = rest.match(/^(\S+)\s+(.*)$/);
        checks.push({
          health,
          name: split ? split[1] : rest,
          detail: split ? split[2].trim() : "",
        });
        continue;
      }
    }

    const sum = line.match(/(\d+)\s*ok\b.*?(\d+)\s*warning.*?(\d+)\s*error/i);
    if (sum) {
      summary = { ok: Number(sum[1]), warn: Number(sum[2]), err: Number(sum[3]) };
    }
  }

  return { checks, summary };
}

/** Verdict global dérivé des vérifications : err l'emporte sur warn sur ok. */
function overallHealth(checks: Check[]): Health {
  if (checks.some((c) => c.health === "err")) return "err";
  if (checks.some((c) => c.health === "warn")) return "warn";
  return "ok";
}

function verdictLabel(h: Health): string {
  if (h === "ok") return "Système sain";
  if (h === "warn") return "Système dégradé";
  return "Système en défaut";
}

function verdictHint(h: Health): string {
  if (h === "ok") return "Toutes les vérifications critiques passent.";
  if (h === "warn") return "Au moins une vérification non critique signale un avertissement.";
  return "Une ou plusieurs vérifications critiques ont échoué.";
}

export function DoctorDashboard() {
  const run = useRun();
  const [text, setText] = useState("");
  const [code, setCode] = useState<number | null>(null);

  const load = () => {
    run.mutate(["doctor"], {
      onSuccess: (res) => {
        setCode(res.code);
        setText(execText(res));
      },
    });
  };

  // Lance le diagnostic au premier rendu (équivalent du ngOnInit).
  if (code === null && !run.isPending && run.isIdle) load();

  const parsed = useMemo(() => parseDoctor(text), [text]);
  const health = overallHealth(parsed.checks);
  const hasChecks = parsed.checks.length > 0;

  return (
    <div className="aph-doctor">
      <header className="aph-tool__head">
        <span className="aph-tool__glyph">
          <MdIcon>monitor_heart</MdIcon>
        </span>
        <div>
          <h1 className="aph-pagehead__title">Diagnostic</h1>
          <p className="aph-muted">
            État du système, supply-chain et intégration A2A (aphrody doctor).
          </p>
        </div>
        <span className="aph-doctor__refresh">
          <MdIconButton
            aria-label="Rafraîchir le diagnostic"
            disabled={run.isPending}
            onClick={load}
          >
            <MdIcon>refresh</MdIcon>
          </MdIconButton>
        </span>
      </header>

      {run.isPending ? (
        <div className="aph-doctor__state">
          <MdCircularProgress indeterminate aria-label="Chargement" />
          <span>Exécution de aphrody doctor…</span>
        </div>
      ) : !hasChecks ? (
        <div className="aph-doctor__state aph-doctor__state--err">
          <MdIcon style={{ color: "var(--md-sys-color-error)" }}>error</MdIcon>
          <div>
            <b>Diagnostic illisible.</b>
            <p className="aph-muted">
              La commande doctor n'a renvoyé aucune ligne de vérification exploitable.
            </p>
            {code !== null && <p className="aph-muted">Code de sortie : {code}</p>}
            {text && <pre className="aph-doctor__raw">{text}</pre>}
          </div>
        </div>
      ) : (
        <>
          <MdOutlinedCard
            className="aph-doctor__verdict"
            style={{ borderInlineStartColor: META[health].color } as React.CSSProperties}
          >
            <MdIcon className="aph-doctor__verdict-glyph" style={{ color: META[health].color }}>
              {META[health].icon}
            </MdIcon>
            <div className="aph-doctor__verdict-text">
              <b>{verdictLabel(health)}</b>
              <span className="aph-muted">{verdictHint(health)}</span>
            </div>
            {parsed.summary && (
              <span className="aph-doctor__counts">
                <span className="aph-doctor__count aph-doctor__count--ok">
                  {parsed.summary.ok} ok
                </span>
                <span className="aph-doctor__count aph-doctor__count--warn">
                  {parsed.summary.warn} avert.
                </span>
                <span className="aph-doctor__count aph-doctor__count--err">
                  {parsed.summary.err} err.
                </span>
              </span>
            )}
          </MdOutlinedCard>

          <MdOutlinedCard className="aph-doctor__checks">
            {parsed.checks.map((c, i) => (
              <div className="aph-doctor__row" key={`${c.name}-${i}`}>
                <MdIcon
                  className="aph-doctor__row-icon"
                  style={{ color: META[c.health].color }}
                  aria-label={META[c.health].label}
                >
                  {META[c.health].icon}
                </MdIcon>
                <span className="aph-doctor__row-name">{c.name}</span>
                <span className="aph-doctor__row-detail">{c.detail}</span>
                <span
                  className="aph-doctor__chip"
                  style={
                    {
                      color: META[c.health].color,
                      borderColor: META[c.health].color,
                    } as React.CSSProperties
                  }
                >
                  {META[c.health].label}
                </span>
              </div>
            ))}
          </MdOutlinedCard>
        </>
      )}
    </div>
  );
}
