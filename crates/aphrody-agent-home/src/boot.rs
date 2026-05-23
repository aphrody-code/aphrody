// SPDX-License-Identifier: Apache-2.0
//! `BOOT.md` -> typed [`Boot`] (AH-7).
//!
//! The boot checklist runs once at the start of an interactive session
//! (distinct from the one-shot `BOOTSTRAP.md` ritual, which is deleted after
//! first-run setup). Reuses the bullet extractor from [`crate::heartbeat`].

use serde::{Deserialize, Serialize};

use crate::frontmatter;
use crate::heartbeat::extract_bullets;

/// Parsed `BOOT.md`: a start-of-session checklist.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Boot {
    /// Checklist items.
    pub steps: Vec<String>,
    /// Residual body.
    pub body: String,
}

impl Boot {
    /// Parse `BOOT.md` content. Never fails.
    #[must_use]
    pub fn parse(content: &str) -> Self {
        let (_fm, body) = frontmatter::split(content);
        Self {
            steps: extract_bullets(body),
            body: body.to_string(),
        }
    }

    /// True when there is nothing to do at boot.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty() && self.body.trim().is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_boot_checklist() {
        let b = Boot::parse("- read AGENTS.md\n- check open PRs\n");
        assert_eq!(b.steps, vec!["read AGENTS.md".to_string(), "check open PRs".to_string()]);
        assert!(!b.is_empty());
    }

    #[test]
    fn empty_boot_is_empty() {
        assert!(Boot::parse("").is_empty());
    }
}
