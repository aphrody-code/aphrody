// SPDX-License-Identifier: Apache-2.0
//! `HEARTBEAT.md` -> typed [`Heartbeat`] (AH-7).
//!
//! The heartbeat checklist is injected only during heartbeat / cron runs (it
//! is in openclaw's `CRON_BOOTSTRAP_ALLOWLIST`, never the subagent allowlist).
//! This module exposes a typed checklist + a session-kind gate so the runtime
//! can decide whether to include it.

use serde::{Deserialize, Serialize};

use crate::frontmatter;

/// What kind of session is requesting the bootstrap files. Mirrors openclaw's
/// `isSubagentSessionKey` / `isCronSessionKey` routing classes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SessionKind {
    /// A normal interactive session (full bootstrap).
    #[default]
    Interactive,
    /// A scheduled / heartbeat run (gets the heartbeat checklist).
    Cron,
    /// A spawned subagent (minimal bootstrap: rules + tools only).
    Subagent,
}

impl SessionKind {
    /// Whether the heartbeat checklist should be injected for this session.
    #[must_use]
    pub const fn wants_heartbeat(self) -> bool {
        matches!(self, SessionKind::Cron)
    }

    /// Whether the boot checklist should be injected for this session.
    #[must_use]
    pub const fn wants_boot(self) -> bool {
        matches!(self, SessionKind::Interactive)
    }

    /// Whether the persona files (SOUL/IDENTITY/USER) should be injected.
    /// Subagents get only rules + tools (openclaw `SUBAGENT_BOOTSTRAP_ALLOWLIST`).
    #[must_use]
    pub const fn wants_persona(self) -> bool {
        !matches!(self, SessionKind::Subagent)
    }
}

/// Parsed `HEARTBEAT.md`: a checklist of steps to run on each heartbeat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Heartbeat {
    /// Checklist items (parsed from markdown `- ` / `* ` bullets in the body).
    pub steps: Vec<String>,
    /// Residual body (anything that is not a bullet).
    pub body: String,
}

impl Heartbeat {
    /// Parse `HEARTBEAT.md` content. Never fails.
    #[must_use]
    pub fn parse(content: &str) -> Self {
        let (_fm, body) = frontmatter::split(content);
        Self {
            steps: extract_bullets(body),
            body: body.to_string(),
        }
    }

    /// True when the checklist has no actionable items.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty() && self.body.trim().is_empty()
    }
}

/// Extract markdown bullet items (`- foo`, `* foo`, `1. foo`).
pub(crate) fn extract_bullets(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in body.lines() {
        let l = line.trim_start();
        if let Some(rest) = l.strip_prefix("- ").or_else(|| l.strip_prefix("* ")) {
            let item = rest.trim();
            if !item.is_empty() {
                out.push(item.to_string());
            }
        } else if let Some((num, rest)) = split_ordered(l) {
            if num && !rest.is_empty() {
                out.push(rest.to_string());
            }
        }
    }
    out
}

/// Split `12. text` into `(true, "text")`; returns `None` when not ordered.
fn split_ordered(line: &str) -> Option<(bool, &str)> {
    let digits: String = line.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    let rest = &line[digits.len()..];
    let rest = rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") "))?;
    Some((true, rest.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_kind_gates() {
        assert!(SessionKind::Cron.wants_heartbeat());
        assert!(!SessionKind::Interactive.wants_heartbeat());
        assert!(SessionKind::Interactive.wants_boot());
        assert!(!SessionKind::Subagent.wants_persona());
        assert!(SessionKind::Interactive.wants_persona());
    }

    #[test]
    fn parses_dash_star_and_ordered_bullets() {
        let hb = Heartbeat::parse("- check mail\n* sweep inbox\n1. reply\n2) escalate\n");
        assert_eq!(hb.steps, vec![
            "check mail".to_string(),
            "sweep inbox".to_string(),
            "reply".to_string(),
            "escalate".to_string()
        ]);
    }

    #[test]
    fn empty_when_no_content() {
        assert!(Heartbeat::parse("").is_empty());
        assert!(Heartbeat::parse("   \n").is_empty());
    }
}
