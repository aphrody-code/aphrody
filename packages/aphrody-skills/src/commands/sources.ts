// SPDX-License-Identifier: Apache-2.0
//
// `skills sources` — print the 6-source registry with discovered-count.

import { SOURCES, discoverSkills, resolveSourceRoot } from "../sources.ts";

interface Row {
  readonly slug: string;
  readonly label: string;
  readonly schema: string;
  readonly path: string;
  readonly count: number;
  readonly status: string;
}

export function runSources(argv: readonly string[]): number {
  let json = false;
  for (const arg of argv) {
    if (arg === "--help" || arg === "-h") {
      process.stdout.write(
        "Usage: skills sources [--json]\n" +
          "       Prints the source registry (slug, schema, resolved path, skill count).\n",
      );
      return 0;
    }
    if (arg === "--json") {
      json = true;
    } else {
      process.stderr.write(`sources: unknown argument: ${arg}\n`);
      return 2;
    }
  }

  const rows: Row[] = [];
  for (const spec of SOURCES) {
    const root = resolveSourceRoot(spec);
    const skills = discoverSkills(spec);
    rows.push({
      slug: spec.slug,
      label: spec.label,
      schema: spec.schema,
      path: root ?? "(missing)",
      count: skills.length,
      status: root === null ? "missing" : skills.length === 0 ? "empty" : "ok",
    });
  }

  if (json) {
    process.stdout.write(`${JSON.stringify(rows, null, 2)}\n`);
    return 0;
  }

  const slugW = Math.max(4, ...rows.map((r) => r.slug.length));
  const schemaW = Math.max(6, ...rows.map((r) => r.schema.length));
  const statusW = 7;
  process.stdout.write(
    `${"SLUG".padEnd(slugW)}  ${"SCHEMA".padEnd(schemaW)}  ${"SKILLS".padStart(6)}  ` +
      `${"STATUS".padEnd(statusW)}  PATH\n`,
  );
  process.stdout.write(`${"-".repeat(slugW + schemaW + 6 + statusW + 10)}\n`);
  for (const r of rows) {
    process.stdout.write(
      `${r.slug.padEnd(slugW)}  ${r.schema.padEnd(schemaW)}  ${String(r.count).padStart(6)}  ` +
        `${r.status.padEnd(statusW)}  ${r.path}\n`,
    );
  }
  return 0;
}
