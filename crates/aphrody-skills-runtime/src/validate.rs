// SPDX-License-Identifier: Apache-2.0
//
// Validation — enforce the minimum SKILL.md frontmatter contract documented
// in docs/cargo/SKILLS.md §2.
//
// Avoids pulling a full JSON-Schema engine: the contract is small enough
// (3 required fields, kebab-case name, description ≥1 char) that hand-coded
// rules stay shorter, faster, and AOT-friendly. If we later need richer
// schemas we can swap in `jsonschema` without breaking callers.

use std::fmt;

use crate::frontmatter::FrontMatter;

/// One validation error surfaced for a single SKILL.md.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    MissingName,
    NameNotKebabCase(String),
    NameMismatch { expected: String, found: String },
    MissingDescription,
    DescriptionEmpty,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingName => write!(f, "frontmatter is missing required field `name`"),
            Self::NameNotKebabCase(n) => {
                write!(f, "name `{n}` is not kebab-case (use lowercase + dashes)")
            }
            Self::NameMismatch { expected, found } => write!(
                f,
                "frontmatter name `{found}` does not match directory name `{expected}`"
            ),
            Self::MissingDescription => {
                write!(f, "frontmatter is missing required field `description`")
            }
            Self::DescriptionEmpty => write!(f, "frontmatter `description` is empty"),
        }
    }
}

impl std::error::Error for ValidationError {}

/// Validate a parsed [`FrontMatter`]. Pass the SKILL's directory name in
/// `dir_name` (used to enforce the `name == basename(dir)` rule). Pass
/// `None` to skip that check.
#[must_use]
pub fn validate_frontmatter(
    fm: &FrontMatter,
    dir_name: Option<&str>,
) -> Vec<ValidationError> {
    let mut errs = Vec::new();
    let name = match &fm.name {
        Some(n) if !n.trim().is_empty() => n.trim().to_string(),
        _ => {
            errs.push(ValidationError::MissingName);
            String::new()
        }
    };
    if !name.is_empty() && !is_kebab_case(&name) {
        errs.push(ValidationError::NameNotKebabCase(name.clone()));
    }
    if let Some(expected) = dir_name {
        if !name.is_empty() && name != expected {
            errs.push(ValidationError::NameMismatch {
                expected: expected.to_string(),
                found: name,
            });
        }
    }
    match &fm.description {
        None => errs.push(ValidationError::MissingDescription),
        Some(d) if d.trim().is_empty() => errs.push(ValidationError::DescriptionEmpty),
        _ => {}
    }
    errs
}

fn is_kebab_case(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    // Allowed: a-z, 0-9, '-'. Cannot start/end with '-'. No consecutive '-'.
    let bytes = s.as_bytes();
    if bytes[0] == b'-' || bytes[bytes.len() - 1] == b'-' {
        return false;
    }
    let mut prev_dash = false;
    for &b in bytes {
        let ok = b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-';
        if !ok {
            return false;
        }
        if b == b'-' {
            if prev_dash {
                return false;
            }
            prev_dash = true;
        } else {
            prev_dash = false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fm(name: Option<&str>, desc: Option<&str>) -> FrontMatter {
        FrontMatter {
            name: name.map(str::to_string),
            description: desc.map(str::to_string),
            ..FrontMatter::default()
        }
    }

    #[test]
    fn validates_minimum_contract() {
        let f = fm(Some("my-skill"), Some("hello"));
        assert!(validate_frontmatter(&f, Some("my-skill")).is_empty());
    }

    #[test]
    fn flags_missing_name() {
        let f = fm(None, Some("hello"));
        let errs = validate_frontmatter(&f, None);
        assert!(errs.contains(&ValidationError::MissingName));
    }

    #[test]
    fn flags_non_kebab_case() {
        let f = fm(Some("My_Skill"), Some("hello"));
        let errs = validate_frontmatter(&f, None);
        assert!(matches!(
            errs.first(),
            Some(ValidationError::NameNotKebabCase(_))
        ));
    }

    #[test]
    fn flags_dir_name_mismatch() {
        let f = fm(Some("foo"), Some("hello"));
        let errs = validate_frontmatter(&f, Some("bar"));
        assert!(errs
            .iter()
            .any(|e| matches!(e, ValidationError::NameMismatch { .. })));
    }

    #[test]
    fn flags_empty_description() {
        let f = fm(Some("foo"), Some("   "));
        let errs = validate_frontmatter(&f, None);
        assert!(errs.contains(&ValidationError::DescriptionEmpty));
    }
}
