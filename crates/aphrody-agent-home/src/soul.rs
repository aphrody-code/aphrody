// SPDX-License-Identifier: Apache-2.0
//! `SOUL.md` -> typed [`Soul`] (AH-1).
//!
//! openclaw treats `SOUL.md` as opaque markdown injected wholesale. aphrody
//! parses an optional YAML-ish frontmatter block into a typed [`Soul`] struct
//! (tone / opinions / brevity / humour / boundaries / bluntness) and validates
//! the residual body against the anti-patterns the openclaw soul docs warn
//! against (life-stories, changelogs, policy/licence dumps, vague philosophy)
//! BEFORE the persona is allowed into a system prompt. Validation never panics
//! — it returns an actionable [`crate::HomeError::SoulValidation`].
//!
//! The frontmatter parser is intentionally tiny (no `serde_yaml` dependency,
//! which the workspace dropped over RUSTSEC-2025-0068): it understands
//! `key: scalar` and block lists (`key:` followed by `  - item` lines). This
//! covers the persona schema without inviting arbitrary YAML.

use serde::{Deserialize, Serialize};

use crate::frontmatter::{self, FmValue};
use crate::HomeError;

/// How terse the agent should be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Brevity {
    /// One-liners; minimal preamble.
    Terse,
    /// A few sentences; the default.
    #[default]
    Balanced,
    /// Full explanations when the topic warrants.
    Verbose,
}

/// How much humour to deploy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Humor {
    /// No jokes.
    None,
    /// Occasional dry wit; the default.
    #[default]
    Dry,
    /// Playful and frequent.
    Playful,
}

/// How blunt the agent is by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Bluntness {
    /// Softened, diplomatic phrasing.
    Gentle,
    /// Direct but courteous; the default.
    #[default]
    Direct,
    /// No hedging; says the hard thing.
    Frank,
}

/// Parsed `SOUL.md`: the agent's persona.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Soul {
    /// Overall tone descriptor (free text, e.g. "warm but precise").
    #[serde(default)]
    pub tone: String,
    /// Strong opinions the agent holds and defends.
    #[serde(default)]
    pub opinions: Vec<String>,
    /// Preferred verbosity.
    #[serde(default)]
    pub brevity: Brevity,
    /// Humour level.
    #[serde(default)]
    pub humor: Humor,
    /// Hard limits / things the agent will not do.
    #[serde(default)]
    pub boundaries: Vec<String>,
    /// Default bluntness.
    #[serde(default)]
    pub default_bluntness: Bluntness,
    /// Residual markdown body after the frontmatter block.
    #[serde(default)]
    pub body: String,
}

impl Soul {
    /// Parse `SOUL.md` content into a typed [`Soul`], running anti-pattern
    /// lints on the body.
    ///
    /// # Errors
    /// Returns [`HomeError::SoulValidation`] when the body trips an
    /// anti-pattern lint (changelog markers, policy/licence dumps, an
    /// over-long body, or a high ratio of vague-philosophy sentences).
    pub fn parse(content: &str) -> Result<Self, HomeError> {
        let (fm, body) = frontmatter::split(content);
        let mut soul = Soul {
            body: body.to_string(),
            ..Soul::default()
        };
        if let Some(fm) = fm {
            for (key, val) in frontmatter::parse(fm) {
                soul.apply_field(&key, val);
            }
        }
        soul.validate()?;
        Ok(soul)
    }

    /// Parse without running the anti-pattern lints. Useful for tests and for
    /// reading a soul that the operator has explicitly accepted.
    #[must_use]
    pub fn parse_lenient(content: &str) -> Self {
        let (fm, body) = frontmatter::split(content);
        let mut soul = Soul {
            body: body.to_string(),
            ..Soul::default()
        };
        if let Some(fm) = fm {
            for (key, val) in frontmatter::parse(fm) {
                soul.apply_field(&key, val);
            }
        }
        soul
    }

