// SPDX-License-Identifier: Apache-2.0
//! Programmatic SKILL.md scaffolder.
//!
//! [`scaffold_skill`] creates `<root>/.claude/skills/<name>/SKILL.md` with
//! valid frontmatter and a three-section body (Description, Usage, Notes).
//! It refuses to overwrite an existing directory — callers must delete or
//! rename first. The emitted file always passes [`crate::lint::lint_skill`]
//! with zero Error-severity findings.

use std::path::{Path, PathBuf};

use crate::registry::ForgeError;
use crate::skill::SkillRuntime;

/// Create a fresh SKILL.md at `<root>/.claude/skills/<name>/SKILL.md`.
///
/// - `name` must satisfy kebab-case `^[a-z][a-z0-9-]{1,40}$`.
/// - `description` is written verbatim (callers should keep it ≤ 200 chars).
/// - `runtime` populates the frontmatter `runtime:` field.
///
/// Returns the absolute path of the created file.
///
/// # Errors
///
/// - [`ForgeError::InvalidName`] if `name` is not kebab-case.
/// - [`ForgeError::AlreadyExists`] if the target skill directory exists.
/// - [`ForgeError::Io`] on filesystem failures while creating the directory
///   or writing the file.
pub fn scaffold_skill(
    root: &Path,
    name: &str,
    description: &str,
    runtime: SkillRuntime,
) -> Result<PathBuf, ForgeError> {
    if !is_kebab_case(name) {
        return Err(ForgeError::InvalidName(name.to_string()));
    }
    let dir = root.join(".claude").join("skills").join(name);
    if dir.exists() {
        return Err(ForgeError::AlreadyExists(dir));
    }
    std::fs::create_dir_all(&dir).map_err(|e| ForgeError::io(&dir, e))?;
    let file_path = dir.join("SKILL.md");
    let body = render(name, description.trim(), &runtime);
    std::fs::write(&file_path, body).map_err(|e| ForgeError::io(&file_path, e))?;
    Ok(file_path)
}

fn is_kebab_case(s: &str) -> bool {
    if s.is_empty() || s.len() > 41 {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_lowercase() {
        return false;
    }
    chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn render(name: &str, description: &str, runtime: &SkillRuntime) -> String {
    // Description is single-line: collapse any newlines the caller may have
    // injected so the YAML stays a scalar (avoids accidental block scalars).
    let one_line_desc = description.replace(['\r', '\n'], " ");
    format!(
        "---\nname: {name}\ndescription: {desc}\nruntime: {runtime}\n---\n\
         \n# {title}\n\n\
         ## Description\n\n\
         {desc_long}\n\n\
         ## Usage\n\n\
         Document the invocation surface here: command line, slash trigger, \
         or programmatic entry-point. Example:\n\n\
         ```bash\n# replace with the real command\n{name} --help\n```\n\n\
         ## Notes\n\n\
         - Authoring conventions: see `docs/cargo/SKILLS.md`.\n\
         - Lint with `aphrody-skills-forge` before publishing.\n",
        name = name,
        desc = yaml_escape(&one_line_desc),
        runtime = runtime.as_str(),
        title = humanise(name),
        desc_long = description,
    )
}

fn yaml_escape(s: &str) -> String {
    // If the value contains YAML special punctuation, wrap in double quotes
    // and escape embedded quotes / backslashes. Otherwise return as-is.
    let needs_quote = s.is_empty()
        || s.starts_with(|c: char| {
            matches!(
                c,
                '!' | '&' | '*' | '{' | '}' | '[' | ']' | '|' | '>' | '\''
                    | '"' | '%' | '@' | '`' | '#'
            )
        })
        || s.contains(':')
        || s.contains('\t');
    if !needs_quote {
        return s.to_string();
    }
    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

fn humanise(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut cap_next = true;
    for c in name.chars() {
        if c == '-' {
            out.push(' ');
            cap_next = true;
        } else if cap_next {
            out.extend(c.to_uppercase());
            cap_next = false;
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::load_registry;
    use tempfile::TempDir;

    #[test]
    fn scaffold_rejects_bad_name() {
        let dir = TempDir::new().unwrap();
        let err = scaffold_skill(dir.path(), "BadName", "x", SkillRuntime::Rust).unwrap_err();
        assert!(matches!(err, ForgeError::InvalidName(_)));
    }

    #[test]
    fn scaffold_creates_loadable_skill() {
        let dir = TempDir::new().unwrap();
        let path =
            scaffold_skill(dir.path(), "demo-skill", "Demo skill", SkillRuntime::Rust).unwrap();
        assert!(path.exists());
        let skills = load_registry(dir.path()).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].frontmatter.name, "demo-skill");
    }

    #[test]
    fn humanise_title() {
        assert_eq!(humanise("foo-bar-baz"), "Foo Bar Baz");
    }

    #[test]
    fn yaml_escape_passes_through_simple() {
        assert_eq!(yaml_escape("simple text"), "simple text");
    }

    #[test]
    fn yaml_escape_quotes_colon() {
        assert_eq!(yaml_escape("has: colon"), "\"has: colon\"");
    }
}
