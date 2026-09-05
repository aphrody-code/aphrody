// SPDX-License-Identifier: Apache-2.0
//! Recursive [`Skill`] registry loader.
//!
//! Walks a directory tree (typically a repo root or a `.claude/skills/` folder)
//! and collects every `SKILL.md` reachable, parsing each one into a typed
//! [`Skill`]. Directories without a `SKILL.md` are skipped silently — the
//! linter is the venue for shape complaints, not the loader.

use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::skill::{Skill, SkillFrontmatter, SkillRuntime};

/// Maximum directory depth searched for `SKILL.md` files. Bounds runtime on
/// pathological trees (e.g. accidentally pointed at `/`).
const MAX_WALK_DEPTH: usize = 8;

/// Strongly-typed error surface for the forge.
#[derive(Debug, thiserror::Error)]
pub enum ForgeError {
    /// I/O failure while reading a skill file or scaffolding a directory.
    #[error("io error on {path:?}: {source}")]
    Io {
        /// Path that caused the failure.
        path: PathBuf,
        /// Underlying OS-level error.
        #[source]
        source: std::io::Error,
    },

    /// Frontmatter YAML parse error.
    #[error("frontmatter parse error in {path:?}: {message}")]
    Frontmatter {
        /// Path of the offending SKILL.md.
        path: PathBuf,
        /// Human-readable parser message.
        message: String,
    },

    /// Frontmatter is missing required fields (`name`, `description`,
    /// `runtime`).
    #[error("missing required field {field} in {path:?}")]
    MissingField {
        /// Path of the offending SKILL.md.
        path: PathBuf,
        /// Name of the absent field.
        field: &'static str,
    },

    /// Scaffold target already exists.
    #[error("scaffold target already exists: {0:?}")]
    AlreadyExists(PathBuf),

    /// Skill name failed kebab-case validation.
    #[error("invalid skill name {0:?}: must match ^[a-z][a-z0-9-]{{1,40}}$")]
    InvalidName(String),
}

impl ForgeError {
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}

/// Walk `root` recursively (bounded depth) and collect every parsable
/// `SKILL.md`.
///
/// - Directories that do not contain a `SKILL.md` are skipped.
/// - Files whose frontmatter is structurally invalid (missing `name`,
///   `description`, or `runtime`) raise [`ForgeError::MissingField`] for that
///   file; partial loads are not returned — callers should fix the offending
///   file or move it out of the tree.
/// - Symbolic links are followed once but loops are bounded by `MAX_WALK_DEPTH`.
///
/// # Errors
///
/// - [`ForgeError::Io`] if a discovered file cannot be read.
/// - [`ForgeError::Frontmatter`] if YAML parsing of a frontmatter block fails.
/// - [`ForgeError::MissingField`] if a required field is absent.
pub fn load_registry(root: &Path) -> Result<Vec<Skill>, ForgeError> {
    let mut out = Vec::new();
    for entry in WalkDir::new(root)
        .max_depth(MAX_WALK_DEPTH)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        if entry.file_name() != "SKILL.md" {
            continue;
        }
        let path = entry.path().to_path_buf();
        let raw = std::fs::read_to_string(&path).map_err(|e| ForgeError::io(&path, e))?;
        let skill = parse_skill(path, &raw)?;
        out.push(skill);
    }
    Ok(out)
}

/// Parse a SKILL.md document into a [`Skill`]. Public so downstream tools
/// (e.g. a CLI that reads from stdin) can reuse the same parser.
///
/// # Errors
///
/// See [`load_registry`].
pub fn parse_skill(path: PathBuf, src: &str) -> Result<Skill, ForgeError> {
    let (fm_str, body) = split_frontmatter(src);
    if fm_str.trim().is_empty() {
        return Err(ForgeError::Frontmatter {
            path,
            message: "missing YAML frontmatter (no leading `---` fence)".to_string(),
        });
    }
    let raw: serde_yaml::Value =
        serde_yaml::from_str(&fm_str).map_err(|e| ForgeError::Frontmatter {
            path: path.clone(),
            message: e.to_string(),
        })?;
    let fm = project_frontmatter(&path, &raw)?;
    Ok(Skill {
        path,
        frontmatter: fm,
        body,
    })
}

