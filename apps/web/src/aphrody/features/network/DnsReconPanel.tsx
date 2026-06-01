// Typed M3 panel for `aphrody dns <domaine>` — parses the plain-text OSINT report.

import { useMemo, useState } from "react";
import {
  MdCircularProgress,
  MdFilledButton,
  MdIcon,
  MdOutlinedCard,
  MdOutlinedTextField,
} from "@aphrody-code/m3-react";
import { run } from "../../client.ts";
import { CodeOutput } from "../../ui.tsx";

/** Typed result of a DNS OSINT reconnaissance run. */
interface DnsReport {
  /** Resolved target domain (echoed from the `[~] Cible:` line). */
  domain: string;
  /** Total unique subdomains found, parsed from the `[+]` summary line. */
  total: number;
  /** The subdomains actually printed by the CLI (capped at 10). */
  subdomains: string[];
  /** Count of subdomains found but not printed (`... et M autres`). */
  hiddenCount: number;
}

/**
 * Parse the plain-text `aphrody dns` output. Returns null when no usable
 * summary/subdomain data is present (caller surfaces the raw output instead).
 */
function parseDns(stdout: string, domain: string): DnsReport | null {
  const lines = stdout.split(/\r?\n/);

  if (lines.some((l) => l.includes("[-] Erreur lors de la résolution OSINT"))) {
    return null;
  }

  let total: number | null = null;
  let hiddenCount = 0;
  const subdomains: string[] = [];
  let resolvedDomain = domain;

  for (const raw of lines) {
    const line = raw.trim();
    const cible = /^\[~\]\s*Cible\s*:\s*(.+)$/.exec(line);
    if (cible) {
      resolvedDomain = cible[1].trim() || domain;
      continue;
    }
    const summary = /(\d+)\s+sous-domaines?\s+uniques/.exec(line);
    if (summary) {
      total = Number.parseInt(summary[1], 10);
      continue;
    }
    const more = /^\.\.\.\s*et\s+(\d+)\s+autres?$/.exec(line);
    if (more) {
      hiddenCount = Number.parseInt(more[1], 10);
      continue;
    }
    const sub = /^-\s+(\S.*)$/.exec(line);
    if (sub) {
      subdomains.push(sub[1].trim());
    }
  }

  if (total === null && subdomains.length === 0) {
    return null;
  }

  const resolvedTotal = total ?? subdomains.length + hiddenCount;
  const finalTotal = Math.max(resolvedTotal, subdomains.length + hiddenCount);

  return { domain: resolvedDomain, total: finalTotal, subdomains, hiddenCount };
}

