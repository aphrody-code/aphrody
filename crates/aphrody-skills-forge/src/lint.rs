// SPDX-License-Identifier: Apache-2.0
//! SKILL.md style + policy linter.
//!
//! Rules enforced (see `docs/cargo/SKILLS.md` and the project Rust-only
//! policy):
//!
//! | Rule                                        | Severity | Reference                              |
//! |---------------------------------------------|----------|----------------------------------------|
//! | `name` matches `^[a-z][a-z0-9-]{1,40}$`     | Error    | docs/cargo/SKILLS.md §5.1              |
//! | `description` present and ≤200 chars        | Error    | docs/cargo/SKILLS.md §2                |
//! | `runtime` field present                     | Warning  | this crate's frontmatter contract      |
//! | `runtime == Bun`                            | Warning  | Rust-only policy                       |
//! | Body contains at least one ATX heading      | Warning  | docs/cargo/SKILLS.md §5.2 (one H1)     |
//! | Body length ≥ 50 chars                      | Warning  | minimal-instruction floor              |
//! | No mention of `Claude` / `AI` / `Anthropic` | Error    | aphrody-code anonymisation standards   |
//! | No mention of `OpenAI`                      | Error    | providers whitelist 3 (Anthropic +     |
//! |                                             |          | Gemini + Vertex AG only)               |
//!
//! Each violation returns a [`LintWarning`] tagged with line number (1-based,
//! relative to the SKILL.md source) and severity.

use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::skill::{Skill, SkillRuntime};

/// Severity of a single lint finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LintSeverity {
    /// Blocks publication — fix required.
    Error,
    /// Should be fixed but not blocking.
    Warning,
    /// Informational only.
    Info,
}

/// One finding from [`lint_skill`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LintWarning {
    /// Stable rule id, e.g. `name.kebab-case`.
    pub rule: String,
    /// Severity classification.
    pub severity: LintSeverity,
    /// 1-based line number where the violation was detected, or `0` if the
    /// rule applies to the document as a whole.
    pub line: usize,
    /// Human-readable message; safe to print to a terminal.
    pub message: String,
}

static NAME_RX: Lazy<Regex> = Lazy::new(|| {
    // Anchored kebab-case: starts with a letter, then 1–40 lower-alnum/hyphen.
    Regex::new(r"^[a-z][a-z0-9-]{1,40}$").expect("static regex compiles")
});

static FORBIDDEN_BRANDS: &[(&str, &str)] = &[
    ("claude", "Claude"),
    ("anthropic", "Anthropic"),
    ("openai", "OpenAI"),
    // We deliberately match the standalone token "AI" (case-sensitive) inside
    // the body iteration below; not in this lower-cased fast path.
];

/// Run every lint rule against `skill` and return the accumulated findings.
///
/// The returned vector is ordered by line number, then severity, so a CLI can
/// stream output deterministically.
#[must_use]
pub fn lint_skill(skill: &Skill) -> Vec<LintWarning> {
    let mut out = Vec::new();
    check_name(skill, &mut out);
    check_description(skill, &mut out);
    check_runtime(skill, &mut out);
    check_body(skill, &mut out);
    check_brand_mentions(skill, &mut out);
    out.sort_by(|a, b| a.line.cmp(&b.line).then(a.rule.cmp(&b.rule)));
    out
}

fn check_name(skill: &Skill, out: &mut Vec<LintWarning>) {
    if !NAME_RX.is_match(&skill.frontmatter.name) {
        out.push(LintWarning {
            rule: "name.kebab-case".to_string(),
            severity: LintSeverity::Error,
            line: 0,
            message: format!(
                "skill name {:?} must match ^[a-z][a-z0-9-]{{1,40}}$",
                skill.frontmatter.name
            ),
        });
    }
}

fn check_description(skill: &Skill, out: &mut Vec<LintWarning>) {
    let d = skill.frontmatter.description.trim();
    if d.is_empty() {
        out.push(LintWarning {
            rule: "description.required".to_string(),
            severity: LintSeverity::Error,
            line: 0,
            message: "description must be a non-empty single-line summary".to_string(),
        });
    } else if d.chars().count() > 200 {
        out.push(LintWarning {
            rule: "description.length".to_string(),
            severity: LintSeverity::Error,
            line: 0,
            message: format!(
                "description is {} chars (max 200); move detail into the body",
                d.chars().count()
            ),
        });
    }
}

fn check_runtime(skill: &Skill, out: &mut Vec<LintWarning>) {
    if let SkillRuntime::Other(tag) = &skill.frontmatter.runtime
        && tag == "legacy"
    {
        out.push(LintWarning {
            rule: "runtime.missing".to_string(),
            severity: LintSeverity::Warning,
            line: 0,
            message: "frontmatter has no `runtime:` field (defaulted to other:legacy)".to_string(),
        });
    }
    if matches!(skill.frontmatter.runtime, SkillRuntime::Bun) {
        out.push(LintWarning {
            rule: "runtime.bun-deprecated".to_string(),
            severity: LintSeverity::Warning,
            line: 0,
            message: "runtime: bun is deprecated for aphrody skills — migrate to rust/cargo/shell"
                .to_string(),
        });
    }
}

