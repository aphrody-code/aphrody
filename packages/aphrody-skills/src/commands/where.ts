// SPDX-License-Identifier: Apache-2.0
//
// `skills where <name>` — print the absolute path of one skill's SKILL.md.

import { activeSources, discoverSkills } from "../sources.ts";

export function runWhere(argv: readonly string[]): number {
  let name: string | null = null;
  let sourceSlug: string | undefined;
  for (const arg of argv) {
    if (arg === "--help" || arg === "-h") {
      process.stdout.write(
        "Usage: skills where <name> [--source=<slug>]\n" +
          "       Prints the source absolute path of the SKILL.md for the named skill.\n",
      );
      return 0;
    }
    if (arg.startsWith("--source=")) {
      sourceSlug = arg.slice("--source=".length);
    } else if (name === null) {
      name = arg;
    } else {
      process.stderr.write(`where: unexpected argument: ${arg}\n`);
      return 2;
    }
  }
  if (name === null) {
    process.stderr.write("where: missing <name>. Usage: skills where <name>\n");
    return 2;
  }

  let found = 0;
  for (const spec of activeSources()) {
    if (sourceSlug && spec.slug !== sourceSlug) {
      continue;
    }
    for (const sk of discoverSkills(spec)) {
      if (sk.name === name) {
        process.stdout.write(`${spec.slug}\t${sk.skillMdPath}\n`);
        found += 1;
      }
    }
  }
  if (found === 0) {
    process.stderr.write(`where: no skill named '${name}' found.\n`);
    return 1;
  }
  return 0;
}
