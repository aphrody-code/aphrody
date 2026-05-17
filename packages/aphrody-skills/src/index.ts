// SPDX-License-Identifier: Apache-2.0
//
// @aphrody/skills — public TS API (for `import { ... } from "@aphrody/skills"`).

export type { SourceSlug, SourceSpec, DiscoveredSkill } from "./sources.ts";
export {
  SOURCES,
  activeSources,
  discoverSkills,
  resolveSourceRoot,
  sourceBySlug,
} from "./sources.ts";

export type { FrontMatter } from "./frontmatter.ts";
export { parseFrontMatter, splitFrontMatter } from "./frontmatter.ts";

export type { Manifest, ManifestEntry } from "./manifest.ts";
export {
  ensureHomeExists,
  manifestPath,
  readManifest,
  skillsHome,
  upsertEntry,
  writeManifest,
} from "./manifest.ts";
