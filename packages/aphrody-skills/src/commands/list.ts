// SPDX-License-Identifier: Apache-2.0
//
// `skills list` — flat table of every discovered skill across active sources.

import { readFileSync, statSync } from "node:fs";

import { parseFrontMatter } from "../frontmatter.ts";
import { activeSources, discoverSkills, sourceBySlug, type SourceSlug } from "../sources.ts";

export interface ListRow {
  readonly name: string;
  readonly source: SourceSlug;
  readonly schema: string;
  readonly mode: string;
  readonly description: string;
  readonly loc: number;
  readonly size_bytes: number;
  readonly path: string;
}

interface ListOptions {
  readonly sourceSlug?: string;
  readonly json?: boolean;
}

function locOf(path: string): { loc: number; size: number } {
  try {
    const buf = readFileSync(path, "utf8");
    const loc = buf.split(/\r?\n/).length;
    const size = statSync(path).size;
    return { loc, size };
  } catch {
    return { loc: 0, size: 0 };
  }
}

function rowsFor(opts: ListOptions): ListRow[] {
  const sources = opts.sourceSlug
    ? [sourceBySlug(opts.sourceSlug)].filter((s): s is NonNullable<typeof s> => s !== null)
    : activeSources();

  const rows: ListRow[] = [];
  for (const spec of sources) {
    for (const skill of discoverSkills(spec)) {
      const { loc, size } = locOf(skill.skillMdPath);
      let mode = "";
      let description = "";
      try {
        const raw = readFileSync(skill.skillMdPath, "utf8");
        const fm = parseFrontMatter(raw);
        mode = fm.od?.mode ?? "";
        description = fm.description ?? "";
      } catch {
        // tolerate read errors — leave fields blank
      }
      rows.push({
        name: skill.name,
        source: spec.slug,
        schema: spec.schema,
        mode,
        description,
        loc,
        size_bytes: size,
        path: skill.skillMdPath,
      });
    }
  }
  rows.sort((a, b) =>
    a.source === b.source ? a.name.localeCompare(b.name) : a.source.localeCompare(b.source),
  );
  return rows;
}

function truncate(s: string, max: number): string {
  if (s.length <= max) {
    return s;
  }
  return `${s.slice(0, Math.max(0, max - 1))}…`;
}

export function runList(argv: readonly string[]): number {
  const opts: { sourceSlug?: string; json: boolean } = { json: false };
  for (const arg of argv) {
    if (arg === "--json") {
      opts.json = true;
    } else if (arg.startsWith("--source=")) {
      opts.sourceSlug = arg.slice("--source=".length);
    } else if (arg === "--help" || arg === "-h") {
      process.stdout.write(
        "Usage: skills list [--source=<slug>] [--json]\n" +
          "       Lists every discovered skill across active sources.\n",
      );
      return 0;
    } else {
      process.stderr.write(`list: unknown argument: ${arg}\n`);
      return 2;
    }
  }

  const listOpts: ListOptions = opts.sourceSlug
    ? { sourceSlug: opts.sourceSlug, json: opts.json }
    : { json: opts.json };

  const rows = rowsFor(listOpts);

  if (opts.json) {
    process.stdout.write(`${JSON.stringify(rows, null, 2)}\n`);
    return 0;
  }

  if (rows.length === 0) {
    process.stdout.write("No skills discovered. Run `skills sources` to see source status.\n");
    return 0;
  }

  const nameW = Math.max(4, ...rows.map((r) => r.name.length));
  const srcW = Math.max(6, ...rows.map((r) => r.source.length));
  const modeW = Math.max(4, ...rows.map((r) => r.mode.length));

  const header =
    `${"NAME".padEnd(nameW)}  ${"SOURCE".padEnd(srcW)}  ${"MODE".padEnd(modeW)}  ` +
    `${"LOC".padStart(5)}  DESCRIPTION\n`;
  process.stdout.write(header);
  process.stdout.write(`${"-".repeat(header.length - 1)}\n`);
  for (const r of rows) {
    process.stdout.write(
      `${r.name.padEnd(nameW)}  ${r.source.padEnd(srcW)}  ${r.mode.padEnd(modeW)}  ` +
        `${String(r.loc).padStart(5)}  ${truncate(r.description, 72)}\n`,
    );
  }
  process.stdout.write(`\n${rows.length} skill(s).\n`);
  return 0;
}
