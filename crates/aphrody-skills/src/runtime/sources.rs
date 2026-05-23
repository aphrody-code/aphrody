// SPDX-License-Identifier: Apache-2.0
//
// Source registry — 8 upstream SKILL.md catalogs.
//
// Port of packages/aphrody-skills/src/sources.ts. The registry is statically
// declared (no runtime reflection) and resolves filesystem paths lazily via
// environment overrides and platform-specific defaults.

use std::{env, path::PathBuf};

use serde::{Deserialize, Serialize};

/// All known upstream catalog slugs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceSlug {
    OpenDesign,
    Openclaw,
    ClaudeCode,
    GeminiCli,
    VercelLabs,
    VercelSkills,
    VercelOpenAgents,
    GoogleLabs,
    /// Antigravity / `agy` CLI agent skill dirs (`.agents/skills`,
    /// `.antigravity/skills`, `~/.gemini/antigravity/skills`, …). Discovery-only
    /// label (not a static upstream catalog in `SOURCES`).
    Antigravity,
}

impl SourceSlug {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenDesign => "open-design",
            Self::Openclaw => "openclaw",
            Self::ClaudeCode => "claude-code",
            Self::GeminiCli => "gemini-cli",
            Self::VercelLabs => "vercel-labs",
            Self::VercelSkills => "vercel-skills",
            Self::VercelOpenAgents => "vercel-open-agents",
            Self::GoogleLabs => "google-labs",
            Self::Antigravity => "antigravity",
        }
    }

    #[must_use]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "open-design" => Some(Self::OpenDesign),
            "openclaw" => Some(Self::Openclaw),
            "claude-code" => Some(Self::ClaudeCode),
            "gemini-cli" => Some(Self::GeminiCli),
            "vercel-labs" => Some(Self::VercelLabs),
            "vercel-skills" => Some(Self::VercelSkills),
            "vercel-open-agents" => Some(Self::VercelOpenAgents),
            "google-labs" => Some(Self::GoogleLabs),
            "antigravity" | "agy" | "ag" => Some(Self::Antigravity),
            _ => None,
        }
    }
}

/// Schema family each source conforms to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SourceSchema {
    OpenDesign,
    Vercel,
    Aphrody,
    Gemini,
}

impl SourceSchema {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenDesign => "open-design",
            Self::Vercel => "vercel",
            Self::Aphrody => "aphrody",
            Self::Gemini => "gemini",
        }
    }
}

/// Static description of one upstream catalog.
#[derive(Debug, Clone)]
pub struct SourceSpec {
    pub slug: SourceSlug,
    pub label: &'static str,
    pub schema: SourceSchema,
    pub upstream: &'static str,
    pub env: &'static str,
    /// Function returning the ordered list of default candidate paths.
    /// Indirected so that env-dependent defaults (`HOME`, `APHRODY_WORKTREE`)
    /// are re-evaluated on each call rather than baked in at startup.
    pub defaults: fn() -> Vec<PathBuf>,
}

