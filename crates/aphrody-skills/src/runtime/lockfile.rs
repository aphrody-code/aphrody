// SPDX-License-Identifier: Apache-2.0
//
// Persistent install lockfile at `~/.aphrody/skills.lock` (TOML).
//
// Fuses:
//   1. `manifest.rs::Manifest` (envelope + upsert-by-key pattern)
//   2. `cache.rs::cache_dir` (XDG-aware path resolution)
//   3. open-design daemon DB schema for installed-skill rows (name + source +
//      install_path + checksum + installed_at).
//
// Tracks what is installed where so `agent-skills add --uninstall` can later
// reverse the operation symmetrically.

use std::{env, fs, path::PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledSkill {
    pub name: String,
    pub source_slug: String,
    pub source_path: PathBuf,
    pub installed_to: Vec<PathBuf>,
    pub sha256: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillsLock {
    #[serde(default)]
    pub version: u32,
    #[serde(default, rename = "skill")]
    pub skills: Vec<InstalledSkill>,
}

impl SkillsLock {
    pub fn new() -> Self {
        Self { version: 1, skills: Vec::new() }
    }
}

/// Resolve the lockfile path. Honours `APHRODY_SKILLS_LOCK` first, then
/// falls back to `~/.aphrody/skills.lock`.
#[must_use]
pub fn lockfile_path() -> PathBuf {
    if let Ok(o) = env::var("APHRODY_SKILLS_LOCK") {
        if !o.is_empty() {
            return PathBuf::from(o);
        }
    }
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join(".aphrody").join("skills.lock")
}

pub fn read_lock() -> Result<SkillsLock> {
    let p = lockfile_path();
    if !p.exists() {
        return Ok(SkillsLock::new());
    }
    let raw = fs::read_to_string(&p)
        .with_context(|| format!("read lockfile {}", p.display()))?;
    parse_lock(&raw).with_context(|| format!("parse lockfile {}", p.display()))
}

pub fn parse_lock(raw: &str) -> Result<SkillsLock> {
    let mut lock: SkillsLock = toml::from_str(raw).context("decode TOML lockfile")?;
    if lock.version == 0 {
        lock.version = 1;
    }
    Ok(lock)
}

pub fn write_lock(lock: &SkillsLock) -> Result<()> {
    let p = lockfile_path();
    if let Some(parent) = p.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create lockfile parent {}", parent.display()))?;
        }
    }
    let body = toml::to_string_pretty(lock).context("encode lockfile as TOML")?;
    fs::write(&p, body).with_context(|| format!("write lockfile {}", p.display()))
}

/// Upsert one installed skill (matched by `name` + `source_slug`). Returns
/// `true` if an existing entry was replaced.
pub fn upsert_skill(lock: &mut SkillsLock, entry: InstalledSkill) -> bool {
    let key = (entry.name.clone(), entry.source_slug.clone());
    let prior_len = lock.skills.len();
    lock.skills
        .retain(|e| !(e.name == key.0 && e.source_slug == key.1));
    let replaced = lock.skills.len() < prior_len;
    lock.skills.push(entry);
    lock.skills.sort_by(|a, b| {
        if a.source_slug == b.source_slug {
            a.name.cmp(&b.name)
        } else {
            a.source_slug.cmp(&b.source_slug)
        }
    });
    replaced
}

/// Remove an entry (matched by `name` + `source_slug`). Returns the removed
/// entry, or `None` if not found.
pub fn remove_skill(
    lock: &mut SkillsLock,
    name: &str,
    source_slug: &str,
) -> Option<InstalledSkill> {
    let idx = lock
        .skills
        .iter()
        .position(|e| e.name == name && e.source_slug == source_slug)?;
    Some(lock.skills.remove(idx))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk(name: &str, source: &str, sha: &str) -> InstalledSkill {
        InstalledSkill {
            name: name.to_string(),
            source_slug: source.to_string(),
            source_path: PathBuf::from(format!("/src/{name}/SKILL.md")),
            installed_to: vec![PathBuf::from(format!("/agents/claude-code/skills/{name}"))],
            sha256: sha.to_string(),
        }
    }

    #[test]
    fn roundtrip_via_toml() {
        let mut lock = SkillsLock::new();
        upsert_skill(&mut lock, mk("alpha", "vercel-labs", "deadbeef"));
        upsert_skill(&mut lock, mk("beta", "anthropics", "cafebabe"));
        let raw = toml::to_string_pretty(&lock).unwrap();
        let parsed = parse_lock(&raw).unwrap();
        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.skills.len(), 2);
        assert_eq!(parsed.skills[0].name, "beta");
        assert_eq!(parsed.skills[1].name, "alpha");
    }

    #[test]
    fn upsert_dedups_by_name_plus_source() {
        let mut lock = SkillsLock::new();
        assert!(!upsert_skill(&mut lock, mk("alpha", "vercel-labs", "v1")));
        assert!(upsert_skill(&mut lock, mk("alpha", "vercel-labs", "v2")));
        assert_eq!(lock.skills.len(), 1);
        assert_eq!(lock.skills[0].sha256, "v2");
        // Different source → not deduped.
        assert!(!upsert_skill(&mut lock, mk("alpha", "anthropics", "v3")));
        assert_eq!(lock.skills.len(), 2);
    }

    #[test]
    fn remove_skill_finds_and_returns_entry() {
        let mut lock = SkillsLock::new();
        upsert_skill(&mut lock, mk("alpha", "vercel-labs", "v1"));
        upsert_skill(&mut lock, mk("beta", "vercel-labs", "v2"));
        let removed = remove_skill(&mut lock, "alpha", "vercel-labs").unwrap();
        assert_eq!(removed.name, "alpha");
        assert_eq!(lock.skills.len(), 1);
        assert!(remove_skill(&mut lock, "alpha", "vercel-labs").is_none());
    }

    #[test]
    fn parse_lock_accepts_empty() {
        let parsed = parse_lock("").unwrap();
        assert_eq!(parsed.version, 1);
        assert!(parsed.skills.is_empty());
    }
}