    fn apply_field(&mut self, key: &str, val: FmValue) {
        match (key, val) {
            ("tone", FmValue::Scalar(s)) => self.tone = s,
            ("opinions", FmValue::List(v)) => self.opinions = v,
            ("opinions", FmValue::Scalar(s)) if !s.is_empty() => self.opinions = vec![s],
            ("boundaries", FmValue::List(v)) => self.boundaries = v,
            ("boundaries", FmValue::Scalar(s)) if !s.is_empty() => self.boundaries = vec![s],
            ("brevity", FmValue::Scalar(s)) => self.brevity = parse_brevity(&s),
            ("humor" | "humour", FmValue::Scalar(s)) => self.humor = parse_humor(&s),
            ("default_bluntness" | "bluntness", FmValue::Scalar(s)) => {
                self.default_bluntness = parse_bluntness(&s);
            }
            _ => {}
        }
    }

    /// Run the anti-pattern lints. Idempotent; called by [`Soul::parse`].
    ///
    /// # Errors
    /// See [`Soul::parse`].
    pub fn validate(&self) -> Result<(), HomeError> {
        lint_body(&self.body)
    }

    /// Render the persona as a compact directive block for the system prompt.
    /// Deterministic ordering (cache-friendly per the prompt-cache rule in
    /// CLAUDE.md / openclaw AGENTS.md).
    #[must_use]
    pub fn render_directives(&self) -> String {
        let mut out = String::new();
        if !self.tone.is_empty() {
            out.push_str("Tone: ");
            out.push_str(&self.tone);
            out.push('\n');
        }
        out.push_str("Brevity: ");
        out.push_str(self.brevity.as_str());
        out.push_str("; humour: ");
        out.push_str(self.humor.as_str());
        out.push_str("; bluntness: ");
        out.push_str(self.default_bluntness.as_str());
        out.push('\n');
        if !self.opinions.is_empty() {
            out.push_str("Opinions:\n");
            for op in &self.opinions {
                out.push_str("- ");
                out.push_str(op);
                out.push('\n');
            }
        }
        if !self.boundaries.is_empty() {
            out.push_str("Boundaries:\n");
            for b in &self.boundaries {
                out.push_str("- ");
                out.push_str(b);
                out.push('\n');
            }
        }
        if !self.body.is_empty() {
            out.push('\n');
            out.push_str(self.body.trim_end());
            out.push('\n');
        }
        out
    }
}

impl Brevity {
    /// Lowercase identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Brevity::Terse => "terse",
            Brevity::Balanced => "balanced",
            Brevity::Verbose => "verbose",
        }
    }
}

impl Humor {
    /// Lowercase identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Humor::None => "none",
            Humor::Dry => "dry",
            Humor::Playful => "playful",
        }
    }
}

impl Bluntness {
    /// Lowercase identifier.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Bluntness::Gentle => "gentle",
            Bluntness::Direct => "direct",
            Bluntness::Frank => "frank",
        }
    }
}

fn parse_brevity(s: &str) -> Brevity {
    match s.trim().to_ascii_lowercase().as_str() {
        "terse" | "short" | "minimal" => Brevity::Terse,
        "verbose" | "long" | "detailed" => Brevity::Verbose,
        _ => Brevity::Balanced,
    }
}

fn parse_humor(s: &str) -> Humor {
    match s.trim().to_ascii_lowercase().as_str() {
        "none" | "off" | "serious" => Humor::None,
        "playful" | "high" | "frequent" => Humor::Playful,
        _ => Humor::Dry,
    }
}

fn parse_bluntness(s: &str) -> Bluntness {
    match s.trim().to_ascii_lowercase().as_str() {
        "gentle" | "soft" | "diplomatic" => Bluntness::Gentle,
        "frank" | "brutal" | "blunt" => Bluntness::Frank,
        _ => Bluntness::Direct,
    }
}

