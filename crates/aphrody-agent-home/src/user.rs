// SPDX-License-Identifier: Apache-2.0
//! `USER.md` -> typed [`UserProfile`] (AH-3).
//!
//! Captures who the agent serves and how to address them. The display name is
//! resolved at runtime from `USER.md` (CLAUDE.md anonymisation rule: the
//! crate ships no real person's name; templates use a neutral placeholder).

use serde::{Deserialize, Serialize};

use crate::frontmatter::{self, FmValue};

/// Parsed `USER.md`: the human the agent serves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct UserProfile {
    /// How to address the user (e.g. their first name or a handle).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address_as: Option<String>,
    /// Pronouns to use.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pronouns: Option<String>,
    /// Timezone (IANA name, e.g. `Europe/Paris`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timezone: Option<String>,
    /// Preferred working language.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    /// Free-form preferences the agent should honour.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preferences: Vec<String>,
    /// Residual markdown body.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub body: String,
}

impl UserProfile {
    /// Parse `USER.md` content. Never fails; missing fields stay `None`.
    #[must_use]
    pub fn parse(content: &str) -> Self {
        let (fm, body) = frontmatter::split(content);
        let mut user = UserProfile {
            body: body.to_string(),
            ..UserProfile::default()
        };
        if let Some(fm) = fm {
            for (key, val) in frontmatter::parse(fm) {
                match (key.as_str(), val) {
                    ("address_as" | "name" | "call_me", FmValue::Scalar(s))
                        if !s.trim().is_empty() =>
                    {
                        user.address_as = Some(s.trim().to_string());
                    }
                    ("pronouns", FmValue::Scalar(s)) if !s.trim().is_empty() => {
                        user.pronouns = Some(s.trim().to_string());
                    }
                    ("timezone" | "tz", FmValue::Scalar(s)) if !s.trim().is_empty() => {
                        user.timezone = Some(s.trim().to_string());
                    }
                    ("language" | "lang", FmValue::Scalar(s)) if !s.trim().is_empty() => {
                        user.language = Some(s.trim().to_string());
                    }
                    ("preferences", FmValue::List(v)) => user.preferences = v,
                    ("preferences", FmValue::Scalar(s)) if !s.trim().is_empty() => {
                        user.preferences = vec![s.trim().to_string()];
                    }
                    _ => {}
                }
            }
        }
        user
    }

    /// Render the profile as a directive block for the system prompt.
    #[must_use]
    pub fn render_directives(&self) -> String {
        let mut out = String::new();
        if let Some(a) = &self.address_as {
            out.push_str("Address the user as ");
            out.push_str(a);
            if let Some(p) = &self.pronouns {
                out.push_str(" (");
                out.push_str(p);
                out.push(')');
            }
            out.push_str(".\n");
        } else if let Some(p) = &self.pronouns {
            out.push_str("User pronouns: ");
            out.push_str(p);
            out.push_str(".\n");
        }
        if let Some(tz) = &self.timezone {
            out.push_str("User timezone: ");
            out.push_str(tz);
            out.push_str(".\n");
        }
        if let Some(lang) = &self.language {
            out.push_str("Preferred language: ");
            out.push_str(lang);
            out.push_str(".\n");
        }
        if !self.preferences.is_empty() {
            out.push_str("Preferences:\n");
            for p in &self.preferences {
                out.push_str("- ");
                out.push_str(p);
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
    fn parses_all_fields() {
        let src = "---\naddress_as: Sam\npronouns: they/them\ntimezone: Europe/Paris\n\
                   language: French\npreferences:\n  - metric units\n  - no emoji\n---\nNotes.\n";
        let u = UserProfile::parse(src);
        assert_eq!(u.address_as.as_deref(), Some("Sam"));
        assert_eq!(u.pronouns.as_deref(), Some("they/them"));
        assert_eq!(u.timezone.as_deref(), Some("Europe/Paris"));
        assert_eq!(u.language.as_deref(), Some("French"));
        assert_eq!(u.preferences, vec!["metric units".to_string(), "no emoji".to_string()]);
        assert_eq!(u.body.trim(), "Notes.");
    }

    #[test]
    fn empty_yields_default() {
        assert_eq!(UserProfile::parse(""), UserProfile::default());
    }

    #[test]
    fn render_directives_addresses_user() {
        let u = UserProfile {
            address_as: Some("Sam".into()),
            pronouns: Some("they/them".into()),
            timezone: Some("UTC".into()),
            language: None,
            preferences: vec!["metric".into()],
            body: String::new(),
        };
        let d = u.render_directives();
        assert!(d.contains("Address the user as Sam (they/them)."));
        assert!(d.contains("User timezone: UTC."));
        assert!(d.contains("- metric"));
    }

    #[test]
    fn name_alias_maps_to_address_as() {
        let u = UserProfile::parse("---\nname: Alex\n---\n");
        assert_eq!(u.address_as.as_deref(), Some("Alex"));
    }
}