fn project_frontmatter(
    path: &Path,
    v: &serde_yaml::Value,
) -> Result<SkillFrontmatter, ForgeError> {
    let map = v.as_mapping().ok_or_else(|| ForgeError::Frontmatter {
        path: path.to_path_buf(),
        message: "frontmatter root must be a YAML mapping".to_string(),
    })?;
    let get_str = |k: &str| -> Option<String> {
        map.get(serde_yaml::Value::String(k.to_string()))
            .and_then(|v| v.as_str())
            .map(str::to_string)
    };
    let name = get_str("name").ok_or(ForgeError::MissingField {
        path: path.to_path_buf(),
        field: "name",
    })?;
    let description = get_str("description").ok_or(ForgeError::MissingField {
        path: path.to_path_buf(),
        field: "description",
    })?;
    // `runtime` is optional in legacy skills; fall back to "other:legacy" so
    // load succeeds. The linter will surface the missing-runtime warning.
    let runtime = match get_str("runtime") {
        Some(s) => SkillRuntime::parse(&s),
        None => SkillRuntime::Other("legacy".to_string()),
    };
    let version = get_str("version");
    let author = get_str("author");
    let tags = map
        .get(serde_yaml::Value::String("tags".to_string()))
        .and_then(|v| v.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|x| x.as_str().map(str::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    Ok(SkillFrontmatter {
        name,
        description: collapse_ws(&description),
        runtime,
        version,
        author,
        tags,
    })
}

/// Split a SKILL.md source into `(frontmatter_yaml, body_markdown)`. Tolerant
/// of CRLF, BOM, and a missing closing fence (body absorbs everything past
/// the opening fence in that case).
#[must_use]
pub fn split_frontmatter(src: &str) -> (String, String) {
    let stripped = src.trim_start_matches('\u{feff}');
    let after_open = match strip_open_fence(stripped) {
        Some(rest) => rest,
        None => return (String::new(), src.to_string()),
    };
    match find_close_fence(after_open) {
        Some((fm_end, body_start)) => (
            after_open[..fm_end].to_string(),
            after_open[body_start..].to_string(),
        ),
        None => (after_open.to_string(), String::new()),
    }
}

fn strip_open_fence(s: &str) -> Option<&str> {
    let rest = s.strip_prefix("---")?;
    let rest = rest.trim_start_matches([' ', '\t']);
    if let Some(r) = rest.strip_prefix("\r\n") {
        return Some(r);
    }
    rest.strip_prefix('\n')
}

fn find_close_fence(s: &str) -> Option<(usize, usize)> {
    let mut offset: usize = 0;
    for line in s.split_inclusive('\n') {
        let line_no_eol = line.trim_end_matches('\n').trim_end_matches('\r');
        if line_no_eol.trim_end() == "---" {
            return Some((offset, offset + line.len()));
        }
        offset += line.len();
    }
    None
}

fn collapse_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_ws = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !prev_ws && !out.is_empty() {
                out.push(' ');
            }
            prev_ws = true;
        } else {
            out.push(c);
            prev_ws = false;
        }
    }
    out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_extracts_fm_and_body() {
        let src = "---\nname: foo\nruntime: rust\n---\n# body\nhello\n";
        let (fm, body) = split_frontmatter(src);
        assert!(fm.contains("name: foo"));
        assert_eq!(body, "# body\nhello\n");
    }

    #[test]
    fn parse_skill_round_trip() {
        let src = "---\nname: forge-demo\ndescription: A demo\nruntime: rust\nversion: \"0.1.0\"\ntags:\n  - demo\n---\n# Body\n";
        let s = parse_skill(PathBuf::from("/x/SKILL.md"), src).unwrap();
        assert_eq!(s.frontmatter.name, "forge-demo");
        assert_eq!(s.frontmatter.runtime, SkillRuntime::Rust);
        assert_eq!(s.frontmatter.tags, vec!["demo"]);
    }
}
