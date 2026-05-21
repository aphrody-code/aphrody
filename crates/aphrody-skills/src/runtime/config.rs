// SPDX-License-Identifier: Apache-2.0
//
// User-defined skills source configuration.
//
// Fuses three sources:
//   1. `sources.rs` (static SourceSpec registry → user-defined ConfiguredSource)
//   2. `manifest.rs::skills_home` (env override + dirs fallback pattern)
//   3. `source_spec.rs::SourceLocation` (polymorphic location enum)
//
// Format (TOML):
//   [[source]]
//   slug = "vercel-agent-skills"
//   git = "vercel-labs/agent-skills"
//
//   [[source]]
//   slug = "claude-code-skills"
//   git = "https://github.com/anthropics/skills/tree/main/skills"
//
//   [[source]]
//   slug = "my-local"
//   path = "/abs/path/to/skills"
//
// Resolution order for the config file:
//   1. $APHRODY_SKILLS_CONFIG (explicit override)
//   2. $XDG_CONFIG_HOME/aphrody/skills.toml (or dirs::config_dir())
//   3. (Windows) %APPDATA%/aphrody/skills.toml — same path on Windows via dirs
//   4. Returns empty config if no file is found.

use std::{env, fs, path::PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::runtime::source_spec::{parse_source, SourceLocation};

/// One source declared by the user in `skills.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConfiguredSource {
    pub slug: String,
    pub location: SourceLocation,
}

/// In-memory shape of the user's `skills.toml`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SkillsConfig {
    #[serde(default)]
    pub sources: Vec<ConfiguredSource>,
}

// On-disk shape (separate so we can support both `git=` and `path=` flat
// entries without forcing the user to write a tagged enum).
#[derive(Debug, Deserialize)]
struct RawConfig {
    #[serde(default, rename = "source")]
    sources: Vec<RawSource>,
}

#[derive(Debug, Deserialize)]
struct RawSource {
    slug: String,
    #[serde(default)]
    git: Option<String>,
    #[serde(default)]
    path: Option<String>,
}

impl SkillsConfig {
    /// Resolve the default config file path. Returns the path even if it does
    /// not exist — callers should check `.exists()`.
    #[must_use]
    pub fn default_path() -> PathBuf {
        if let Ok(o) = env::var("APHRODY_SKILLS_CONFIG") {
            if !o.is_empty() {
                return PathBuf::from(o);
            }
        }
        let base = dirs::config_dir().unwrap_or_else(|| {
            let home = env::var("HOME")
                .or_else(|_| env::var("USERPROFILE"))
                .unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".config")
        });
        base.join("aphrody").join("skills.toml")
    }

    /// Load config from the default path. Returns an empty config if the file
    /// is absent. Returns Err on I/O or parse failures.
    pub fn load_from_default_path() -> Result<Self> {
        Self::load_from_path(&Self::default_path())
    }

    pub fn load_from_path(path: &std::path::Path) -> Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path)
            .with_context(|| format!("read skills config {}", path.display()))?;
        Self::parse_toml(&raw)
            .with_context(|| format!("parse skills config {}", path.display()))
    }

    pub fn parse_toml(raw: &str) -> Result<Self> {
        let parsed: RawConfig = toml::from_str(raw).context("decode TOML")?;
        let mut sources = Vec::with_capacity(parsed.sources.len());
        for r in parsed.sources {
            let slug = r.slug.trim().to_string();
            if slug.is_empty() {
                anyhow::bail!("source entry is missing required `slug`");
            }
            let location = match (r.git.as_deref(), r.path.as_deref()) {
                (Some(g), None) => parse_source(g)
                    .with_context(|| format!("parse `git` for source `{slug}`"))?,
                (None, Some(p)) => SourceLocation::Local(PathBuf::from(p)),
                (Some(_), Some(_)) => anyhow::bail!(
                    "source `{slug}` declares both `git` and `path` — choose one"
                ),
                (None, None) => anyhow::bail!(
                    "source `{slug}` is missing `git` or `path`"
                ),
            };
            sources.push(ConfiguredSource { slug, location });
        }
        Ok(Self { sources })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_empty_toml() {
        let cfg = SkillsConfig::parse_toml("").unwrap();
        assert!(cfg.sources.is_empty());
    }

    #[test]
    fn parses_mixed_git_and_path_entries() {
        let raw = r#"
[[source]]
slug = "vercel-agent-skills"
git = "vercel-labs/agent-skills"

[[source]]
slug = "my-local"
path = "/tmp/skills"
"#;
        let cfg = SkillsConfig::parse_toml(raw).unwrap();
        assert_eq!(cfg.sources.len(), 2);
        assert_eq!(cfg.sources[0].slug, "vercel-agent-skills");
        assert!(matches!(
            cfg.sources[0].location,
            SourceLocation::GitHub { ref owner, ref repo, .. }
            if owner == "vercel-labs" && repo == "agent-skills"
        ));
        assert!(matches!(
            cfg.sources[1].location,
            SourceLocation::Local(ref p) if p.to_string_lossy() == "/tmp/skills"
        ));
    }

    #[test]
    fn rejects_source_with_both_git_and_path() {
        let raw = r#"
[[source]]
slug = "bad"
git = "a/b"
path = "/x"
"#;
        assert!(SkillsConfig::parse_toml(raw).is_err());
    }

    #[test]
    fn rejects_source_missing_location() {
        let raw = r#"
[[source]]
slug = "bad"
"#;
        assert!(SkillsConfig::parse_toml(raw).is_err());
    }

    #[test]
    fn default_path_honors_env_override() {
        // SAFETY: scoped per-test, restored at end. Single-threaded test
        // process by default; env-var races are tolerable here.
        let prev = env::var("APHRODY_SKILLS_CONFIG").ok();
        unsafe { env::set_var("APHRODY_SKILLS_CONFIG", "/custom/path/skills.toml") };
        let p = SkillsConfig::default_path();
        assert_eq!(p, PathBuf::from("/custom/path/skills.toml"));
        match prev {
            Some(v) => unsafe { env::set_var("APHRODY_SKILLS_CONFIG", v) },
            None => unsafe { env::remove_var("APHRODY_SKILLS_CONFIG") },
        }
    }

    #[test]
    fn load_from_missing_path_returns_empty() {
        let cfg = SkillsConfig::load_from_path(std::path::Path::new(
            "/this/should/not/exist/skills-xyz.toml",
        ))
        .unwrap();
        assert!(cfg.sources.is_empty());
    }
}
