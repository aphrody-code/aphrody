// Typed M3 panel for `aphrody forensics map` — walks a target directory, writes
// a classified map.json on disk, and renders the JSON stdout summary as stat
// tiles plus the path of the written map.json. Read-only: secret-bearing files
// are recorded by metadata only, never opened.

import { useMemo, useState } from "react";
import {
  MdFilledButton,
  MdIcon,
  MdOutlinedCard,
  MdOutlinedTextField,
} from "@aphrody/m3-react";
import { run } from "../../client.ts";

/**
 * Stdout summary of `aphrody forensics map --target <dir> --out <dir>`. The rich
 * per-file map is written to `<out>/map.json` ON DISK; stdout carries only this
 * machine-readable object (field names verbatim from `forensics_cmd.rs`).
 */
export interface MapSummary {
  wrote: string;
  file_count: number;
  hashed_count: number;
  secret_meta_only_count: number;
}

/** One stat tile rendered from the summary. */
interface FsTile {
  label: string;
  value: number;
  icon: string;
  /** Visual tone — "secret" tints the secret-skipped count amber. */
  tone: "default" | "ok" | "secret";
  hint: string;
}

const TONE_COLOR: Record<FsTile["tone"], string | undefined> = {
  default: "var(--md-sys-color-primary)",
  ok: "#34a853",
  secret: "#c9920a",
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function parseSummary(raw: unknown): MapSummary {
  const r = isRecord(raw) ? raw : {};
  return {
    wrote: typeof r.wrote === "string" ? r.wrote : "",
    file_count: typeof r.file_count === "number" ? r.file_count : 0,
    hashed_count: typeof r.hashed_count === "number" ? r.hashed_count : 0,
    secret_meta_only_count:
      typeof r.secret_meta_only_count === "number" ? r.secret_meta_only_count : 0,
  };
}

export function FsMapPanel() {
  const [target, setTarget] = useState("");
  const [out, setOut] = useState("");
  const [running, setRunning] = useState(false);
  const [report, setReport] = useState<MapSummary | null>(null);
  const [error, setError] = useState<string | null>(null);

  const tiles = useMemo<FsTile[]>(() => {
    if (!report) return [];
    return [
      {
        label: "Fichiers cartographiés",
        value: report.file_count,
        icon: "description",
        tone: "default",
        hint: "Total des fichiers réguliers parcourus.",
      },
      {
        label: "Fichiers hachés",
        value: report.hashed_count,
        icon: "fingerprint",
        tone: "ok",
        hint: "SHA-256 calculé (non sensibles, ≤ 1 Mio).",
      },
      {
        label: "Sensibles (métadonnées seules)",
        value: report.secret_meta_only_count,
        icon: "lock",
        tone: "secret",
        hint: "Chemins sur liste sensible — jamais ouverts.",
      },
    ];
  }, [report]);

  const analyze = async () => {
    const t = target.trim();
    const o = out.trim();
    if (!t || !o || running) return;
    setRunning(true);
    setError(null);
    setReport(null);
    try {
      const res = await run(["forensics", "map", "--target", t, "--out", o]);
      if (res.code !== 0) {
        setError(
          (res.stderr || res.stdout || `La commande a renvoyé le code ${res.code}.`)
            .trim()
            .slice(0, 2000),
        );
        return;
      }
      const stdout = res.stdout.trim();
      const start = stdout.indexOf("{");
      if (start < 0) {
        setError(
          (res.stderr || "La commande n'a renvoyé aucun objet JSON exploitable.")
            .trim()
            .slice(0, 2000),
        );
        return;
      }
      setReport(parseSummary(JSON.parse(stdout.slice(start))));
    } catch (err) {
      setError(`Sortie illisible (JSON invalide) : ${String(err)}`);
    } finally {
      setRunning(false);
    }
  };

  return (
    <div className="aph-fsmap">
      <MdOutlinedCard className="aph-runcard">
        <div className="aph-runcard__row">
          <MdIcon className="aph-runcard__glyph">account_tree</MdIcon>
          <div className="aph-runcard__text">
            <span className="aph-runcard__title">Carte du système de fichiers</span>
            <span className="aph-muted aph-runcard__hint">
              Parcours parallèle + classification (extension, taille, SHA-256). Les fichiers
              sensibles (cookies, jetons, leveldb…) sont recensés sans jamais être ouverts.
            </span>
          </div>
        </div>
        <div className="aph-runcard__fields">
          <MdOutlinedTextField
            className="aph-runcard__field"
            label="Répertoire à cartographier"
            placeholder="ex. /home/moi/.gemini"
            value={target}
            disabled={running}
            onInput={(e) => setTarget((e.target as HTMLInputElement).value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") void analyze();
            }}
          />
          <MdOutlinedTextField
            className="aph-runcard__field"
            label="Répertoire de sortie (reçoit map.json)"
            placeholder="ex. var/data/forensics"
            value={out}
            disabled={running}
            onInput={(e) => setOut((e.target as HTMLInputElement).value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") void analyze();
            }}
          />
        </div>
        <div className="aph-runcard__actions">
          <MdFilledButton
            disabled={running || !target.trim() || !out.trim()}
            onClick={() => void analyze()}
          >
            <MdIcon slot="icon">play_arrow</MdIcon>
            {running ? "Analyse…" : "Cartographier"}
          </MdFilledButton>
          <span className="aph-safety">
            <MdIcon>shield</MdIcon>
            Lecture seule — aucun contenu sensible n'est ouvert ni copié.
          </span>
        </div>
      </MdOutlinedCard>

      {error && (
        <MdOutlinedCard className="aph-errcard">
          <div className="aph-errcard__head">
            <MdIcon>error</MdIcon>
            <b>Échec de la cartographie</b>
          </div>
          <pre className="aph-errcard__body">{error}</pre>
        </MdOutlinedCard>
      )}

      {report && (
        <>
          <div className="aph-tiles">
            {tiles.map((t) => (
              <MdOutlinedCard key={t.label} className="aph-tile">
                <MdIcon className="aph-tile__icon" style={{ color: TONE_COLOR[t.tone] }}>
                  {t.icon}
                </MdIcon>
                <span className="aph-tile__value">{t.value.toLocaleString("fr-FR")}</span>
                <span className="aph-tile__label">{t.label}</span>
                <span className="aph-muted aph-tile__hint">{t.hint}</span>
              </MdOutlinedCard>
            ))}
          </div>

          <MdOutlinedCard className="aph-blockcard">
            <div className="aph-blockhead">
              <MdIcon>description</MdIcon>
              <span>Carte écrite</span>
            </div>
            <code className="aph-mono aph-wrotepath" title={report.wrote}>
              {report.wrote}
            </code>
            <p className="aph-muted aph-wrotenote">
              Le détail par fichier (chemin, taille, extension, SHA-256, horodatage) est dans ce{" "}
              <code className="aph-mono">map.json</code>. Ouvrez-le pour la vue exhaustive.
            </p>
          </MdOutlinedCard>
        </>
      )}
    </div>
  );
}
