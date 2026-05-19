// SPDX-License-Identifier: Apache-2.0
//! Strongly-typed DTOs for a parsed SKILL.md document.
//!
//! The frontmatter shape mirrors the spec in `docs/cargo/SKILLS.md`:
//!
//! ```yaml
//! ---
//! name: my-skill                # kebab-case (^[a-z][a-z0-9-]{1,40}$)
//! description: One-line summary
//! runtime: rust|cargo|shell|bun|other:<id>
//! version: "1.0.0"              # optional
//! author: ada                   # optional
//! tags: [build, audit]          # optional
//! ---
//! ```
//!
//! We intentionally do **not** re-derive every field already covered by
//! [`aphrody_skills_runtime::FrontMatter`]; this DTO is the authoring view
//! used by the scaffolder + linter, scoped to the fields they care about.

use std::path::PathBuf;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Execution runtime declared by a skill's `runtime:` frontmatter field.
///
/// `Bun` is **deprecated** in the aphrody catalog (see the project Rust-only
/// policy) but is still recognised by the parser so that legacy upstream
/// imports parse cleanly — [`crate::lint::lint_skill`] emits a warning when
/// encountered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum SkillRuntime {
    /// Pure Rust source skill (handler is a Rust crate / binary).
    Rust,
    /// Cargo subcommand or workspace recipe.
    Cargo,
    /// POSIX shell / pwsh / cmd helper.
    Shell,
    /// Bun / TypeScript handler (deprecated, lint-warned).
    Bun,
    /// Anything else, with a free-form identifier (e.g. `python`, `wasm`).
    #[serde(untagged)]
    Other(String),
}

impl SkillRuntime {
    /// Canonical string form used inside the YAML frontmatter.
    #[must_use]
    pub fn as_str(&self) -> &str {
        match self {
            Self::Rust => "rust",
            Self::Cargo => "cargo",
            Self::Shell => "shell",
            Self::Bun => "bun",
            Self::Other(s) => s.as_str(),
        }
    }

    /// Parse a runtime tag from its lowercase string form. Unknown tags become
    /// [`SkillRuntime::Other`] preserving the original spelling.
    #[must_use]
    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "rust" => Self::Rust,
            "cargo" => Self::Cargo,
            "shell" | "sh" | "bash" | "pwsh" | "powershell" => Self::Shell,
            "bun" | "typescript" | "ts" | "node" => Self::Bun,
            other => Self::Other(other.to_string()),
        }
    }
}

/// Frontmatter view of a SKILL.md document, scoped to the fields the forge
/// cares about (authoring + lint). Unknown fields in the source are tolerated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SkillFrontmatter {
    /// Kebab-case identifier. Must match `^[a-z][a-z0-9-]{1,40}$`.
    pub name: String,
    /// One-line activation summary. Capped at 200 chars by the linter.
    pub description: String,
    /// Execution runtime for the skill body (see [`SkillRuntime`]).
    pub runtime: SkillRuntime,
    /// SemVer string. Optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Free-form author label. Optional.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    /// Free-form tags for catalog search. Optional.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

/// Fully-loaded SKILL.md document: filesystem path, parsed frontmatter, body
/// markdown.
#[derive(Debug, Clone)]
pub struct Skill {
    /// Path to the on-disk `SKILL.md` file.
    pub path: PathBuf,
    /// Parsed YAML frontmatter (between the leading `---` fences).
    pub frontmatter: SkillFrontmatter,
    /// Markdown body after the closing `---` fence.
    pub body: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_round_trip_known() {
        for r in [
            SkillRuntime::Rust,
            SkillRuntime::Cargo,
            SkillRuntime::Shell,
            SkillRuntime::Bun,
        ] {
            let s = r.as_str().to_string();
            assert_eq!(SkillRuntime::parse(&s), r);
        }
    }

    #[test]
    fn runtime_parse_shell_aliases() {
        assert_eq!(SkillRuntime::parse("bash"), SkillRuntime::Shell);
        assert_eq!(SkillRuntime::parse("PowerShell"), SkillRuntime::Shell);
    }

    #[test]
    fn runtime_parse_other_preserves_spelling() {
        match SkillRuntime::parse("python") {
            SkillRuntime::Other(s) => assert_eq!(s, "python"),
            _ => panic!("expected Other(python)"),
        }
    }
}