fn worktree_root() -> PathBuf {
    if let Ok(o) = env::var("APHRODY_WORKTREE") {
        if !o.is_empty() {
            return PathBuf::from(o);
        }
    }
    // 2026-05-21 : ancien défaut hardcodé `C:/worktree` retiré (le dossier
    // n'existe plus). On se rabat sur `<home>/worktree` sur toutes les
    // plateformes ; l'emplacement réel reste surchargeable via APHRODY_WORKTREE.
    let home = env::var("HOME")
        .or_else(|_| env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    PathBuf::from(home).join("worktree")
}

fn project_root() -> PathBuf {
    // Walk upwards from cwd until we find `.git` or `Cargo.toml`.
    let start = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut dir = start.clone();
    for _ in 0..8 {
        if dir.join(".git").exists() || dir.join("Cargo.toml").exists() {
            return dir;
        }
        match dir.parent() {
            Some(p) if p != dir => dir = p.to_path_buf(),
            _ => break,
        }
    }
    start
}

fn open_design_defaults() -> Vec<PathBuf> {
    vec![worktree_root().join("open-design").join("skills")]
}
fn openclaw_defaults() -> Vec<PathBuf> {
    vec![worktree_root().join("openclaw").join(".agents").join("skills")]
}
fn claude_code_defaults() -> Vec<PathBuf> {
    vec![project_root().join(".claude").join("skills")]
}
fn gemini_defaults() -> Vec<PathBuf> {
    vec![worktree_root().join("gemini-cli").join(".gemini").join("skills")]
}
fn vercel_labs_defaults() -> Vec<PathBuf> {
    vec![worktree_root().join("vercel-agent-skills").join("skills")]
}
fn vercel_skills_defaults() -> Vec<PathBuf> {
    vec![worktree_root().join("vercel-skills").join("skills")]
}
fn vercel_open_agents_defaults() -> Vec<PathBuf> {
    vec![worktree_root().join("open-agents").join(".agents").join("skills")]
}
fn google_labs_defaults() -> Vec<PathBuf> {
    vec![worktree_root().join("google-labs-skills").join("skills")]
}

/// Static registry of all 8 sources (mirrors `SOURCES` in TS).
pub const SOURCES: &[SourceSpec] = &[
    SourceSpec {
        slug: SourceSlug::OpenDesign,
        label: "open-design (nexu-io)",
        schema: SourceSchema::OpenDesign,
        upstream: "https://github.com/nexu-io/open-design",
        env: "APHRODY_OD_SKILLS",
        defaults: open_design_defaults,
    },
    SourceSpec {
        slug: SourceSlug::Openclaw,
        label: "openclaw",
        schema: SourceSchema::OpenDesign,
        upstream: "https://github.com/openclaw/openclaw",
        env: "APHRODY_OC_SKILLS",
        defaults: openclaw_defaults,
    },
    SourceSpec {
        slug: SourceSlug::ClaudeCode,
        label: "claude-code (project-local)",
        schema: SourceSchema::Aphrody,
        upstream: "https://github.com/anthropics/skills",
        env: "APHRODY_CC_SKILLS",
        defaults: claude_code_defaults,
    },
    SourceSpec {
        slug: SourceSlug::GeminiCli,
        label: "gemini-cli",
        schema: SourceSchema::Gemini,
        upstream: "https://github.com/google-gemini/gemini-cli",
        env: "APHRODY_GEMINI_SKILLS",
        defaults: gemini_defaults,
    },
    SourceSpec {
        slug: SourceSlug::VercelLabs,
        label: "vercel-labs (agent-skills)",
        schema: SourceSchema::Vercel,
        upstream: "https://github.com/vercel-labs/agent-skills",
        env: "APHRODY_VERCEL_SKILLS",
        defaults: vercel_labs_defaults,
    },
    SourceSpec {
        slug: SourceSlug::VercelSkills,
        label: "vercel-labs (skills)",
        schema: SourceSchema::Vercel,
        upstream: "https://github.com/vercel-labs/skills",
        env: "APHRODY_VERCEL_SKILLS_REPO",
        defaults: vercel_skills_defaults,
    },
    SourceSpec {
        slug: SourceSlug::VercelOpenAgents,
        label: "vercel-labs (open-agents)",
        schema: SourceSchema::Vercel,
        upstream: "https://github.com/vercel-labs/open-agents",
        env: "APHRODY_VERCEL_OPEN_AGENTS",
        defaults: vercel_open_agents_defaults,
    },
    SourceSpec {
        slug: SourceSlug::GoogleLabs,
        label: "google-labs-skills",
        schema: SourceSchema::OpenDesign,
        upstream: "https://github.com/google/labs-skills",
        env: "APHRODY_GOOGLE_SKILLS",
        defaults: google_labs_defaults,
    },
];

/// Resolve a single source root, honouring `$ENV` override then defaults.
/// Returns `None` if neither resolves to an existing directory.
#[must_use]
pub fn resolve_source_root(spec: &SourceSpec) -> Option<PathBuf> {
    if let Ok(o) = env::var(spec.env) {
        if !o.is_empty() {
            let p = PathBuf::from(&o);
            return p.exists().then_some(p);
        }
    }
    for cand in (spec.defaults)() {
        if cand.exists() {
            return Some(cand);
        }
    }
    None
}

/// Apply `APHRODY_SKILLS_SOURCES` (comma-separated allowlist) to filter
/// the registry. Returns the full registry if the var is unset/empty.
#[must_use]
pub fn active_sources() -> Vec<&'static SourceSpec> {
    let raw = env::var("APHRODY_SKILLS_SOURCES").unwrap_or_default();
    if raw.trim().is_empty() {
        return SOURCES.iter().collect();
    }
    let allow: std::collections::HashSet<&str> = raw
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    SOURCES
        .iter()
        .filter(|s| allow.contains(s.slug.as_str()))
        .collect()
}

/// Lookup a source by its kebab-case slug.
#[must_use]
pub fn source_by_slug(slug: &str) -> Option<&'static SourceSpec> {
    SOURCES.iter().find(|s| s.slug.as_str() == slug)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_roundtrip() {
        for spec in SOURCES {
            assert_eq!(
                SourceSlug::from_str(spec.slug.as_str()),
                Some(spec.slug),
                "slug {} did not round-trip",
                spec.slug.as_str()
            );
        }
    }

    #[test]
    fn source_by_slug_finds_all() {
        for spec in SOURCES {
            assert!(source_by_slug(spec.slug.as_str()).is_some());
        }
        assert!(source_by_slug("not-a-real-slug").is_none());
    }

    #[test]
    fn registry_has_eight_entries() {
        assert_eq!(SOURCES.len(), 8);
    }
}
