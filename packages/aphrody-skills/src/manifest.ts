// SPDX-License-Identifier: Apache-2.0
//
// @aphrody/skills — local manifest persisted under `$APHRODY_SKILLS_HOME/manifest.json`.

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";

import type { SourceSlug } from "./sources.ts";

export interface ManifestEntry {
  readonly name: string;
  readonly source: SourceSlug;
  readonly source_path: string;
  readonly copied_at: string;
  readonly size_bytes: number;
}

export interface Manifest {
  readonly version: 1;
  readonly home: string;
  readonly updated_at: string;
  readonly entries: ManifestEntry[];
}

export function skillsHome(): string {
  const override = process.env["APHRODY_SKILLS_HOME"];
  if (override && override.length > 0) {
    return override;
  }
  const home = process.env["HOME"] ?? process.env["USERPROFILE"] ?? ".";
  return join(home, ".aphrody", "skills");
}

export function manifestPath(): string {
  return join(skillsHome(), "manifest.json");
}

export function ensureHomeExists(): string {
  const dir = skillsHome();
  if (!existsSync(dir)) {
    mkdirSync(dir, { recursive: true });
  }
  return dir;
}

export function readManifest(): Manifest {
  const path = manifestPath();
  if (!existsSync(path)) {
    return {
      version: 1,
      home: skillsHome(),
      updated_at: new Date().toISOString(),
      entries: [],
    };
  }
  try {
    const raw = readFileSync(path, "utf8");
    const parsed = JSON.parse(raw) as Manifest;
    if (parsed.version !== 1 || !Array.isArray(parsed.entries)) {
      throw new Error("manifest schema mismatch");
    }
    return parsed;
  } catch (err) {
    throw new Error(`failed to read manifest at ${path}: ${(err as Error).message}`);
  }
}

export function writeManifest(m: Manifest): void {
  const path = manifestPath();
  const parent = dirname(path);
  if (!existsSync(parent)) {
    mkdirSync(parent, { recursive: true });
  }
  writeFileSync(path, `${JSON.stringify(m, null, 2)}\n`, "utf8");
}

/** Upsert one entry (matched by name+source) and persist. */
export function upsertEntry(entry: ManifestEntry): Manifest {
  const current = readManifest();
  const filtered = current.entries.filter(
    (e) => !(e.name === entry.name && e.source === entry.source),
  );
  filtered.push(entry);
  filtered.sort((a, b) =>
    a.source === b.source
      ? a.name.localeCompare(b.name)
      : a.source.localeCompare(b.source),
  );
  const next: Manifest = {
    version: 1,
    home: skillsHome(),
    updated_at: new Date().toISOString(),
    entries: filtered,
  };
  writeManifest(next);
  return next;
}
