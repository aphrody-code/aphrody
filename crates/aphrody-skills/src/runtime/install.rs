// SPDX-License-Identifier: Apache-2.0
//
// Skill installation engine — copies or symlinks a skill directory into one
// or more agent target directories (project-scoped or global).
//
// Fuses:
//   1. `manifest.rs::ensure_home_exists` (XDG / env-var path resolution)
//   2. `sources.rs::SourceSlug::ClaudeCode` (agent target whitelist)
//   3. Open-design installer pattern: copy SKILL.md + references/* + scripts/*
//      preserving the directory layout.
//
// Agent target whitelist (per memory `project_aphrody_providers_3only`):
//   * claude-code → `<base>/.claude/skills/<name>/`
//   * gemini      → `<base>/.gemini/skills/<name>/`
//   * antigravity → `<base>/.antigravity/skills/<name>/`

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Symlink or full copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstallMethod {
    Symlink,
    Copy,
}

impl Default for InstallMethod {
    fn default() -> Self {
        Self::Symlink
    }
}

/// Project-local (cwd) or user-global (home).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstallScope {
    Project,
    Global,
}

impl Default for InstallScope {
    fn default() -> Self {
        Self::Project
    }
}

/// Agents we support installing skills for. Whitelist of 3 (claude-code,
/// gemini, antigravity) per memory `project_aphrody_providers_3only`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AgentTarget {
    ClaudeCode,
    Gemini,
    Antigravity,
}

impl AgentTarget {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude-code",
            Self::Gemini => "gemini",
            Self::Antigravity => "antigravity",
        }
    }

    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "claude-code" | "claude" | "cc" => Some(Self::ClaudeCode),
            "gemini" | "gemini-cli" => Some(Self::Gemini),
            "antigravity" | "ag" => Some(Self::Antigravity),
            _ => None,
        }
    }

    #[must_use]
    pub fn all() -> &'static [Self] {
        &[Self::ClaudeCode, Self::Gemini, Self::Antigravity]
    }

    /// Subdir under the scope base where this agent reads its skills.
    #[must_use]
    pub fn skills_subdir(self) -> &'static str {
        match self {
            Self::ClaudeCode => ".claude/skills",
            Self::Gemini => ".gemini/skills",
            Self::Antigravity => ".antigravity/skills",
        }
    }
}

/// Compute the on-disk skills directory for `(agent, scope)`.
#[must_use]
pub fn target_dir_for(agent: AgentTarget, scope: InstallScope) -> PathBuf {
    let base = match scope {
        InstallScope::Project => env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        InstallScope::Global => env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(".")),
    };
    base.join(agent.skills_subdir())
}

/// Install one skill (whole directory) into each agent target. Returns the
/// list of final destination paths actually written. Symlink installs that
/// fail (e.g. on Windows without developer mode) fall back to a copy.
pub fn install_skill(
    skill_path: &Path,
    target_agents: &[AgentTarget],
    scope: InstallScope,
    method: InstallMethod,
) -> Result<Vec<PathBuf>> {
    if !skill_path.exists() {
        anyhow::bail!("skill source `{}` does not exist", skill_path.display());
    }
    if !skill_path.is_dir() {
        anyhow::bail!(
            "skill source `{}` is not a directory (expected a folder containing SKILL.md)",
            skill_path.display()
        );
    }
    let name = skill_path
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .ok_or_else(|| anyhow::anyhow!("skill path has no basename"))?;

    let mut written = Vec::with_capacity(target_agents.len());
    for &agent in target_agents {
        let dir = target_dir_for(agent, scope).join(&name);
        if let Some(parent) = dir.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("mkdir {}", parent.display()))?;
        }
        // Remove pre-existing entry to keep semantics idempotent.
        if dir.exists() || is_symlink(&dir) {
            remove_path(&dir).with_context(|| format!("remove existing {}", dir.display()))?;
        }
        match method {
            InstallMethod::Symlink => match make_symlink(skill_path, &dir) {
                Ok(()) => {}
                Err(_) => copy_dir_recursive(skill_path, &dir).with_context(|| {
                    format!(
                        "symlink failed, falling back to copy {} → {}",
                        skill_path.display(),
                        dir.display()
                    )
                })?,
            },
            InstallMethod::Copy => copy_dir_recursive(skill_path, &dir).with_context(|| {
                format!("copy {} → {}", skill_path.display(), dir.display())
            })?,
        }
        written.push(dir);
    }
    Ok(written)
}

