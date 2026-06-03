// Skills view: catalogue of SKILL.md entries (aphrody skills list), filterable, with inline SKILL.md preview on expand.

import { useCallback, useEffect, useMemo, useState } from "react";
import {
  MdCircularProgress,
  MdIcon,
  MdIconButton,
  MdOutlinedTextField,
} from "@aphrody/m3-react";
import { run } from "../../client.ts";
import { Hint, PageHead, Spinner } from "../../ui.tsx";

/** One discovered skill parsed from `aphrody skills list`. */
interface Skill {
  name: string;
  source: string;
  mode: string;
  loc: number;
  description: string;
}

/** Source -> Material Symbol glyph. */
function sourceIcon(source: string): string {
  switch (source) {
    case "claude-code":
      return "smart_toy";
    case "antigravity":
      return "rocket_launch";
    case "gemini":
      return "auto_awesome";
    default:
      return "extension";
  }
}

/**
 * Parse the fixed-width table emitted by `aphrody skills list`. The header row
 * (NAME SOURCE MODE LOC DESCRIPTION) defines the column offsets; each data row
 * is sliced on those offsets so multi-word descriptions and empty MODE cells
 * parse cleanly. Falls back to a simple whitespace split when no header is found
 * (e.g. the mock's bullet list).
 */
function parseSkillsTable(raw: string): Skill[] {
  const lines = raw.split(/\r?\n/);
  const headerIdx = lines.findIndex((l) => /^\s*NAME\s+SOURCE/.test(l));
  if (headerIdx < 0) return parseLooseList(lines);
  const header = lines[headerIdx];
  const cols = ["NAME", "SOURCE", "MODE", "LOC", "DESCRIPTION"];
  const offsets = cols.map((c) => header.indexOf(c)).filter((i) => i >= 0);
  if (offsets.length < 5) return parseLooseList(lines);
  const [nameAt, sourceAt, modeAt, locAt, descAt] = offsets;

  const skills: Skill[] = [];
  for (let i = headerIdx + 1; i < lines.length; i++) {
    const line = lines[i];
    if (!line.trim() || /^[-\s]+$/.test(line)) continue;
    const name = line.slice(nameAt, sourceAt).trim();
    if (!name) continue;
    skills.push({
      name,
      source: line.slice(sourceAt, modeAt).trim(),
      mode: line.slice(modeAt, locAt).trim(),
      loc: Number.parseInt(line.slice(locAt, descAt).trim(), 10) || 0,
      description: line.slice(descAt).replace(/…$/, "").trim(),
    });
  }
  return skills.sort((a, b) => a.name.localeCompare(b.name));
}

/** Fallback parser for "  <name>   <description>" bullet lines. */
function parseLooseList(lines: string[]): Skill[] {
  const skills: Skill[] = [];
  for (const line of lines) {
    const m = line.match(/^\s+(\S+)\s{2,}(.+)$/);
    if (!m) continue;
    skills.push({ name: m[1], source: "", mode: "", loc: 0, description: m[2].trim() });
  }
  return skills.sort((a, b) => a.name.localeCompare(b.name));
}

export function Skills() {
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");
  const [skills, setSkills] = useState<Skill[]>([]);
  const [expanded, setExpanded] = useState<string | null>(null);
  const [body, setBody] = useState("");
  const [bodyLoading, setBodyLoading] = useState(false);

  const load = useCallback(async () => {
    setLoading(true);
    setError("");
    setExpanded(null);
    const res = await run(["skills", "list"]);
    if (res.code !== 0 && !res.stdout.trim()) {
      setError(
        (res.stderr || "aphrody skills a échoué").slice(0, 400) +
          "\nVérifiez que le catalogue de skills est disponible.",
      );
      setSkills([]);
      setLoading(false);
      return;
    }
    const parsed = parseSkillsTable(res.stdout);
    setSkills(parsed);
    if (parsed.length === 0) setError("Aucun skill découvert (catalogue vide).");
    setLoading(false);
  }, []);

  useEffect(() => {
    void load();
  }, [load]);

  const open = useCallback(
    async (s: Skill) => {
      if (expanded === s.name) {
        setExpanded(null);
        return;
      }
      setExpanded(s.name);
      setBody("");
      setBodyLoading(true);
      const res = await run(["skills", "info", s.name]);
      setBody(res.stdout.trim() || res.stderr.trim() || "(SKILL.md vide)");
      setBodyLoading(false);
    },
    [expanded],
  );

  const filtered = useMemo(() => {
    const q = query.trim().toLowerCase();
    if (!q) return skills;
    return skills.filter(
      (s) =>
        s.name.toLowerCase().includes(q) ||
        s.description.toLowerCase().includes(q) ||
        s.source.toLowerCase().includes(q),
    );
  }, [query, skills]);

  return (
    <div className="aph-skills">
      <PageHead
        title="Skills"
        subtitle="Catalogue des compétences SKILL.md (aphrody skills list)."
        actions={
          <MdIconButton aria-label="Actualiser" disabled={loading} onClick={() => void load()}>
            <MdIcon>refresh</MdIcon>
          </MdIconButton>
        }
      />

      <div className="aph-row" style={{ gap: 10, marginBottom: 16 }}>
        <MdOutlinedTextField
          style={{ flex: "1 1 auto" }}
          label="Filtrer les skills"
          value={query}
          onInput={(e) => setQuery((e.target as HTMLInputElement).value)}
        >
          <MdIcon slot="leading-icon">search</MdIcon>
        </MdOutlinedTextField>
        {skills.length > 0 && (
          <span className="aph-muted">
            {filtered.length} / {skills.length}
          </span>
        )}
      </div>

      {loading ? (
        <Spinner label="Chargement du catalogue…" />
      ) : error ? (
        <Hint icon="error" title="Catalogue de skills indisponible." text={error} />
      ) : filtered.length === 0 ? (
        <Hint icon="search_off" title={`Aucun skill ne correspond à « ${query} ».`} />
      ) : (
        <div className="aph-stack">
          {filtered.map((s) => (
            <div key={s.name} className="aph-skill">
              <div
                className="aph-skill__row aph-row"
                style={{ gap: 14, cursor: "pointer" }}
                onClick={() => void open(s)}
              >
                <MdIcon style={{ color: "var(--md-sys-color-primary)" }}>
                  {sourceIcon(s.source)}
                </MdIcon>
                <div style={{ flex: "1 1 auto", minWidth: 0 }}>
                  <div className="aph-row" style={{ gap: 8, flexWrap: "wrap" }}>
                    <span style={{ fontWeight: 600 }}>{s.name}</span>
                    {s.source && <span className="aph-tag">{s.source}</span>}
                    {s.mode && <span className="aph-tag aph-tag--accent">{s.mode}</span>}
                  </div>
                  <span className="aph-muted" style={{ fontSize: 12 }}>
                    {s.description || "(sans description)"}
                  </span>
                </div>
                <MdIconButton aria-label="Ouvrir le SKILL.md">
                  <MdIcon>{expanded === s.name ? "expand_less" : "unfold_more"}</MdIcon>
                </MdIconButton>
              </div>
              {expanded === s.name && (
                <div style={{ padding: "4px 16px 16px" }}>
                  {bodyLoading ? (
                    <span className="aph-row aph-muted" style={{ gap: 8 }}>
                      <MdCircularProgress indeterminate style={{ width: 18, height: 18 }} />
                      Lecture du SKILL.md…
                    </span>
                  ) : (
                    <div className="aph-output">
                      <pre>{body}</pre>
                    </div>
                  )}
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
