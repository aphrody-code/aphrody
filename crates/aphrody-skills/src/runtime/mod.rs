// SPDX-License-Identifier: Apache-2.0
//
// aphrody-skills-runtime — pure-Rust port of @aphrody/skills (TS, 1169 LoC).
//
// Public API mirrors the TS package: a SKILL.md `Frontmatter` parser, a
// filesystem-resolved `Source` registry (6 upstream catalogs), a discovery
// walker, a JSON-backed install manifest, and a schema validator.
//
// All modules are sync, allocation-bounded by the number of skills on disk,
// and AOT-friendly (no reflection, all (de)serialization via serde).

pub mod frontmatter;
pub mod sources;
pub mod manifest;
pub mod discover;
pub mod plugin_manifest;
pub mod validate;

pub use frontmatter::{parse_frontmatter, split_frontmatter, FrontMatter, OdMeta};
pub use sources::{
    active_sources, resolve_source_root, source_by_slug, SourceSchema, SourceSlug, SourceSpec,
    SOURCES,
};
pub use discover::{
    agent_skill_roots, discover_agent_skills, discover_plugin_skills, discover_skills,
    DiscoveredSkill,
};
pub use plugin_manifest::plugin_skill_dirs;
pub use manifest::{
    ensure_home_exists, manifest_path, read_manifest, skills_home, upsert_entry, write_manifest,
    Manifest, ManifestEntry,
};
pub use validate::{validate_frontmatter, ValidationError};
