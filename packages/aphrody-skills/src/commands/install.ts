// SPDX-License-Identifier: Apache-2.0
//
// `skills install <name>` — copy ONE skill into $APHRODY_SKILLS_HOME.

import { copyFileSync, mkdirSync, statSync } from "node:fs";
import { join } from "node:path";

import { ensureHomeExists, upsertEntry } from "../manifest.ts";
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

export function runInstall(argv: readonly string[]): number {
  let name: string | null = null;
  let sourceSlug: string | undefined;
  for (const arg of argv) {
    if (arg === "--help" || arg === "-h") {
      process.stdout.write(
        "Usage: skills install <name> [--source=<slug>]\n" +
          "       Copies one SKILL.md into $APHRODY_SKILLS_HOME and records it in the manifest.\n",
      );
      return 0;
    }
    if (arg.startsWith("--source=")) {
      sourceSlug = arg.slice("--source=".length);
    } else if (name === null) {
      name = arg;
    } else {
      process.stderr.write(`install: unexpected argument: ${arg}\n`);
      return 2;
    }
  }
  if (name === null) {
    process.stderr.write("install: missing <name>. Usage: skills install <name>\n");
    return 2;
  }

  const matches = findByName(name, sourceSlug);
  if (matches.length === 0) {
    process.stderr.write(`install: no skill named '${name}' found.\n`);
    return 1;
  }
  if (matches.length > 1) {
    process.stderr.write(
      `install: '${name}' is ambiguous (${matches.map((m) => m.source).join(", ")}). ` +
        `Re-run with --source=<slug>.\n`,
    );
    return 1;
  }
  const hit = matches[0];
  if (hit === undefined) {
    process.stderr.write("install: internal error: matches[0] missing.\n");
    return 1;
  }

  const home = ensureHomeExists();
  const targetDir = join(home, hit.source, hit.name);
  const targetPath = join(targetDir, "SKILL.md");
  mkdirSync(targetDir, { recursive: true });
  copyFileSync(hit.skillMdPath, targetPath);
  let size = 0;
  try {
    size = statSync(hit.skillMdPath).size;
  } catch {
    size = 0;
  }
  upsertEntry({
    name: hit.name,
    source: hit.source,
    source_path: hit.skillMdPath,
    copied_at: new Date().toISOString(),
    size_bytes: size,
  });
  process.stdout.write(`installed ${hit.source}/${hit.name} -> ${targetPath} (${size} bytes)\n`);
  return 0;
}
