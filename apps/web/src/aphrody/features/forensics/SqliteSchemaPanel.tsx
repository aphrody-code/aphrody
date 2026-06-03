// Typed M3 panel for `aphrody forensics sqlite` — opens the DB read-only and
// dumps `sqlite_master` only (object type / name / CREATE statement, never any
// row value), grouped by object kind with collapsible CREATE statements.

import { useMemo, useState } from "react";
import {
  MdFilledButton,
  MdIcon,
  MdOutlinedCard,
  MdOutlinedTextField,
} from "@aphrody/m3-react";
import { run } from "../../client.ts";

/**
 * One schema object from `sqlite_master`. The wire field is `type` (serde
 * rename); `sql` (the CREATE statement) is null for internal objects.
 */
export interface SqliteObject {
  type: string;
  name: string;
  sql: string | null;
}

/** Top-level SQLite schema dump — matches the Rust `SqliteReport` DTO. `tables` carries every object kind. */
export interface SqliteReport {
  db: string;
  object_count: number;
  tables: SqliteObject[];
}

/** A group of schema objects sharing the same `type`, for sectioned rendering. */
interface ObjectGroup {
  kind: string;
  label: string;
  icon: string;
  objects: SqliteObject[];
}

/** Display order for the known SQLite object kinds. */
const KIND_ORDER = ["table", "view", "index", "trigger"] as const;

const KIND_COLOR: Record<string, string> = {
  table: "color-mix(in srgb, #4285f4 22%, transparent)",
  view: "color-mix(in srgb, #34a853 22%, transparent)",
  index: "color-mix(in srgb, #fbbc04 28%, transparent)",
  trigger: "color-mix(in srgb, #ea4335 22%, transparent)",
};

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function parseReport(raw: unknown, fallbackDb: string): SqliteReport {
  const r = isRecord(raw) ? raw : {};
  const tables = Array.isArray(r.tables) ? (r.tables as unknown[]) : [];
  return {
    db: typeof r.db === "string" ? r.db : fallbackDb,
    object_count: typeof r.object_count === "number" ? r.object_count : 0,
    tables: tables.map((o): SqliteObject => {
      const or = isRecord(o) ? o : {};
      return {
        type: typeof or.type === "string" ? or.type : "",
        name: typeof or.name === "string" ? or.name : "",
        sql: typeof or.sql === "string" ? or.sql : null,
      };
    }),
  };
}

/** Build a display group (label + icon) for a SQLite object kind. */
function makeGroup(kind: string, objects: SqliteObject[]): ObjectGroup {
  switch (kind) {
    case "table":
      return { kind, label: "Tables", icon: "table_chart", objects };
    case "view":
      return { kind, label: "Vues", icon: "view_agenda", objects };
    case "index":
      return { kind, label: "Index", icon: "format_list_numbered", objects };
    case "trigger":
      return { kind, label: "Déclencheurs", icon: "bolt", objects };
    default:
      return { kind, label: kind ? `Type « ${kind} »` : "Objets", icon: "category", objects };
  }
}

/** Schema objects grouped by kind, in a stable display order. */
function groupObjects(objs: SqliteObject[]): ObjectGroup[] {
  const byKind = new Map<string, SqliteObject[]>();
  for (const o of objs) {
    const key = (o.type ?? "").toLowerCase();
    const bucket = byKind.get(key);
    if (bucket) bucket.push(o);
    else byKind.set(key, [o]);
  }
  const ordered: ObjectGroup[] = [];
  for (const k of KIND_ORDER) {
    const list = byKind.get(k);
    if (list && list.length > 0) {
      ordered.push(makeGroup(k, list));
      byKind.delete(k);
    }
  }
  for (const k of [...byKind.keys()].sort((a, b) => a.localeCompare(b))) {
    ordered.push(makeGroup(k, byKind.get(k) as SqliteObject[]));
  }
  return ordered;
}

