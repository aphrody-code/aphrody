// SPDX-License-Identifier: Apache-2.0
//
// Discovery — enumerate SKILL.md files under one or more source roots.
//
// Port of `discoverSkills` from packages/aphrody-skills/src/sources.ts plus
// a generic `discover_skills_in_path` helper used by the `agent-skills list
// <PATH>` subcommand.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use walkdir::WalkDir;

use crate::sources::{resolve_source_root, SourceSlug, SourceSpec};

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
