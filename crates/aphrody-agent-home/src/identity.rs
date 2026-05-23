// SPDX-License-Identifier: Apache-2.0
//! `IDENTITY.md` -> typed [`Identity`] (AH-2).
//!
//! Port of `var/openclaw/src/agents/identity.ts` (name / vibe / emoji +
//! ack-reaction + name-prefix resolution), but sourced from the workspace
//! `IDENTITY.md` frontmatter rather than a config object. The default name is
//! the neutral `aphrody` placeholder (CLAUDE.md anonymisation rule: never a
//! real person).

use serde::{Deserialize, Serialize};

use crate::frontmatter::{self, FmValue};

/// Default agent name when `IDENTITY.md` does not set one.
pub const DEFAULT_AGENT_NAME: &str = "aphrody";

/// Default acknowledgement reaction (openclaw `identity.ts:5` uses an emoji;
/// aphrody forbids emoji in source, so the neutral default is `ack`).
pub const DEFAULT_ACK_REACTION: &str = "ack";

/// Parsed `IDENTITY.md`: who the agent is.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identity {
    /// Agent name (used as message prefix `[name]`).
    pub name: String,
    /// One-line vibe / personality descriptor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vibe: Option<String>,
    /// Acknowledgement reaction token (emoji upstream; neutral string here).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub emoji: Option<String>,
    /// Residual markdown body.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub body: String,
}

impl Default for Identity {
    fn default() -> Self {
        Self {
            name: DEFAULT_AGENT_NAME.to_string(),
            vibe: None,
            emoji: None,
            body: String::new(),
        }
    }
}

impl Identity {
    /// Parse `IDENTITY.md` content into a typed [`Identity`]. Never fails: an
    /// empty / malformed document yields the [`Identity::default`].
    #[must_use]
    pub fn parse(content: &str) -> Self {
        let (fm, body) = frontmatter::split(content);
        let mut id = Identity {
            body: body.to_string(),
            ..Identity::default()
        };
        if let Some(fm) = fm {
            for (key, val) in frontmatter::parse(fm) {
                match (key.as_str(), val) {
                    ("name", FmValue::Scalar(s)) if !s.trim().is_empty() => {
                        id.name = s.trim().to_string();
                    }
                    ("vibe", FmValue::Scalar(s)) if !s.trim().is_empty() => {
                        id.vibe = Some(s.trim().to_string());
                    }
                    ("emoji" | "ack" | "ack_reaction", FmValue::Scalar(s))
                        if !s.trim().is_empty() =>
                    {
                        id.emoji = Some(s.trim().to_string());
                    }
                    _ => {}
                }
            }
        }
        // If no frontmatter name but the body starts with an H1, adopt it as
        // the name (common authoring shorthand: `# Aria`).
        if id.name == DEFAULT_AGENT_NAME {
            if let Some(h1) = first_h1(&id.body) {
                id.name = h1;
            }
        }
        id
    }

    /// Message prefix `[name]` (openclaw `resolveIdentityNamePrefix`).
    #[must_use]
    pub fn name_prefix(&self) -> String {
        format!("[{}]", self.name)
    }

    /// Acknowledgement reaction: the configured token, else the neutral
    /// default (openclaw `resolveAckReaction` L4 fallback).
    #[must_use]
    pub fn ack_reaction(&self) -> &str {
        self.emoji
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(DEFAULT_ACK_REACTION)
    }

    /// Render the identity as a directive line for the system prompt.
    #[must_use]
    pub fn render_directives(&self) -> String {
        let mut out = format!("You are {}.", self.name);
        if let Some(v) = &self.vibe {
            out.push(' ');
            out.push_str(v);
            if !v.ends_with(['.', '!', '?']) {
                out.push('.');
            }
        }
        out.push('\n');
        if !self.body.is_empty() {
            out.push_str(self.body.trim_end());
            out.push('\n');
        }
        out
    }
}

/// Extract the first `# Heading` text from a markdown body.
fn first_h1(body: &str) -> Option<String> {
    for line in body.lines() {
        let l = line.trim_start();
        if let Some(rest) = l.strip_prefix("# ") {
            let text = rest.trim();
            if !text.is_empty() {
                return Some(text.to_string());
            }
        }
        // Stop scanning past the first non-blank, non-H1 line.
        if !l.trim().is_empty() && !l.starts_with('#') {
            break;
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_name_is_neutral_placeholder() {
        let id = Identity::default();
        assert_eq!(id.name, "aphrody");
        assert_eq!(id.ack_reaction(), "ack");
        assert_eq!(id.name_prefix(), "[aphrody]");
    }

    #[test]
    fn parses_frontmatter_name_vibe_emoji() {
        let src = "---\nname: Aria\nvibe: precise and warm\nemoji: sparkle\n---\nHello.\n";
        let id = Identity::parse(src);
        assert_eq!(id.name, "Aria");
        assert_eq!(id.vibe.as_deref(), Some("precise and warm"));
        assert_eq!(id.ack_reaction(), "sparkle");
        assert_eq!(id.name_prefix(), "[Aria]");
    }

    #[test]
    fn adopts_h1_when_no_frontmatter_name() {
        let id = Identity::parse("# Nova\n\nA fast, focused assistant.\n");
        assert_eq!(id.name, "Nova");
    }

    #[test]
    fn empty_document_yields_default() {
        let id = Identity::parse("");
        assert_eq!(id, Identity::default());
    }

    #[test]
    fn render_directives_includes_name_and_vibe() {
        let id = Identity {
            name: "Aria".into(),
            vibe: Some("precise and warm".into()),
            emoji: None,
            body: String::new(),
        };
        let d = id.render_directives();
        assert!(d.starts_with("You are Aria. precise and warm.\n"));
    }

    #[test]
    fn ack_reaction_ignores_blank_emoji() {
        let id = Identity {
            name: "x".into(),
            vibe: None,
            emoji: Some("   ".into()),
            body: String::new(),
        };
        assert_eq!(id.ack_reaction(), "ack");
    }
}
