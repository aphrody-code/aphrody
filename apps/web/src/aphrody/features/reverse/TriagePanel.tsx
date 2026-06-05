// Typed M3 panel for `aphrody re triage` — runs the command, parses the JSON
// TriageReport, and renders a header card, sections table with entropy bars,
// import/export chips and a collapsible strings sample.

import { useMemo, useState } from "react";
import {
  MdAssistChip,
  MdChipSet,
  MdFilledButton,
  MdIcon,
  MdLinearProgress,
  MdOutlinedCard,
  MdOutlinedTextField,
} from "@aphrody/m3-react";
import { run } from "../../client.ts";

/**
 * Detected binary format — mirrors the Rust `aphrody_re::Format` enum
 * (`#[serde(rename_all = "lowercase")]`). `unknown` is always populated when no
 * magic matched (never null).
 */
export type ReFormat = "pe32" | "pe64" | "elf32" | "elf64" | "unknown";

/** One section row. `entropy` is Shannon entropy in bits/byte; null for sections with no on-disk content. */
export interface ReSection {
  name: string;
  vaddr: number;
  size: number;
  entropy: number | null;
}

/** Full triage report — matches the Rust `aphrody_re::TriageReport` DTO. */
export interface TriageReport {
  format: ReFormat;
  size: number;
  sha256: string;
  arch: string | null;
  entry_point: number | null;
  sections: ReSection[];
  imports: string[];
  exports: string[];
  strings_sample: string[];
}

/** Entropy threshold (bits/byte) above which a section is flagged packed/encrypted. */
const ENTROPY_HIGH = 7.2;
/** Entropy threshold (bits/byte) above which a section is merely "dense". */
const ENTROPY_MED = 6.0;

type FormatBucket = "pe" | "elf" | "unknown";
type EntropyLevel = "high" | "med" | "low";

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

/** Defensive parse of the triage JSON (the Rust DTO guarantees arrays, but coerce so the view is safe). */
function parseReport(raw: unknown): TriageReport {
  const r = isRecord(raw) ? raw : {};
  const sections = Array.isArray(r.sections) ? (r.sections as unknown[]) : [];
  return {
    format: (typeof r.format === "string" ? r.format : "unknown") as ReFormat,
    size: typeof r.size === "number" ? r.size : 0,
    sha256: typeof r.sha256 === "string" ? r.sha256 : "",
    arch: typeof r.arch === "string" ? r.arch : null,
    entry_point: typeof r.entry_point === "number" ? r.entry_point : null,
    sections: sections.map((s): ReSection => {
      const sr = isRecord(s) ? s : {};
      return {
        name: typeof sr.name === "string" ? sr.name : "",
        vaddr: typeof sr.vaddr === "number" ? sr.vaddr : 0,
        size: typeof sr.size === "number" ? sr.size : 0,
        entropy: typeof sr.entropy === "number" ? sr.entropy : null,
      };
    }),
    imports: Array.isArray(r.imports) ? r.imports.map(String) : [],
    exports: Array.isArray(r.exports) ? r.exports.map(String) : [],
    strings_sample: Array.isArray(r.strings_sample) ? r.strings_sample.map(String) : [],
  };
}

function formatBucket(format: ReFormat): FormatBucket {
  if (format === "pe32" || format === "pe64") return "pe";
  if (format === "elf32" || format === "elf64") return "elf";
  return "unknown";
}

function formatLabel(format: ReFormat): string {
  switch (format) {
    case "pe32":
      return "PE32 (Windows)";
    case "pe64":
      return "PE32+ (Windows 64-bit)";
    case "elf32":
      return "ELF32 (Linux/Unix)";
    case "elf64":
      return "ELF64 (Linux/Unix)";
    default:
      return "Format inconnu";
  }
}

function formatIcon(bucket: FormatBucket): string {
  switch (bucket) {
    case "pe":
      return "window";
    case "elf":
      return "terminal";
    default:
      return "help";
  }
}

/** Format a byte count as o / Ko / Mo / Go / To (French units). */
function humanSize(bytes: number): string {
  if (bytes < 1024) return `${bytes} o`;
  const units = ["Ko", "Mo", "Go", "To"];
  let value = bytes / 1024;
  let i = 0;
  while (value >= 1024 && i < units.length - 1) {
    value /= 1024;
    i++;
  }
  return `${value.toFixed(value >= 10 || Number.isInteger(value) ? 0 : 1)} ${units[i]}`;
}

/** Render a 64-bit virtual address as 0x-prefixed hex. */
function hex(value: number): string {
  return `0x${value.toString(16)}`;
}

