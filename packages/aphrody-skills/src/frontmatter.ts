// SPDX-License-Identifier: Apache-2.0
//
// @aphrody/skills — minimal YAML front matter parser.
//
// Targets the subset that SKILL.md files actually use across the 6 upstream
// schemas (open-design, openclaw, aphrody, gemini, vercel, google). Avoids
// pulling in `js-yaml` so the package stays zero-dep.

export interface FrontMatter {
  readonly name?: string;
  readonly description?: string;
  readonly license?: string;
  readonly triggers?: readonly string[];
  readonly when_to_use?: string;
  readonly od?: {
    readonly mode?: string;
    readonly category?: string;
    readonly upstream?: string;
  };
  readonly metadata?: Record<string, string>;
  readonly raw: Record<string, unknown>;
}

const FENCE = /^---\s*\r?\n/;

/** Extract front matter + body from a SKILL.md source string. */
export function splitFrontMatter(src: string): { fm: string; body: string } {
  if (!FENCE.test(src)) {
    return { fm: "", body: src };
  }
  const afterOpen = src.replace(FENCE, "");
  const closeIdx = afterOpen.search(/\r?\n---\s*(\r?\n|$)/);
  if (closeIdx < 0) {
    return { fm: "", body: src };
  }
  const fm = afterOpen.slice(0, closeIdx);
  const body = afterOpen.slice(closeIdx).replace(/^\r?\n---\s*\r?\n?/, "");
  return { fm, body };
}

interface Line {
  readonly indent: number;
  readonly key: string;
  readonly value: string;
  readonly raw: string;
}

const stripQuotes = (s: string): string => {
  const trimmed = s.trim();
  if (trimmed.length >= 2) {
    const first = trimmed.charAt(0);
    const last = trimmed.charAt(trimmed.length - 1);
    if ((first === '"' && last === '"') || (first === "'" && last === "'")) {
      return trimmed.slice(1, -1);
    }
  }
  return trimmed;
};

const parseLine = (line: string): Line | null => {
  if (line.trim().length === 0 || line.trim().startsWith("#")) {
    return null;
  }
  const indent = line.length - line.replace(/^[ \t]+/, "").length;
  const stripped = line.slice(indent);
  if (stripped.startsWith("- ")) {
    return { indent, key: "-", value: stripped.slice(2).trim(), raw: line };
  }
  const colonIdx = stripped.indexOf(":");
  if (colonIdx < 0) {
    return null;
  }
  const key = stripped.slice(0, colonIdx).trim();
  const value = stripped.slice(colonIdx + 1);
  return { indent, key, value: value.trim(), raw: line };
};

/** Parse a YAML front-matter string (subset). Tolerant by design. */
export function parseFrontMatter(src: string): FrontMatter {
  const { fm } = splitFrontMatter(src);
  const raw: Record<string, unknown> = {};
  if (fm.length === 0) {
    return { raw };
  }

  const lines = fm.split(/\r?\n/);
  const parsed: Line[] = [];
  for (const line of lines) {
    const p = parseLine(line);
    if (p !== null) {
      parsed.push(p);
    }
  }

  let i = 0;
  while (i < parsed.length) {
    const line = parsed[i];
    if (line === undefined) {
      i += 1;
      continue;
    }
    if (line.indent !== 0 || line.key === "-") {
      i += 1;
      continue;
    }

    // Block scalar: `key: |` or `key: >` followed by an indented block.
    if (line.value === "|" || line.value === ">") {
      const block: string[] = [];
      let j = i + 1;
      while (j < parsed.length) {
        const next = parsed[j];
        if (next === undefined || next.indent === 0) {
          break;
        }
        block.push(next.raw.trim());
        j += 1;
      }
      raw[line.key] = block.join(line.value === "|" ? "\n" : " ");
      i = j;
      continue;
    }

    // Nested mapping or list: empty value with indented children.
    if (line.value.length === 0) {
      // Look ahead.
      let j = i + 1;
      const childLines: Line[] = [];
      while (j < parsed.length) {
        const next = parsed[j];
        if (next === undefined || next.indent === 0) {
          break;
        }
        childLines.push(next);
        j += 1;
      }
      if (childLines.length > 0 && childLines.every((c) => c.key === "-")) {
        raw[line.key] = childLines.map((c) => stripQuotes(c.value));
      } else if (childLines.length > 0) {
        const map: Record<string, string> = {};
        for (const c of childLines) {
          if (c.key !== "-" && c.value.length > 0) {
            map[c.key] = stripQuotes(c.value);
          }
        }
        raw[line.key] = map;
      } else {
        raw[line.key] = "";
      }
      i = j;
      continue;
    }

    raw[line.key] = stripQuotes(line.value);
    i += 1;
  }

  // Multi-line continuations: handle the open-design pattern where
  // `description: |` spans further lines that all start at column 2.
  // Already handled above; nothing to do.

  const odField = raw["od"];
  const od =
    odField && typeof odField === "object" && !Array.isArray(odField)
      ? (odField as Record<string, string>)
      : undefined;
  const triggersField = raw["triggers"];
  const triggers = Array.isArray(triggersField)
    ? (triggersField as string[])
    : undefined;
  const metadataField = raw["metadata"];
  const metadata =
    metadataField && typeof metadataField === "object" && !Array.isArray(metadataField)
      ? (metadataField as Record<string, string>)
      : undefined;

  const out: FrontMatter = {
    raw,
    ...(typeof raw["name"] === "string" ? { name: raw["name"] as string } : {}),
    ...(typeof raw["description"] === "string"
      ? { description: (raw["description"] as string).replace(/\s+/g, " ").trim() }
      : {}),
    ...(typeof raw["license"] === "string" ? { license: raw["license"] as string } : {}),
    ...(typeof raw["when_to_use"] === "string"
      ? { when_to_use: raw["when_to_use"] as string }
      : {}),
    ...(triggers ? { triggers } : {}),
    ...(od ? { od } : {}),
    ...(metadata ? { metadata } : {}),
  };
  return out;
}
