// MCP view: parses `aphrody mcp list` (NDJSON, one server per line) into a filterable list of tools with inline input schema.

import { useCallback, useEffect, useMemo, useState } from "react";
import { MdIcon, MdIconButton, MdOutlinedTextField } from "@aphrody-code/m3-react";
import { run } from "../../client.ts";
import { Hint, PageHead, Spinner } from "../../ui.tsx";

/** A single MCP tool as advertised by `aphrody mcp list`. */
interface McpTool {
  name: string;
  description: string;
  input_schema?: unknown;
  /** Originating MCP server (added when flattening across servers). */
  server: string;
}

/** One server line of `aphrody mcp list` (NDJSON). */
interface McpServerJson {
  server?: string;
  server_info?: { name?: string; version?: string; protocol_version?: string };
  tools?: Omit<McpTool, "server">[];
  error?: string;
}

/** A parsed server with its tool count, for the summary bar. */
interface McpServer {
  name: string;
  toolCount: number;
  error?: string;
}

function pretty(value: unknown): string {
  try {
    return JSON.stringify(value, null, 2);
  } catch {
    return String(value);
  }
}

export function Mcp() {
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [tools, setTools] = useState<McpTool[]>([]);
  const [servers, setServers] = useState<McpServer[]>([]);
  const [expanded, setExpanded] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError("");
    const res = await run(["mcp", "list"]);
    const out = res.stdout.trim();
    if (res.code !== 0 && !out) {
      setError(
        (res.stderr || "sortie inattendue").slice(0, 400) ||
          "La commande mcp list n'a renvoyé aucune donnée.",
      );
      setTools([]);
      setServers([]);
      setLoading(false);
      return;
    }

    // NDJSON: one server object per line. Parse each line independently so one
    // malformed/oversized line does not blank the whole view, and aggregate
    // every server's tools into one tagged list.
    const parsedServers: McpServer[] = [];
    const allTools: McpTool[] = [];
    for (const line of out.split("\n")) {
      const trimmed = line.trim();
      if (!trimmed.startsWith("{")) continue;
      let obj: McpServerJson;
      try {
        obj = JSON.parse(trimmed) as McpServerJson;
      } catch {
        continue;
      }
      const name = obj.server ?? obj.server_info?.name ?? "(serveur)";
      const serverTools = obj.tools ?? [];
      parsedServers.push({ name, toolCount: serverTools.length, error: obj.error });
      for (const t of serverTools) allTools.push({ ...t, server: name });
    }

    if (parsedServers.length === 0) {
      setError((res.stderr || out || "aucun serveur MCP configuré").slice(0, 400));
      setTools([]);
      setServers([]);
      setLoading(false);
      return;
    }

    setServers(parsedServers);
    setTools(
      allTools
        .slice()
        .sort((a, b) => a.server.localeCompare(b.server) || a.name.localeCompare(b.name)),
    );
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const toggle = useCallback((name: string) => {
    setExpanded((cur) => (cur === name ? null : name));
  }, []);

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return tools;
    return tools.filter(
      (t) =>
        t.name.toLowerCase().includes(q) ||
        (t.description ?? "").toLowerCase().includes(q) ||
        t.server.toLowerCase().includes(q),
    );
  }, [query, tools]);

  return (
    <div className="aph-mcp">
      <PageHead
        title="MCP"
        subtitle="Outils exposés par le serveur Model Context Protocol d'aphrody (aphrody mcp list)."
        actions={
          <MdIconButton aria-label="Actualiser" disabled={loading} onClick={() => void load()}>
            <MdIcon>refresh</MdIcon>
          </MdIconButton>
        }
      />

      {servers.length > 0 && (
        <div className="aph-chips" style={{ marginBottom: 16, alignItems: "center" }}>
          {servers.map((s) => (
            <span key={s.name} className={`aph-badge${s.error ? " aph-badge--err" : ""}`}>
              <MdIcon style={{ fontSize: 16 }}>{s.error ? "error" : "dns"}</MdIcon>
              {s.name}
              <span className="aph-badge__count">{s.toolCount}</span>
            </span>
          ))}
          <span className="aph-muted" style={{ fontSize: 12 }}>
            {servers.length} serveur(s) · {tools.length} outil(s)
          </span>
        </div>
      )}

      <div className="aph-row" style={{ marginBottom: 16 }}>
        <MdOutlinedTextField
          style={{ flex: "1 1 auto" }}
          label="Filtrer les outils"
          value={query}
          onInput={(e) => setQuery((e.target as HTMLInputElement).value)}
        >
          <MdIcon slot="leading-icon">search</MdIcon>
        </MdOutlinedTextField>
      </div>

      {loading ? (
        <Spinner label="Chargement des outils MCP…" />
      ) : error ? (
        <Hint icon="error" title="Serveur MCP indisponible." text={error} />
      ) : filtered.length === 0 ? (
        <Hint icon="search_off" title={`Aucun outil ne correspond à « ${query} ».`} />
      ) : (
        <div className="aph-stack">
          {filtered.map((t) => (
            <div key={`${t.server}/${t.name}`} className="aph-tool-card">
              <div
                className="aph-row"
                style={{ gap: 14, cursor: "pointer" }}
                onClick={() => toggle(t.name)}
              >
                <MdIcon style={{ color: "var(--md-sys-color-primary)" }}>build</MdIcon>
                <div style={{ flex: "1 1 auto", minWidth: 0 }}>
                  <div className="aph-row" style={{ gap: 8 }}>
                    <code style={{ fontFamily: '"Roboto Mono", ui-monospace, monospace' }}>
                      {t.name}
                    </code>
                    <span className="aph-tag">{t.server}</span>
                  </div>
                  <span className="aph-muted" style={{ fontSize: 12 }}>
                    {t.description || "(sans description)"}
                  </span>
                </div>
                {t.input_schema != null && (
                  <MdIconButton aria-label="Afficher le schéma">
                    <MdIcon>{expanded === t.name ? "expand_less" : "data_object"}</MdIcon>
                  </MdIconButton>
                )}
              </div>
              {expanded === t.name && t.input_schema != null && (
                <div className="aph-output" style={{ marginTop: 8 }}>
                  <pre>{pretty(t.input_schema)}</pre>
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