// ---------------------------------------------------------------------------
// Anti-pattern lints (the "push past openclaw" guarantee from the plan §3).
// ---------------------------------------------------------------------------

/// Upper bound on the residual body (chars). A soul that needs more than this
/// is almost certainly a life-story / policy dump, not a persona.
const MAX_SOUL_BODY_CHARS: usize = 4_000;

/// Vague-philosophy phrase fragments. A high density of these signals a
/// motivational-poster soul rather than an operational persona.
const VAGUE_PHILOSOPHY_MARKERS: &[&str] = &[
    "the journey",
    "embrace the",
    "unleash your",
    "true potential",
    "synergy",
    "holistic",
    "paradigm shift",
    "at the end of the day",
    "think outside the box",
    "passionate about",
];

/// Policy / licence dump markers — these belong in `AGENTS.md` or a LICENSE
/// file, never the persona.
const POLICY_DUMP_MARKERS: &[&str] = &[
    "SPDX-License-Identifier",
    "Permission is hereby granted",
    "Apache License",
    "MIT License",
    "GNU GENERAL PUBLIC LICENSE",
    "Redistribution and use in source",
    "## Security",
    "## Compliance",
];

fn lint_body(body: &str) -> Result<(), HomeError> {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    let char_count = trimmed.chars().count();
    if char_count > MAX_SOUL_BODY_CHARS {
        return Err(HomeError::SoulValidation(format!(
            "SOUL.md body is {char_count} chars (cap {MAX_SOUL_BODY_CHARS}); a soul is a \
             persona, not a life story. Move long-form content to MEMORY.md or AGENTS.md."
        )));
    }

    // Changelog markers: `## v1.2.3`, `## [1.0.0]`, `### v2`.
    for line in trimmed.lines() {
        let l = line.trim_start();
        if is_changelog_heading(l) {
            return Err(HomeError::SoulValidation(format!(
                "SOUL.md contains a changelog-style heading ({l:?}); a soul is timeless \
                 persona, not release notes."
            )));
        }
    }

    let lower = trimmed.to_ascii_lowercase();
    for marker in POLICY_DUMP_MARKERS {
        if lower.contains(&marker.to_ascii_lowercase()) {
            return Err(HomeError::SoulValidation(format!(
                "SOUL.md contains a policy/licence dump marker ({marker:?}); keep policy in \
                 AGENTS.md and licences in LICENSE."
            )));
        }
    }

    // Vague-philosophy density: count sentences vs. flagged fragments.
    let sentence_count = trimmed
        .split(|c| matches!(c, '.' | '!' | '?'))
        .filter(|s| !s.trim().is_empty())
        .count()
        .max(1);
    let mut philosophy_hits = 0usize;
    for marker in VAGUE_PHILOSOPHY_MARKERS {
        if lower.contains(marker) {
            philosophy_hits += 1;
        }
    }
    // Only flag when there are several hits AND they dominate the prose: this
    // avoids tripping on a single colloquialism in an otherwise concrete soul.
    if philosophy_hits >= 3 && philosophy_hits * 3 >= sentence_count {
        return Err(HomeError::SoulValidation(format!(
            "SOUL.md reads as vague philosophy ({philosophy_hits} buzzword fragments across \
             {sentence_count} sentences). Replace mottos with concrete operational traits."
        )));
    }

    Ok(())
}