fn check_body(skill: &Skill, out: &mut Vec<LintWarning>) {
    let body = &skill.body;
    if body.trim().chars().count() < 50 {
        out.push(LintWarning {
            rule: "body.too-short".to_string(),
            severity: LintSeverity::Warning,
            line: 0,
            message: "body is shorter than 50 chars — add usage/notes sections".to_string(),
        });
    }
    let has_heading = body
        .lines()
        .any(|l| l.trim_start().starts_with("# ") || l.trim_start().starts_with("## "));
    if !has_heading {
        out.push(LintWarning {
            rule: "body.no-heading".to_string(),
            severity: LintSeverity::Warning,
            line: 0,
            message: "body must contain at least one `#` or `##` heading".to_string(),
        });
    }
}

fn check_brand_mentions(skill: &Skill, out: &mut Vec<LintWarning>) {
    // Combine frontmatter (description) + body so the rule covers the whole
    // document. Frontmatter is at the top, body follows the closing fence; we
    // therefore report lines as numbered through the body for accuracy and
    // line 0 for frontmatter hits.
    let desc = skill.frontmatter.description.to_ascii_lowercase();
    for (needle, label) in FORBIDDEN_BRANDS {
        if contains_word(&desc, needle) {
            out.push(LintWarning {
                rule: format!("brand.{needle}"),
                severity: LintSeverity::Error,
                line: 0,
                message: format!(
                    "frontmatter description must not mention {label} (anonymisation policy)"
                ),
            });
        }
    }
    for (idx, raw_line) in skill.body.lines().enumerate() {
        let line_no = idx + 1;
        let lc = raw_line.to_ascii_lowercase();
        for (needle, label) in FORBIDDEN_BRANDS {
            if contains_word(&lc, needle) {
                out.push(LintWarning {
                    rule: format!("brand.{needle}"),
                    severity: LintSeverity::Error,
                    line: line_no,
                    message: format!("body must not mention {label} (anonymisation policy)"),
                });
            }
        }
        // Case-sensitive standalone "AI" detection (avoids matching "tail",
        // "rail", "claim", "ailment", etc.).
        for m in find_word_token(raw_line, "AI") {
            out.push(LintWarning {
                rule: "brand.ai".to_string(),
                severity: LintSeverity::Error,
                line: line_no,
                message: format!("body must not mention \"AI\" (column {m}, anonymisation policy)"),
            });
        }
    }
}

/// Returns true if `haystack` contains `needle` surrounded by non-alphanumeric
/// boundaries (or string edges). `needle` is assumed lowercase; `haystack`
/// should already be lowercased by the caller.
fn contains_word(haystack: &str, needle: &str) -> bool {
    let bytes = haystack.as_bytes();
    let nlen = needle.len();
    let mut i = 0;
    while i + nlen <= bytes.len() {
        if &bytes[i..i + nlen] == needle.as_bytes() {
            let left_ok = i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
            let right_ok =
                i + nlen == bytes.len() || !bytes[i + nlen].is_ascii_alphanumeric();
            if left_ok && right_ok {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// Returns starting byte offsets (1-based for human reporting) of each
/// occurrence of `needle` as a whole word in `haystack`.
fn find_word_token(haystack: &str, needle: &str) -> Vec<usize> {
    let bytes = haystack.as_bytes();
    let nlen = needle.len();
    let mut out = Vec::new();
    let mut i = 0;
    while i + nlen <= bytes.len() {
        if &bytes[i..i + nlen] == needle.as_bytes() {
            let left_ok = i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
            let right_ok =
                i + nlen == bytes.len() || !bytes[i + nlen].is_ascii_alphanumeric();
            if left_ok && right_ok {
                out.push(i + 1);
            }
        }
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skill::SkillFrontmatter;
    use std::path::PathBuf;

    fn mk(name: &str, desc: &str, runtime: SkillRuntime, body: &str) -> Skill {
        Skill {
            path: PathBuf::from("/x/SKILL.md"),
            frontmatter: SkillFrontmatter {
                name: name.to_string(),
                description: desc.to_string(),
                runtime,
                version: None,
                author: None,
                tags: Vec::new(),
            },
            body: body.to_string(),
        }
    }

    #[test]
    fn clean_skill_has_no_findings() {
        let body = "# Forge demo\nLong enough body block to satisfy the floor rule with margin.";
        let s = mk("forge-demo", "A clean demo skill", SkillRuntime::Rust, body);
        assert!(lint_skill(&s).is_empty(), "{:?}", lint_skill(&s));
    }

    #[test]
    fn bad_name_is_error() {
        let s = mk(
            "BadName",
            "ok",
            SkillRuntime::Rust,
            "# h\nlong body block above the fifty char floor rule margin.",
        );
        let w = lint_skill(&s);
        assert!(w.iter().any(|w| w.rule == "name.kebab-case"));
    }

    #[test]
    fn bun_runtime_is_warning_not_error() {
        let s = mk(
            "ok-name",
            "ok",
            SkillRuntime::Bun,
            "# h\nlong body block above the fifty char floor rule margin.",
        );
        let w = lint_skill(&s);
        assert!(w
            .iter()
            .any(|w| w.rule == "runtime.bun-deprecated" && w.severity == LintSeverity::Warning));
    }

    #[test]
    fn brand_word_boundary_avoids_substrings() {
        // "claim" and "tail" must NOT trigger brand.claude / brand.ai.
        let body =
            "# h\nWe should claim the trail and finish the rail above the fifty char floor.";
        let s = mk("ok-name", "ok", SkillRuntime::Rust, body);
        let w = lint_skill(&s);
        assert!(w.is_empty(), "unexpected findings: {w:?}");
    }
}