export function DnsReconPanel() {
  const [domain, setDomain] = useState("");
  const [running, setRunning] = useState(false);
  const [report, setReport] = useState<DnsReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [filter, setFilter] = useState("");

  const filtered = useMemo(() => {
    const subs = report?.subdomains ?? [];
    const q = filter.trim().toLowerCase();
    if (!q) return subs;
    return subs.filter((s) => s.toLowerCase().includes(q));
  }, [report, filter]);

  const exec = async () => {
    const d = domain.trim();
    if (!d || running) return;
    setRunning(true);
    setError(null);
    setReport(null);
    setFilter("");
    try {
      const res = await run(["dns", d]);
      if (res.code !== 0) {
        setError(
          (res.stderr || res.stdout || `La commande a renvoyé le code ${res.code}.`)
            .trim()
            .slice(0, 2000),
        );
        return;
      }
      const parsed = parseDns(res.stdout, d);
      if (!parsed) {
        setError((res.stdout || res.stderr || "Sortie vide.").trim().slice(0, 2000));
        return;
      }
      setReport(parsed);
    } catch (err) {
      setError(`Reconnaissance impossible : ${String(err)}`);
    } finally {
      setRunning(false);
    }
  };

  return (
    <div className="aph-net-panel">
      <MdOutlinedCard className="aph-net-run">
        <div className="aph-net-run__row">
          <MdIcon className="aph-net-run__glyph">dns</MdIcon>
          <div className="aph-net-run__text">
            <span className="aph-net-run__title">Reconnaissance DNS typée</span>
            <span className="aph-muted aph-net-run__hint">
              Énumération OSINT passive des sous-domaines via crt.sh + HackerTarget (aphrody dns).
            </span>
          </div>
        </div>
        <div className="aph-net-run__input">
          <MdOutlinedTextField
            className="aph-net-run__field"
            label="Domaine (ex. example.com)"
            value={domain}
            disabled={running}
            onInput={(e) => setDomain((e.target as HTMLInputElement).value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") void exec();
            }}
          />
          <MdFilledButton disabled={running || !domain.trim()} onClick={() => void exec()}>
            {running ? (
              <MdCircularProgress indeterminate slot="icon" />
            ) : (
              <MdIcon slot="icon">travel_explore</MdIcon>
            )}
            {running ? "Reconnaissance…" : "Reconnaissance"}
          </MdFilledButton>
        </div>
      </MdOutlinedCard>

      {error && (
        <MdOutlinedCard className="aph-net-err">
          <div className="aph-net-err__head">
            <MdIcon>error</MdIcon>
            <b>Échec de la reconnaissance DNS</b>
          </div>
          <CodeOutput text={error} empty="Sortie vide." />
        </MdOutlinedCard>
      )}

      {report && (
        <>
          <MdOutlinedCard className="aph-net-hdr">
            <div className="aph-net-hdr__top">
              <span className="aph-net-chip">
                <MdIcon>public</MdIcon>
                {report.domain}
              </span>
              <span className="aph-net-kv">
                <span className="aph-net-kv__k">Sous-domaines uniques</span>
                <span className="aph-net-kv__v">{report.total}</span>
              </span>
              {report.hiddenCount > 0 && (
                <span className="aph-net-kv">
                  <span className="aph-net-kv__k">Affichés par le CLI</span>
                  <span className="aph-net-kv__v">
                    {report.subdomains.length} <span className="aph-muted">/ {report.total}</span>
                  </span>
                </span>
              )}
            </div>
            {report.hiddenCount > 0 && (
              <p className="aph-net-note">
                <MdIcon>info</MdIcon>
                Le CLI ne liste que les 10 premiers sous-domaines ; {report.hiddenCount} autres ne
                sont pas détaillés.
              </p>
            )}
          </MdOutlinedCard>

          {report.subdomains.length > 0 ? (
            <MdOutlinedCard className="aph-net-list">
              <div className="aph-net-block-head">
                <MdIcon>lan</MdIcon>
                <span>Sous-domaines</span>
                <span className="aph-net-count">{filtered.length}</span>
                {report.subdomains.length > 6 && (
                  <MdOutlinedTextField
                    className="aph-net-filter"
                    label="Filtrer…"
                    value={filter}
                    onInput={(e) => setFilter((e.target as HTMLInputElement).value)}
                  />
                )}
              </div>
              {filtered.length === 0 ? (
                <p className="aph-muted aph-net-empty">
                  Aucun sous-domaine ne correspond au filtre.
                </p>
              ) : (
                <ul className="aph-net-grid">
                  {filtered.map((sub) => (
                    <li key={sub} className="aph-net-item">
                      <MdIcon className="aph-net-item__glyph">subdirectory_arrow_right</MdIcon>
                      <code className="aph-net-mono">{sub}</code>
                    </li>
                  ))}
                </ul>
              )}
            </MdOutlinedCard>
          ) : (
            <MdOutlinedCard className="aph-net-list">
              <p className="aph-muted aph-net-empty">
                <MdIcon>search_off</MdIcon>
                Aucun sous-domaine détaillé n'a été retourné pour ce domaine.
              </p>
            </MdOutlinedCard>
          )}
        </>
      )}
    </div>
  );
}
