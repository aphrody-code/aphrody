// SPDX-License-Identifier: Apache-2.0
//! `TOOLS.md` -> typed [`ToolsDoc`] (AH-3).
//!
//! Local tool conventions: which tools exist, how the agent should call them,
//! and any house rules. Required (not optional) in onboarding, like
//! `AGENTS.md` — the agent must always know its tool conventions.

use serde::{Deserialize, Serialize};

use crate::frontmatter::{self, FmValue};

/// Parsed `TOOLS.md`: local tool conventions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ToolsDoc {
    /// Names of tools the agent is expected to use.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tools: Vec<String>,
    /// House conventions / call rules (free-form bullet text).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub conventions: Vec<String>,
    /// Residual markdown body (the human-readable detail).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub body: String,
}

impl ToolsDoc {
    /// Parse `TOOLS.md` content. Never fails; the body is always retained.
    #[must_use]
    pub fn parse(content: &str) -> Self {
        let (fm, body) = frontmatter::split(content);
        let mut doc = ToolsDoc {
            body: body.to_string(),
            ..ToolsDoc::default()
        };
        if let Some(fm) = fm {
            for (key, val) in frontmatter::parse(fm) {
                match (key.as_str(), val) {
                    ("tools", FmValue::List(v)) => doc.tools = v,
                    ("tools", FmValue::Scalar(s)) if !s.trim().is_empty() => {
                        doc.tools = vec![s.trim().to_string()];
                    }
                    ("conventions" | "rules", FmValue::List(v)) => doc.conventions = v,
                    ("conventions" | "rules", FmValue::Scalar(s)) if !s.trim().is_empty() => {
                        doc.conventions = vec![s.trim().to_string()];
                    }
                    _ => {}
                }
            }
        }
        doc
    }

    /// Render the tools doc as a directive block for the system prompt.
    #[must_use]
    pub fn render_directives(&self) -> String {
        let mut out = String::new();
        if !self.tools.is_empty() {
            out.push_str("Available tools: ");
            out.push_str(&self.tools.join(", "));
            out.push_str(".\n");
        }
        if !self.conventions.is_empty() {
            out.push_str("Tool conventions:\n");
            for c in &self.conventions {
                out.push_str("- ");
                out.push_str(c);
                out.push('\n');
            }
        }
        if !self.body.is_empty() {
            out.push_str(self.body.trim_end());
            out.push('\n');
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tools_and_conventions() {
        let src = "---\ntools: [read, write, bash]\nconventions:\n  - prefer Read over cat\n\
                     - absolute paths only\n---\nDetail.\n";
        let d = ToolsDoc::parse(src);
        assert_eq!(d.tools, vec!["read".to_string(), "write".to_string(), "bash".to_string()]);
        assert_eq!(d.conventions.len(), 2);
        assert_eq!(d.body.trim(), "Detail.");
    }

    #[test]
    fn body_only_is_retained() {
        let d = ToolsDoc::parse("Use the bash tool for shell access.\n");
        assert!(d.tools.is_empty());
        assert!(d.body.contains("bash tool"));
    }

    #[test]
    fn render_directives_lists_tools() {
        let d = ToolsDoc {
            tools: vec!["read".into(), "bash".into()],
            conventions: vec!["absolute paths".into()],
            body: String::new(),
        };
        let r = d.render_directives();
        assert!(r.contains("Available tools: read, bash."));
        assert!(r.contains("- absolute paths"));
    }
}
