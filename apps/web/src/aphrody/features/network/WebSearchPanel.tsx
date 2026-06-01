// Typed M3 panel for `aphrody search <requête…>` — parses the plain-text DuckDuckGo results.

import { useState } from "react";
import {
  MdCircularProgress,
  MdFilledButton,
  MdIcon,
  MdOutlinedCard,
  MdOutlinedTextField,
} from "@aphrody-code/m3-react";
import { run } from "../../client.ts";
import { CodeOutput } from "../../ui.tsx";

/** One typed web search result. */
interface SearchResult {
  /** Result title (line preceding the URL). */
  title: string;
  /** Absolute result URL (already un-wrapped from the DDG redirect by the CLI). */
  url: string;
  /** Optional snippet/description (line following the URL). */
  snippet: string;
}

/** Matches an http(s) URL line emitted by the CLI for a result. */
const URL_LINE = /^https?:\/\/\S+$/i;

/**
 * Reconstruct result blocks from the line-oriented CLI output. A URL line is the
 * structural anchor: the preceding non-URL line is the title, the following
 * non-URL line (if any) is the snippet. The CLI's leading search-glyph header is
 * ignored.
 */
function parseSearch(stdout: string): SearchResult[] {
  const SEARCH_HEADER = "\u{1F50D}";
  const lines = stdout
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter((l) => l.length > 0 && !l.startsWith(SEARCH_HEADER));

  const out: SearchResult[] = [];
  for (let i = 0; i < lines.length; i++) {
    if (!URL_LINE.test(lines[i])) continue;
    const url = lines[i];
    const prev = i > 0 ? lines[i - 1] : "";
    const title = prev && !URL_LINE.test(prev) ? prev : url;
    const next = i + 1 < lines.length ? lines[i + 1] : "";
    const snippet = next && !URL_LINE.test(next) ? next : "";
    out.push({ title, url, snippet });
  }
  return out;
}

export function WebSearchPanel() {
  const [query, setQuery] = useState("");
  const [running, setRunning] = useState(false);
  const [results, setResults] = useState<SearchResult[] | null>(null);
  const [rawFallback, setRawFallback] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [ranQuery, setRanQuery] = useState("");

  const exec = async () => {
    const q = query.trim();
    if (!q || running) return;
    setRunning(true);
    setError(null);
    setResults(null);
    setRawFallback(null);
    setRanQuery(q);
    try {
      // `query` is a trailing var-arg (Vec<String>): split on whitespace so a
      // multi-word query maps to distinct argv tokens like the shell would.
      const res = await run(["search", ...q.split(/\s+/).filter(Boolean)]);
      if (res.code !== 0) {
        setError(
          (res.stderr || res.stdout || `La commande a renvoyé le code ${res.code}.`)
            .trim()
            .slice(0, 2000),
        );
        return;
      }
      const out = res.stdout ?? "";
      if (/Aucun résultat trouvé/i.test(out)) {
        setResults([]);
        return;
      }
      const parsed = parseSearch(out);
      if (parsed.length > 0) {
        setResults(parsed);
      } else {
        const trimmed = out.trim();
        setRawFallback(trimmed || (res.stderr || "").trim() || "Aucune sortie.");
      }
    } catch (err) {
      setError(`Recherche impossible : ${String(err)}`);
    } finally {
      setRunning(false);
    }
  };

  return (
    <div className="aph-net-panel">
      <MdOutlinedCard className="aph-net-run">
        <div className="aph-net-run__row">
          <MdIcon className="aph-net-run__glyph">search</MdIcon>
          <div className="aph-net-run__text">
            <span className="aph-net-run__title">Recherche web typée</span>
            <span className="aph-muted aph-net-run__hint">
              Résultats web sans clé ni navigateur, via DuckDuckGo (aphrody search).
            </span>
          </div>
        </div>
        <div className="aph-net-run__input">
          <MdOutlinedTextField
            className="aph-net-run__field"
            label="Requête de recherche"
            value={query}
            disabled={running}
            onInput={(e) => setQuery((e.target as HTMLInputElement).value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") void exec();
            }}
          />
          <MdFilledButton disabled={running || !query.trim()} onClick={() => void exec()}>
            {running ? (
              <MdCircularProgress indeterminate slot="icon" />
            ) : (
              <MdIcon slot="icon">search</MdIcon>
            )}
            {running ? "Recherche…" : "Rechercher"}
          </MdFilledButton>
        </div>
      </MdOutlinedCard>

      {error && (
        <MdOutlinedCard className="aph-net-err">
          <div className="aph-net-err__head">
            <MdIcon>error</MdIcon>
            <b>Échec de la recherche</b>
          </div>
          <CodeOutput text={error} empty="Sortie vide." />
        </MdOutlinedCard>
      )}

      {results ? (
        <>
          <div className="aph-net-res-head">
            <MdIcon>travel_explore</MdIcon>
            <span>Résultats pour « {ranQuery} »</span>
            <span className="aph-net-count">{results.length}</span>
          </div>
          {results.length === 0 ? (
            <MdOutlinedCard className="aph-net-list">
              <p className="aph-muted aph-net-empty">
                <MdIcon>search_off</MdIcon>
                Aucun résultat trouvé (DuckDuckGo a renvoyé une page vide — réessayer plus tard).
              </p>
            </MdOutlinedCard>
          ) : (
            results.map((r) => (
              <MdOutlinedCard key={r.url + r.title} className="aph-net-res">
                <a
                  className="aph-net-res__title"
                  href={r.url}
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  {r.title}
                </a>
                <a
                  className="aph-net-res__url"
                  href={r.url}
                  target="_blank"
                  rel="noopener noreferrer"
                >
                  <MdIcon className="aph-net-res__link-glyph">link</MdIcon>
                  <span className="aph-net-res__url-text">{r.url}</span>
                </a>
                {r.snippet && <p className="aph-net-res__snippet">{r.snippet}</p>}
              </MdOutlinedCard>
            ))
          )}
        </>
      ) : (
        rawFallback && (
          <MdOutlinedCard className="aph-net-list">
            <div className="aph-net-block-head">
              <MdIcon>notes</MdIcon>
              <span>Sortie brute</span>
            </div>
            <CodeOutput text={rawFallback} empty="Aucune sortie." />
          </MdOutlinedCard>
        )
      )}
    </div>
  );
}
