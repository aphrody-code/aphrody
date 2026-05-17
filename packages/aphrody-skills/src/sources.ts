// SPDX-License-Identifier: Apache-2.0
//
// @aphrody/skills — source registry.
//
// Each source describes ONE upstream catalog of SKILL.md files. The package
// does NOT vendor any of the SKILL.md content — `roots()` resolves filesystem
// paths at runtime so `skills sync` always mirrors the latest upstream.

import { existsSync, readdirSync, statSync } from "node:fs";
import { join, resolve } from "node:path";

export type SourceSlug =
  | "open-design"
  | "openclaw"
  | "claude-code"
  | "gemini-cli"
  | "vercel-labs"
  | "vercel-skills"
  | "vercel-open-agents"
  | "google-labs";

export interface SourceSpec {
  readonly slug: SourceSlug;
  readonly label: string;
  readonly schema: "open-design" | "vercel" | "aphrody" | "gemini";
  readonly upstream: string;
  /** Environment variable name that overrides the default path. */
  readonly env: string;
  /** Default candidate paths, tried in order. First existing wins. */
  readonly defaults: readonly string[];
}

const worktreeRoot = (): string => {
  // Allow override for non-Windows hosts. Default to the canonical
  // aphrody dev layout: `C:/worktree` on Windows, `~/worktree` elsewhere.
  const override = process.env["APHRODY_WORKTREE"];
  if (override && override.length > 0) {
    return override;
  }
  if (process.platform === "win32") {
    return "C:/worktree";
  }
  const home = process.env["HOME"] ?? process.env["USERPROFILE"] ?? ".";
  return join(home, "worktree");
};

const projectRoot = (): string => {
  // Walk upwards from cwd until we find a package.json with "workspaces"
  // or a `.git` directory. Fall back to cwd.
  let dir = resolve(process.cwd());
  for (let i = 0; i < 8; i += 1) {
    if (existsSync(join(dir, ".git")) || existsSync(join(dir, "Cargo.toml"))) {
      return dir;
    }
    const parent = resolve(dir, "..");
    if (parent === dir) {
      break;
    }
    dir = parent;
  }
  return resolve(process.cwd());
};

export const SOURCES: readonly SourceSpec[] = [
  {
    slug: "open-design",
    label: "open-design (nexu-io)",
    schema: "open-design",
    upstream: "https://github.com/nexu-io/open-design",
    env: "APHRODY_OD_SKILLS",
    defaults: [join(worktreeRoot(), "open-design", "skills")],
  },
  {
    slug: "openclaw",
    label: "openclaw",
    schema: "open-design",
    upstream: "https://github.com/openclaw/openclaw",
    env: "APHRODY_OC_SKILLS",
    defaults: [join(worktreeRoot(), "openclaw", ".agents", "skills")],
  },
  {
    slug: "claude-code",
    label: "claude-code (project-local)",
    schema: "aphrody",
    upstream: "https://github.com/anthropics/skills",
    env: "APHRODY_CC_SKILLS",
    defaults: [join(projectRoot(), ".claude", "skills")],
  },
  {
    slug: "gemini-cli",
    label: "gemini-cli",
    schema: "gemini",
    upstream: "https://github.com/google-gemini/gemini-cli",
    env: "APHRODY_GEMINI_SKILLS",
    defaults: [join(worktreeRoot(), "gemini-cli", ".gemini", "skills")],
  },
  {
    slug: "vercel-labs",
    label: "vercel-labs (agent-skills)",
    schema: "vercel",
    upstream: "https://github.com/vercel-labs/agent-skills",
    env: "APHRODY_VERCEL_SKILLS",
    defaults: [join(worktreeRoot(), "vercel-agent-skills", "skills")],
  },
  {
    slug: "vercel-skills",
    label: "vercel-labs (skills)",
    schema: "vercel",
    upstream: "https://github.com/vercel-labs/skills",
    env: "APHRODY_VERCEL_SKILLS_REPO",
    defaults: [join(worktreeRoot(), "vercel-skills", "skills")],
  },
  {
    slug: "vercel-open-agents",
    label: "vercel-labs (open-agents)",
    schema: "vercel",
    upstream: "https://github.com/vercel-labs/open-agents",
    env: "APHRODY_VERCEL_OPEN_AGENTS",
    defaults: [join(worktreeRoot(), "open-agents", ".agents", "skills")],
  },
  {
    slug: "google-labs",
    label: "google-labs-skills",
    schema: "open-design",
    upstream: "https://github.com/google/labs-skills",
    env: "APHRODY_GOOGLE_SKILLS",
    defaults: [join(worktreeRoot(), "google-labs-skills", "skills")],
  },
];

/** Resolve the filesystem root for a single source, or `null` if not present. */
export function resolveSourceRoot(spec: SourceSpec): string | null {
  const override = process.env[spec.env];
  if (override && override.length > 0) {
    return existsSync(override) ? override : null;
  }
  for (const candidate of spec.defaults) {
    if (existsSync(candidate)) {
      return candidate;
    }
  }
  return null;
}

/** Result of enumerating one source. */
export interface DiscoveredSkill {
  readonly source: SourceSlug;
  readonly name: string;
  readonly skillMdPath: string;
  readonly skillDir: string;
}

/** List every SKILL.md under a source. Returns empty array if root missing. */
export function discoverSkills(spec: SourceSpec): DiscoveredSkill[] {
  const root = resolveSourceRoot(spec);
  if (root === null) {
    return [];
  }
  const out: DiscoveredSkill[] = [];
  let entries: string[];
  try {
    entries = readdirSync(root);
  } catch {
    return out;
  }
  for (const entry of entries) {
    const dir = join(root, entry);
    let st;
    try {
      st = statSync(dir);
    } catch {
      continue;
    }
    if (!st.isDirectory()) {
      continue;
    }
    const skillMd = join(dir, "SKILL.md");
    if (!existsSync(skillMd)) {
      continue;
    }
    out.push({
      source: spec.slug,
      name: entry,
      skillMdPath: skillMd,
      skillDir: dir,
    });
  }
  return out;
}

/** Apply `APHRODY_SKILLS_SOURCES` allowlist to the registry. */
export function activeSources(): readonly SourceSpec[] {
  const raw = process.env["APHRODY_SKILLS_SOURCES"];
  if (!raw || raw.trim().length === 0) {
    return SOURCES;
  }
  const allow = new Set(
    raw
      .split(",")
      .map((s) => s.trim())
      .filter((s) => s.length > 0),
  );
  return SOURCES.filter((s) => allow.has(s.slug));
}

/** Look up a source spec by slug. */
export function sourceBySlug(slug: string): SourceSpec | null {
  return SOURCES.find((s) => s.slug === slug) ?? null;
}