fn is_symlink(p: &Path) -> bool {
    fs::symlink_metadata(p)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

fn remove_path(p: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(p)
        .with_context(|| format!("stat {}", p.display()))?;
    if meta.file_type().is_symlink() || meta.file_type().is_file() {
        fs::remove_file(p)
            .with_context(|| format!("remove file/link {}", p.display()))?;
    } else {
        fs::remove_dir_all(p)
            .with_context(|| format!("remove dir {}", p.display()))?;
    }
    Ok(())
}

#[cfg(unix)]
fn make_symlink(src: &Path, dst: &Path) -> Result<()> {
    std::os::unix::fs::symlink(src, dst)
        .with_context(|| format!("symlink {} → {}", src.display(), dst.display()))
}

#[cfg(windows)]
fn make_symlink(src: &Path, dst: &Path) -> Result<()> {
    // Requires SeCreateSymbolicLinkPrivilege or Developer Mode. Caller handles
    // fallback to copy when this errors.
    std::os::windows::fs::symlink_dir(src, dst)
        .with_context(|| format!("symlink_dir {} → {}", src.display(), dst.display()))
}

#[cfg(not(any(unix, windows)))]
fn make_symlink(_src: &Path, _dst: &Path) -> Result<()> {
    anyhow::bail!("symlinks unsupported on this platform")
}

pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)
        .with_context(|| format!("mkdir {}", dst.display()))?;
    for entry in fs::read_dir(src).with_context(|| format!("readdir {}", src.display()))? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else if ty.is_file() {
            fs::copy(&from, &to)
                .with_context(|| format!("copy {} → {}", from.display(), to.display()))?;
        }
        // Symlinks under the source are intentionally skipped to keep the
        // installed copy hermetic.
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn mk_skill(root: &Path, name: &str) -> PathBuf {
        let dir = root.join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: tmp\n---\nbody\n"),
        )
        .unwrap();
        let refs = dir.join("references");
        fs::create_dir(&refs).unwrap();
        fs::write(refs.join("note.txt"), "hi").unwrap();
        dir
    }

    #[test]
    fn agent_target_roundtrip() {
        for a in AgentTarget::all() {
            assert_eq!(AgentTarget::from_str(a.as_str()), Some(*a));
        }
        assert_eq!(AgentTarget::from_str("claude"), Some(AgentTarget::ClaudeCode));
        assert_eq!(AgentTarget::from_str("nope"), None);
    }

    #[test]
    fn copy_install_writes_files_in_each_target() {
        let td = tempdir().unwrap();
        let src = mk_skill(td.path(), "alpha");
        let proj = td.path().join("project");
        fs::create_dir_all(&proj).unwrap();
        let prev = env::current_dir().unwrap();
        env::set_current_dir(&proj).unwrap();
        let dests = install_skill(
            &src,
            &[AgentTarget::ClaudeCode, AgentTarget::Gemini],
            InstallScope::Project,
            InstallMethod::Copy,
        )
        .unwrap();
        env::set_current_dir(prev).unwrap();
        assert_eq!(dests.len(), 2);
        for d in dests {
            assert!(d.join("SKILL.md").is_file(), "missing SKILL.md in {d:?}");
            assert!(
                d.join("references").join("note.txt").is_file(),
                "missing references/note.txt in {d:?}"
            );
        }
    }

    #[test]
    fn copy_dir_recursive_handles_nested() {
        let td = tempdir().unwrap();
        let src = td.path().join("src");
        let nested = src.join("a").join("b");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("f.txt"), "hi").unwrap();
        let dst = td.path().join("dst");
        copy_dir_recursive(&src, &dst).unwrap();
        assert!(dst.join("a").join("b").join("f.txt").is_file());
    }

    #[test]
    fn install_errors_when_source_missing() {
        let td = tempdir().unwrap();
        let missing = td.path().join("does-not-exist");
        let res = install_skill(
            &missing,
            &[AgentTarget::ClaudeCode],
            InstallScope::Project,
            InstallMethod::Copy,
        );
        assert!(res.is_err());
    }

    #[test]
    #[cfg(unix)]
    fn symlink_install_on_unix() {
        let td = tempdir().unwrap();
        let src = mk_skill(td.path(), "beta");
        let proj = td.path().join("p2");
        fs::create_dir_all(&proj).unwrap();
        let prev = env::current_dir().unwrap();
        env::set_current_dir(&proj).unwrap();
        let dests = install_skill(
            &src,
            &[AgentTarget::ClaudeCode],
            InstallScope::Project,
            InstallMethod::Symlink,
        )
        .unwrap();
        env::set_current_dir(prev).unwrap();
        assert_eq!(dests.len(), 1);
        let meta = fs::symlink_metadata(&dests[0]).unwrap();
        assert!(meta.file_type().is_symlink());
    }

    #[test]
    fn target_dir_for_uses_agent_subdir() {
        let proj = target_dir_for(AgentTarget::ClaudeCode, InstallScope::Project);
        assert!(proj.ends_with(".claude/skills") || proj.ends_with(".claude\\skills"));
        let gem = target_dir_for(AgentTarget::Gemini, InstallScope::Project);
        assert!(gem.ends_with(".gemini/skills") || gem.ends_with(".gemini\\skills"));
    }
}