function entropyLevel(entropy: number): EntropyLevel {
  if (entropy >= ENTROPY_HIGH) return "high";
  if (entropy >= ENTROPY_MED) return "med";
  return "low";
}

/** Entropy as a 0..1 fraction of the 8 bits/byte maximum (md-linear-progress takes 0..1). */
function entropyValue(entropy: number): number {
  return Math.max(0, Math.min(1, entropy / 8));
}

const LEVEL_COLOR: Record<EntropyLevel, string> = {
  high: "var(--md-sys-color-error)",
  med: "var(--aph-color-secret)",
  low: "var(--md-sys-color-primary)",
};

export function TriagePanel() {
  const [path, setPath] = useState("/usr/bin/ls");
  const [running, setRunning] = useState(false);
  const [report, setReport] = useState<TriageReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [stringsOpen, setStringsOpen] = useState(false);

  const bucket = useMemo<FormatBucket>(
    () => (report ? formatBucket(report.format) : "unknown"),
    [report],
  );

  const shaShort = useMemo(() => {
    const sha = report?.sha256 ?? "";
    return sha.length > 24 ? `${sha.slice(0, 12)}…${sha.slice(-12)}` : sha;
  }, [report]);

  // `re triage` ALWAYS emits JSON (compact by default, pretty with `--pretty`);
  // there is no `--json` flag. We request `--pretty` for a deterministic shape.
  const analyze = async () => {
    const p = path.trim();
    if (!p || running) return;
    setRunning(true);
    setError(null);
    setReport(null);
    setStringsOpen(false);
    try {
      const res = await run(["re", "triage", p, "--pretty"]);
      if (res.code !== 0) {
        setError(
          (res.stderr || res.stdout || `Le binaire a renvoyé le code ${res.code}.`)
            .trim()
            .slice(0, 2000),
        );
        return;
      }
      const out = res.stdout.trim();
      if (!out) {
        setError("La commande n'a renvoyé aucune sortie.");
        return;
      }
      setReport(parseReport(JSON.parse(out)));
    } catch (err) {
      setError(`Sortie illisible (JSON invalide) : ${String(err)}`);
    } finally {
      setRunning(false);
    }
  };

  return (
    <div className="aph-triage">
      <MdOutlinedCard className="aph-runcard">
        <div className="aph-runcard__row">
          <MdIcon className="aph-runcard__glyph">biotech</MdIcon>
          <div className="aph-runcard__text">
            <span className="aph-runcard__title">Triage typé d'un binaire</span>
            <span className="aph-muted aph-runcard__hint">
              Format, sections + entropie, imports/exports, SHA-256 (aphrody re triage).
            </span>
          </div>
        </div>
        <div className="aph-runcard__input">
          <MdOutlinedTextField
            className="aph-runcard__field"
            label="Chemin du binaire (PE / ELF)"
            value={path}
            disabled={running}
            onInput={(e) => setPath((e.target as HTMLInputElement).value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") void analyze();
            }}
          />
          <MdFilledButton disabled={running || !path.trim()} onClick={() => void analyze()}>
            <MdIcon slot="icon">play_arrow</MdIcon>
            {running ? "Analyse…" : "Analyser"}
          </MdFilledButton>
        </div>
      </MdOutlinedCard>

      {error && (
        <MdOutlinedCard className="aph-errcard">
          <div className="aph-errcard__head">
            <MdIcon>error</MdIcon>
            <b>Échec du triage</b>
          </div>
          <pre className="aph-errcard__body">{error}</pre>
        </MdOutlinedCard>
      )}

      {report && (
        <>
          <MdOutlinedCard className="aph-hdrcard">
            <div className="aph-hdrcard__top">
              <span className="aph-fmtchip" data-fmt={bucket}>
                <MdIcon>{formatIcon(bucket)}</MdIcon>
                {formatLabel(report.format)}
              </span>
              {report.arch && (
                <span className="aph-hkv">
                  <span className="aph-hkv__k">Architecture</span>
                  <span className="aph-hkv__v">{report.arch}</span>
                </span>
              )}
              <span className="aph-hkv">
                <span className="aph-hkv__k">Taille</span>
                <span className="aph-hkv__v">{humanSize(report.size)}</span>
              </span>
              {report.entry_point !== null && (
                <span className="aph-hkv">
                  <span className="aph-hkv__k">Point d'entrée</span>
                  <span className="aph-hkv__v aph-mono">{hex(report.entry_point)}</span>
                </span>
              )}
            </div>
            <div className="aph-sharow">
              <span className="aph-hkv__k">SHA-256</span>
              <code className="aph-mono aph-sha" title={report.sha256}>
                {shaShort}
              </code>
            </div>
          </MdOutlinedCard>

          {report.sections.length > 0 && (
            <MdOutlinedCard className="aph-blockcard">
              <div className="aph-blockhead">
                <MdIcon>view_module</MdIcon>
                <span>Sections</span>
                <span className="aph-blockhead__count">{report.sections.length}</span>
              </div>
              <table className="aph-sectable">
                <thead>
                  <tr>
                    <th>Nom</th>
                    <th>Adresse</th>
                    <th>Taille</th>
                    <th>Entropie (bits/octet)</th>
                  </tr>
                </thead>
                <tbody>
                  {report.sections.map((s) => {
                    const lvl = s.entropy === null ? null : entropyLevel(s.entropy);
                    return (
                      <tr key={`${s.name}-${s.vaddr}`}>
                        <td>
                          <code className="aph-mono">{s.name}</code>
                        </td>
                        <td>
                          <code className="aph-mono aph-muted">{hex(s.vaddr)}</code>
                        </td>
                        <td>{humanSize(s.size)}</td>
                        <td>
                          {s.entropy === null || lvl === null ? (
                            <span className="aph-muted">—</span>
                          ) : (
                            <div className="aph-entropy">
                              <span
                                className="aph-entropy__num"
                                style={{
                                  color: lvl === "low" ? undefined : LEVEL_COLOR[lvl],
                                  fontWeight: lvl === "high" ? 600 : undefined,
                                }}
                              >
                                {s.entropy.toFixed(2)}
                              </span>
                              <MdLinearProgress
                                className="aph-entropy__bar"
                                value={entropyValue(s.entropy)}
                                aria-label={`Section ${s.name} : entropie ${s.entropy.toFixed(2)} sur 8 bits par octet`}
                                style={
                                  {
                                    "--md-linear-progress-active-indicator-color": LEVEL_COLOR[lvl],
                                  } as React.CSSProperties
                                }
                              />
                            </div>
                          )}
                        </td>
                      </tr>
                    );
                  })}
                </tbody>
              </table>
              <p className="aph-legend">
                <span className="aph-legend__sw" style={{ background: LEVEL_COLOR.high }} /> ≥{" "}
                {ENTROPY_HIGH} : probablement packé / chiffré
                <span className="aph-legend__sw" style={{ background: LEVEL_COLOR.med }} /> ≥{" "}
                {ENTROPY_MED} : dense
                <span className="aph-legend__sw" style={{ background: LEVEL_COLOR.low }} /> faible
              </p>
            </MdOutlinedCard>
          )}

          {(report.imports.length > 0 || report.exports.length > 0) && (
            <div className="aph-symgrid">
              {report.imports.length > 0 && (
                <MdOutlinedCard className="aph-blockcard">
                  <div className="aph-blockhead">
                    <MdIcon>south_west</MdIcon>
                    <span>Imports</span>
                    <span className="aph-blockhead__count">{report.imports.length}</span>
                  </div>
                  <MdChipSet className="aph-symchips" aria-label="Symboles importés">
                    {report.imports.map((sym, i) => (
                      <MdAssistChip key={`${sym}-${i}`} label={sym} />
                    ))}
                  </MdChipSet>
                </MdOutlinedCard>
              )}
              {report.exports.length > 0 && (
                <MdOutlinedCard className="aph-blockcard">
                  <div className="aph-blockhead">
                    <MdIcon>north_east</MdIcon>
                    <span>Exports</span>
                    <span className="aph-blockhead__count">{report.exports.length}</span>
                  </div>
                  <MdChipSet className="aph-symchips" aria-label="Symboles exportés">
                    {report.exports.map((sym, i) => (
                      <MdAssistChip key={`${sym}-${i}`} label={sym} />
                    ))}
                  </MdChipSet>
                </MdOutlinedCard>
              )}
            </div>
          )}

          {report.strings_sample.length > 0 && (
            <MdOutlinedCard className="aph-blockcard">
              <button
                className="aph-blockhead aph-blockhead--button"
                type="button"
                onClick={() => setStringsOpen((o) => !o)}
                aria-expanded={stringsOpen}
              >
                <MdIcon>data_array</MdIcon>
                <span>Échantillon de chaînes</span>
                <span className="aph-blockhead__count">{report.strings_sample.length}</span>
                <MdIcon className="aph-blockhead__chevron">
                  {stringsOpen ? "expand_less" : "expand_more"}
                </MdIcon>
              </button>
              {stringsOpen && (
                <ul className="aph-strlist">
                  {report.strings_sample.map((str, i) => (
                    <li key={i} className="aph-mono">
                      {str}
                    </li>
                  ))}
                </ul>
              )}
            </MdOutlinedCard>
          )}
        </>
      )}
    </div>
  );
}
