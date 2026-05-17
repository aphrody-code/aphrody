// SPDX-License-Identifier: Apache-2.0
//
// `skills info <name>` — print the full SKILL.md for one skill.

import { readFileSync } from "node:fs";

import { activeSources, discoverSkills, type DiscoveredSkill } from "../sources.ts";

function findByName(name: string, sourceSlug?: string): DiscoveredSkill[] {
  const matches: DiscoveredSkill[] = [];
  for (const spec of activeSources()) {
    if (sourceSlug && spec.slug !== sourceSlug) {
      continue;
    }
    for (const sk of discoverSkills(spec)) {
      if (sk.name === name) {
        matches.push(sk);
      }
    }
  }
  return matches;
}

export function runInfo(argv: readonly string[]): number {
  let name: string | null = null;
  let sourceSlug: string | undefined;
  for (const arg of argv) {
    if (arg === "--help" || arg === "-h") {
      process.stdout.write(
        "Usage: skills info <name> [--source=<slug>]\n" +
          "       Prints the full SKILL.md for the named skill.\n",
      );
      return 0;
    }
    if (arg.startsWith("--source=")) {
      sourceSlug = arg.slice("--source=".length);
    } else if (name === null) {
      name = arg;
    } else {
      process.stderr.write(`info: unexpected argument: ${arg}\n`);
      return 2;
    }
  }
  if (name === null) {
    process.stderr.write("info: missing <name>. Usage: skills info <name>\n");
    return 2;
  }

  const matches = findByName(name, sourceSlug);
  if (matches.length === 0) {
    process.stderr.write(`info: no skill named '${name}' found.\n`);
    return 1;
  }
  if (matches.length > 1) {
    process.stderr.write(
      `info: '${name}' is ambiguous across sources (${matches.map((m) => m.source).join(", ")}). ` +
        `Re-run with --source=<slug>.\n`,
    );
    return 1;
  }
  const hit = matches[0];
  if (hit === undefined) {
    process.stderr.write("info: internal error: matches[0] missing.\n");
    return 1;
  }

  let body: string;
  try {
    body = readFileSync(hit.skillMdPath, "utf8");
  } catch (err) {
    process.stderr.write(`info: failed to read ${hit.skillMdPath}: ${(err as Error).message}\n`);
    return 1;
  }
  process.stdout.write(`# source: ${hit.source}\n`);
  process.stdout.write(`# path:   ${hit.skillMdPath}\n`);
  process.stdout.write("\n");
  process.stdout.write(body);
  if (!body.endsWith("\n")) {
    process.stdout.write("\n");
  }
  return 0;
}
