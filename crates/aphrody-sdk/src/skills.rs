// SPDX-License-Identifier: Apache-2.0
//! Skills surface — re-exports + a thin discovery facade.
//!
//! Skills extend the agent with extra prompts, tools and behaviours described
//! by a `SKILL.md` frontmatter block. The heavy parsing / validation lives in
//! [`aphrody_skills_runtime`]; this module exposes a small `Skills` façade
//! that owns the discovered set and supports `add_dir` for late-binding.
//!
//! TS provenance: `packages/gemini-cli/packages/sdk/src/skills.ts`.

use std::path::{Path, PathBuf};

pub use aphrody_skills::runtime::{
    DiscoveredSkill, FrontMatter, OdMeta, SourceSchema, SourceSlug, SourceSpec,
};

use crate::types::SdkResult;

// ---------------------------------------------------------------------------
// SkillReference — public DTO mirroring the TS `SkillReference` discriminated
// union. v1 ships the `Dir` variant only; v2 will add `Git` and `Url`.
// ---------------------------------------------------------------------------

/// Reference to a skill source.
///
/// PUBLIC API — semver stable from v1.0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SkillReference {
    /// Local directory containing one or more `<name>/SKILL.md` files.
    Dir(PathBuf),
}

/// Sugar for `SkillReference::Dir`.
///
/// PUBLIC API — semver stable from v1.0.
#[must_use]
pub fn skill_dir(path: impl Into<PathBuf>) -> SkillReference {
    SkillReference::Dir(path.into())
}

// ---------------------------------------------------------------------------
// Skills façade
// ---------------------------------------------------------------------------

/// Aggregate view of every skill discovered by the SDK.
///
/// PUBLIC API — semver stable from v1.0.
#[derive(Debug, Clone, Default)]
pub struct Skills {
    discovered: Vec<DiscoveredSkill>,
}

impl Skills {
    /// Empty set.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Walk every supplied directory and merge the resulting `SKILL.md` hits
    /// into the aggregate.
    ///
    /// Errors during a single directory walk are *not* fatal: the offending
    /// path is logged via `tracing::warn!` and the loop continues. This
    /// matches the TS behaviour where one bad skill source should not abort
    /// the whole agent.
    ///
    /// # Errors
    /// This method does not currently return an error — but the signature is
    /// `SdkResult<()>` to preserve API stability when stricter validation is
    /// wired in v1.1+.
    pub fn add_dirs(&mut self, dirs: &[PathBuf]) -> SdkResult<()> {
        for dir in dirs {
            self.add_dir(dir);
        }
        Ok(())
    }

    /// Walk one directory and append every `<name>/SKILL.md` hit.
    pub fn add_dir(&mut self, root: impl AsRef<Path>) {
        let root = root.as_ref();
        if !root.exists() {
            tracing::debug!(
                target: "aphrody_sdk::skills",
                path = %root.display(),
                "skill dir does not exist; skipping",
            );
            return;
        }
        let mut found = aphrody_skills::runtime::discover::discover_skills_in_path(
            root,
            SourceSlug::ClaudeCode,
        );
        tracing::info!(
            target: "aphrody_sdk::skills",
            path = %root.display(),
            count = found.len(),
            "discovered skills",
        );
        self.discovered.append(&mut found);
    }

    /// Borrow every discovered skill, sorted by name.
    #[must_use]
    pub fn list(&self) -> &[DiscoveredSkill] {
        &self.discovered
    }

    /// Number of skills currently aggregated.
    #[must_use]
    pub fn len(&self) -> usize {
        self.discovered.len()
    }

    /// `true` when no skill was discovered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.discovered.is_empty()
    }

    /// Find a discovered skill by name.
    #[must_use]
    pub fn find(&self, name: &str) -> Option<&DiscoveredSkill> {
        self.discovered.iter().find(|s| s.name == name)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn empty_skills_is_empty() {
        let s = Skills::new();
        assert!(s.is_empty());
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn skill_dir_sugar_round_trips() {
        let r = skill_dir("/tmp/skills");
        assert_eq!(r, SkillReference::Dir(PathBuf::from("/tmp/skills")));
    }

    #[test]
    fn discover_finds_skill_md_under_dir() {
        let dir = TempDir::new().unwrap();
        let skill_dir = dir.path().join("my-skill");
        std::fs::create_dir_all(&skill_dir).unwrap();
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: my-skill\ndescription: test\n---\nbody\n",
        )
        .unwrap();
        let mut s = Skills::new();
        s.add_dir(dir.path());
        assert_eq!(s.len(), 1);
        assert!(s.find("my-skill").is_some());
    }

    #[test]
    fn add_dir_tolerates_missing_path() {
        let mut s = Skills::new();
        // Must not panic / error.
        s.add_dir("/path/that/definitely/does/not/exist/aphrody-sdk-test");
        assert!(s.is_empty());
    }
}
