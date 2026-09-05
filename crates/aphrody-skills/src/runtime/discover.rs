// SPDX-License-Identifier: Apache-2.0
//
// Discovery — enumerate SKILL.md files under one or more source roots.
//
// Port of `discoverSkills` from packages/aphrody-skills/src/sources.ts plus
// a generic `discover_skills_in_path` helper used by the `agent-skills list
// <PATH>` subcommand.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::runtime::plugin_manifest::plugin_skill_dirs;
use crate::runtime::sources::{resolve_source_root, SourceSlug, SourceSpec};

/// One SKILL.md hit on disk.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DiscoveredSkill {
    pub source: SourceSlug,
    pub name: String,
    pub skill_md_path: PathBuf,
    pub skill_dir: PathBuf,
}

/// Enumerate every `<root>/<name>/SKILL.md` for a single source. Returns
/// an empty vector if the source root is missing.
#[must_use]
pub fn discover_skills(spec: &SourceSpec) -> Vec<DiscoveredSkill> {
    let Some(root) = resolve_source_root(spec) else {
        return Vec::new();
    };
    discover_skills_in_path(&root, spec.slug)
}

/// Enumerate `<root>/<name>/SKILL.md` for an arbitrary path. Used by
/// `agent-skills list <PATH>` and tests.
#[must_use]
pub fn discover_skills_in_path(root: &Path, slug: SourceSlug) -> Vec<DiscoveredSkill> {
    let mut out: Vec<DiscoveredSkill> = WalkDir::new(root)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_dir())
        .filter_map(|dir_entry| {
            let dir = dir_entry.into_path();
            let skill_md = dir.join("SKILL.md");
            if !skill_md.is_file() {
                return None;
            }
            let name = dir.file_name()?.to_string_lossy().into_owned();
            Some(DiscoveredSkill {
                source: slug,
                name,
                skill_md_path: skill_md,
                skill_dir: dir,
            })
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Discover skills declared by plugin manifests found under `bases`.
///
/// Resolves each `.claude-plugin/{marketplace,plugin}.json` RELATIVE to its
/// base (never a hardcoded plugin path — the dev plugin location is
/// temporary), then scans the resulting `skills/` directories. Plugin skills
/// are surfaced under the `claude-code` slug. Deduplicated by name across all
/// bases.
#[must_use]
pub fn discover_plugin_skills(bases: &[PathBuf]) -> Vec<DiscoveredSkill> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut out = Vec::new();
    for base in bases {
        for dir in plugin_skill_dirs(base) {
            for hit in discover_skills_in_path(&dir, SourceSlug::ClaudeCode) {
                if seen.insert(hit.name.clone()) {
                    out.push(hit);
                }
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Canonical agent skill directories (project-local + global) for the agents
/// aphrody targets: claude-code, gemini-cli, and Antigravity / the `agy` CLI.
///
/// These are where each agent READS its installed skills (per the open
/// agent-skills directory conventions), distinct from the upstream source
/// registry. Only directories that exist on disk are returned. `.agents/skills`
/// is the dir shared by the Gemini-family CLIs (gemini-cli / antigravity / agy)
/// and is surfaced under the `antigravity` slug.
#[must_use]
pub fn agent_skill_roots() -> Vec<(SourceSlug, PathBuf)> {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from);

    let mut roots: Vec<(SourceSlug, PathBuf)> = vec![
        (SourceSlug::ClaudeCode, cwd.join(".claude").join("skills")),
        (SourceSlug::GeminiCli, cwd.join(".gemini").join("skills")),
        (SourceSlug::Antigravity, cwd.join(".agents").join("skills")),
        (SourceSlug::Antigravity, cwd.join(".antigravity").join("skills")),
    ];
    if let Some(h) = home {
        roots.push((SourceSlug::ClaudeCode, h.join(".claude").join("skills")));
        roots.push((SourceSlug::GeminiCli, h.join(".gemini").join("skills")));
        roots.push((
            SourceSlug::Antigravity,
            h.join(".gemini").join("antigravity").join("skills"),
        ));
        roots.push((
            SourceSlug::Antigravity,
            h.join(".config").join("antigravity").join("skills"),
        ));
    }
    roots.retain(|(_, p)| p.is_dir());
    roots
}

/// Discover skills installed in the canonical agent skill directories
/// ([`agent_skill_roots`]). Deduplicated by `(slug, name)`.
#[must_use]
pub fn discover_agent_skills() -> Vec<DiscoveredSkill> {
    let mut seen: HashSet<(SourceSlug, String)> = HashSet::new();
    let mut out = Vec::new();
    for (slug, root) in agent_skill_roots() {
        for hit in discover_skills_in_path(&root, slug) {
            if seen.insert((slug, hit.name.clone())) {
                out.push(hit);
            }
        }
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn discover_skills_in_temp_dir() {
        let td = tempdir().unwrap();
        let root = td.path();
        // Create 3 skills + 1 dir without SKILL.md to confirm filtering.
        for name in ["alpha", "beta", "gamma"] {
            let d = root.join(name);
            fs::create_dir(&d).unwrap();
            fs::write(d.join("SKILL.md"), "---\nname: x\n---\nbody\n").unwrap();
        }
        fs::create_dir(root.join("not-a-skill")).unwrap();

        let hits = discover_skills_in_path(root, SourceSlug::ClaudeCode);
        let names: Vec<_> = hits.iter().map(|h| h.name.as_str()).collect();
        assert_eq!(names, vec!["alpha", "beta", "gamma"]);
        for h in &hits {
            assert_eq!(h.source, SourceSlug::ClaudeCode);
            assert!(h.skill_md_path.is_file());
        }
    }

    #[test]
    fn discover_skills_returns_empty_for_missing_root() {
        let p = Path::new("/this/path/should/never/exist/xyz");
        let hits = discover_skills_in_path(p, SourceSlug::ClaudeCode);
        assert!(hits.is_empty());
    }
}