/// True for `## v1`, `## [1.2.3]`, `### v2.0` style changelog headings.
fn is_changelog_heading(line: &str) -> bool {
    let Some(rest) = line.strip_prefix('#') else {
        return false;
    };
    let rest = rest.trim_start_matches('#').trim_start();
    // `v1`, `v1.2`, `v1.2.3`
    if let Some(ver) = rest.strip_prefix('v') {
        if ver.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            return true;
        }
    }
    // `[1.0.0]`
    if let Some(inner) = rest.strip_prefix('[') {
        if inner.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_frontmatter_scalars_and_lists() {
        let src = "---\ntone: warm but precise\nbrevity: terse\nhumour: dry\n\
                   bluntness: frank\nopinions:\n  - tabs over spaces\n  - ship small PRs\n\
                   boundaries:\n  - never run destructive commands without confirmation\n---\n\
                   I am aphrody. I help you ship.\n";
        let soul = Soul::parse(src).expect("valid soul");
        assert_eq!(soul.tone, "warm but precise");
        assert_eq!(soul.brevity, Brevity::Terse);
        assert_eq!(soul.humor, Humor::Dry);
        assert_eq!(soul.default_bluntness, Bluntness::Frank);
        assert_eq!(soul.opinions, vec![
            "tabs over spaces".to_string(),
            "ship small PRs".to_string()
        ]);
        assert_eq!(soul.boundaries.len(), 1);
        assert!(soul.body.starts_with("I am aphrody"));
    }

    #[test]
    fn parses_inline_flow_list() {
        let src = "---\nopinions: [first, \"second item\", third]\n---\nbody\n";
        let soul = Soul::parse(src).expect("valid");
        assert_eq!(soul.opinions, vec![
            "first".to_string(),
            "second item".to_string(),
            "third".to_string()
        ]);
    }

    #[test]
    fn body_without_frontmatter_is_kept_whole() {
        let src = "Just a persona, no frontmatter.\nConcise and direct.\n";
        let soul = Soul::parse(src).expect("valid");
        assert_eq!(soul.tone, "");
        assert_eq!(soul.brevity, Brevity::Balanced); // default
        assert!(soul.body.contains("Just a persona"));
    }

    #[test]
    fn rejects_changelog_heading() {
        let src = "---\ntone: x\n---\n## v1.2.3\n- fixed a bug\n";
        let err = Soul::parse(src).unwrap_err();
        assert!(matches!(err, HomeError::SoulValidation(_)));
        assert!(err.to_string().contains("changelog"));
    }

    #[test]
    fn rejects_licence_dump() {
        let src = "---\ntone: x\n---\nApache License Version 2.0, January 2004\n\
                   Permission is hereby granted to do stuff.\n";
        let err = Soul::parse(src).unwrap_err();
        assert!(err.to_string().contains("policy/licence"));
    }

    #[test]
    fn rejects_over_long_body() {
        let body = "word ".repeat(1_200); // ~6000 chars
        let src = format!("---\ntone: x\n---\n{body}");
        let err = Soul::parse(&src).unwrap_err();
        assert!(err.to_string().contains("life story"));
    }

    #[test]
    fn rejects_vague_philosophy_dominant_body() {
        let src = "---\ntone: x\n---\nWe embrace the journey. Unleash your true potential. \
                   It is all about synergy.\n";
        let err = Soul::parse(src).unwrap_err();
        assert!(err.to_string().contains("vague philosophy"));
    }

    #[test]
    fn accepts_concrete_soul_with_one_colloquialism() {
        // A single buzzword in an otherwise concrete soul must NOT trip.
        let src = "---\ntone: precise\n---\nI write tight Rust. I prefer let-else. \
                   At the end of the day I verify before claiming success. I cite line numbers.\n";
        let soul = Soul::parse(src).expect("one colloquialism is fine");
        assert!(soul.body.contains("tight Rust"));
    }

    #[test]
    fn render_directives_is_deterministic_and_ordered() {
        let soul = Soul {
            tone: "warm".into(),
            opinions: vec!["a".into(), "b".into()],
            brevity: Brevity::Terse,
            humor: Humor::Dry,
            boundaries: vec!["no rm -rf".into()],
            default_bluntness: Bluntness::Frank,
            body: "Body text.".into(),
        };
        let a = soul.render_directives();
        let b = soul.render_directives();
        assert_eq!(a, b);
        assert!(a.starts_with("Tone: warm\n"));
        assert!(a.contains("Brevity: terse; humour: dry; bluntness: frank"));
        assert!(a.contains("- no rm -rf"));
    }
}