export function SqliteSchemaPanel() {
  const [db, setDb] = useState("");
  const [running, setRunning] = useState(false);
  const [report, setReport] = useState<SqliteReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [openKeys, setOpenKeys] = useState<ReadonlySet<string>>(new Set());

  const groups = useMemo(() => groupObjects(report?.tables ?? []), [report]);

  const keyFor = (kind: string, index: number) => `${kind}#${index}`;
  const isOpen = (kind: string, index: number) => openKeys.has(keyFor(kind, index));
  const toggle = (kind: string, index: number) => {
    const key = keyFor(kind, index);
    setOpenKeys((prev) => {
      const next = new Set(prev);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  };

  const analyze = async () => {
    const d = db.trim();
    if (!d || running) return;
    setRunning(true);
    setError(null);
    setReport(null);
    setOpenKeys(new Set());
    try {
      const res = await run(["forensics", "sqlite", "--db", d]);
      if (res.code !== 0) {
        setError(
          (res.stderr || res.stdout || `La commande a renvoyé le code ${res.code}.`)
            .trim()
            .slice(0, 2000),
        );
        return;
      }
      const stdout = res.stdout.trim();
      // Be tolerant of any leading log lines before the JSON object.
      const start = stdout.indexOf("{");
      if (start < 0) {
        setError(
          (res.stderr || "La commande n'a renvoyé aucun objet JSON exploitable.")
            .trim()
            .slice(0, 2000),
        );
        return;
      }
      setReport(parseReport(JSON.parse(stdout.slice(start)), d));
    } catch (err) {
      setError(`Sortie illisible (JSON invalide) : ${String(err)}`);
    } finally {
      setRunning(false);
    }
  };

  return (
    <div className="aph-sqlite">
      <MdOutlinedCard className="aph-runcard">
        <div className="aph-runcard__row">
          <MdIcon className="aph-runcard__glyph">database</MdIcon>
          <div className="aph-runcard__text">
            <span className="aph-runcard__title">Schéma SQLite (lecture seule)</span>
            <span className="aph-muted aph-runcard__hint">
              Lit <code className="aph-mono">sqlite_master</code> uniquement (noms + instructions
              CREATE), jamais les valeurs (aphrody forensics sqlite).
            </span>
          </div>
        </div>
        <div className="aph-runcard__input">
          <MdOutlinedTextField
            className="aph-runcard__field"
            label="Chemin de la base .db / .sqlite / .vscdb"
            value={db}
            disabled={running}
            onInput={(e) => setDb((e.target as HTMLInputElement).value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") void analyze();
            }}
          />
          <MdFilledButton disabled={running || !db.trim()} onClick={() => void analyze()}>
            <MdIcon slot="icon">play_arrow</MdIcon>
            {running ? "Analyse…" : "Analyser"}
          </MdFilledButton>
        </div>
      </MdOutlinedCard>

      {error && (
        <MdOutlinedCard className="aph-errcard">
          <div className="aph-errcard__head">
            <MdIcon>error</MdIcon>
            <b>Échec de l'inspection du schéma</b>
          </div>
          <pre className="aph-errcard__body">{error}</pre>
        </MdOutlinedCard>
      )}

      {report && (
        <>
          <MdOutlinedCard className="aph-hdrcard">
            <div className="aph-hdrcard__top">
              <span className="aph-fmtchip" data-fmt="pe">
                <MdIcon>database</MdIcon>
                SQLite
              </span>
              <span className="aph-hkv">
                <span className="aph-hkv__k">Objets de schéma</span>
                <span className="aph-hkv__v">{report.object_count}</span>
              </span>
            </div>
            <div className="aph-sharow">
              <span className="aph-hkv__k">Base</span>
              <code className="aph-mono aph-sha" title={report.db}>
                {report.db}
              </code>
            </div>
          </MdOutlinedCard>

          {groups.length === 0 && (
            <MdOutlinedCard className="aph-emptycard">
              <MdIcon>inbox</MdIcon>
              <span>Aucun objet de schéma (base vide ou catalogue interne uniquement).</span>
            </MdOutlinedCard>
          )}

          {groups.map((g) => (
            <MdOutlinedCard key={g.kind} className="aph-blockcard">
              <div className="aph-blockhead">
                <MdIcon>{g.icon}</MdIcon>
                <span>{g.label}</span>
                <span className="aph-blockhead__count">{g.objects.length}</span>
              </div>
              <ul className="aph-objlist">
                {g.objects.map((o, i) => (
                  <li key={o.name + i} className="aph-objitem">
                    <button
                      className="aph-objrow"
                      type="button"
                      onClick={() => toggle(g.kind, i)}
                      aria-expanded={isOpen(g.kind, i)}
                      disabled={!o.sql}
                    >
                      <span className="aph-kindchip" style={{ background: KIND_COLOR[g.kind] }}>
                        {o.type}
                      </span>
                      <code className="aph-mono aph-objname">{o.name}</code>
                      {o.sql ? (
                        <MdIcon className="aph-objrow__chevron">
                          {isOpen(g.kind, i) ? "expand_less" : "expand_more"}
                        </MdIcon>
                      ) : (
                        <span className="aph-nosql">interne</span>
                      )}
                    </button>
                    {o.sql && isOpen(g.kind, i) && <pre className="aph-mono aph-sql">{o.sql}</pre>}
                  </li>
                ))}
              </ul>
            </MdOutlinedCard>
          ))}
        </>
      )}
    </div>
  );
}
