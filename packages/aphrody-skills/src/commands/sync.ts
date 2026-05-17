// SPDX-License-Identifier: Apache-2.0
//
// `skills sync` — mirror discovered SKILL.md files into $APHRODY_SKILLS_HOME.

import { copyFileSync, mkdirSync, statSync } from "node:fs";
import { join } from "node:path";

import {
  ensureHomeExists,
  readManifest,
  skillsHome,
  upsertEntry,
} from "../manifest.ts";
import {
  activeSources,
  discoverSkills,
  sourceBySlug,
  type DiscoveredSkill,
  type SourceSpec,
} from "../sources.ts";

interface SyncOptions {
  readonly sourceSlug?: string;
  readonly dryRun: boolean;
}

function copyOne(home: string, skill: DiscoveredSkill, dryRun: boolean): number {
  const targetDir = join(home, skill.source, skill.name);
  const targetPath = join(targetDir, "SKILL.md");
  let size = 0;
  try {
    size = statSync(skill.skillMdPath).size;
  } catch {
    size = 0;
  }
  if (dryRun) {
    return size;
  }
  mkdirSync(targetDir, { recursive: true });
  copyFileSync(skill.skillMdPath, targetPath);
  upsertEntry({
    name: skill.name,
    source: skill.source,
    source_path: skill.skillMdPath,
    copied_at: new Date().toISOString(),
    size_bytes: size,
  });
  return size;
}

function runForSource(spec: SourceSpec, home: string, dryRun: boolean): { copied: number; bytes: number } {
  let copied = 0;
  let bytes = 0;
  for (const skill of discoverSkills(spec)) {
    const sz = copyOne(home, skill, dryRun);
    copied += 1;
    bytes += sz;
  }
  return { copied, bytes };
}

export function runSync(argv: readonly string[]): number {
  const opts: { sourceSlug?: string; dryRun: boolean } = { dryRun: false };
  for (const arg of argv) {
    if (arg === "--help" || arg === "-h") {
      process.stdout.write(
        "Usage: skills sync [--source=<slug>] [--dry-run]\n" +
          "       Copies every discovered SKILL.md into $APHRODY_SKILLS_HOME and updates the manifest.\n",
      );
      return 0;
    }
    if (arg === "--dry-run") {
      opts.dryRun = true;
    } else if (arg.startsWith("--source=")) {
      opts.sourceSlug = arg.slice("--source=".length);
    } else {
      process.stderr.write(`sync: unknown argument: ${arg}\n`);
      return 2;
    }
  }

  const home = opts.dryRun ? skillsHome() : ensureHomeExists();

  const sources = opts.sourceSlug
    ? [sourceBySlug(opts.sourceSlug)].filter((s): s is SourceSpec => s !== null)
    : activeSources();

  if (sources.length === 0) {
    process.stderr.write(`sync: no source matches '${opts.sourceSlug ?? "<all>"}'.\n`);
    return 1;
  }

  process.stdout.write(`sync target: ${home}${opts.dryRun ? "  (dry-run)" : ""}\n`);
  let total = 0;
  let totalBytes = 0;
  for (const spec of sources) {
    const { copied, bytes } = runForSource(spec, home, opts.dryRun);
    total += copied;
    totalBytes += bytes;
    process.stdout.write(
      `  ${spec.slug.padEnd(14)}  ${String(copied).padStart(4)} skill(s)  ${String(bytes).padStart(8)} bytes\n`,
    );
  }
  process.stdout.write(`done — ${total} skill(s), ${totalBytes} bytes total.\n`);
  if (!opts.dryRun) {
    try {
      const m = readManifest();
      process.stdout.write(`manifest: ${m.entries.length} entries tracked.\n`);
    } catch (err) {
      process.stderr.write(`sync: manifest read failed: ${(err as Error).message}\n`);
    }
  }
  return 0;
}
