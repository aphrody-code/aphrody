// SPDX-License-Identifier: Apache-2.0
//! aphrody-providers — single-source-of-truth `Provider` enum for the aphrody
//! three-only LLM whitelist.
//!
//! ## Whitelist
//!
//! The aphrody workspace supports **exactly three** providers and rejects
//! everything else at the type / deserialisation boundary:
//!
//! 1. [`Provider::Anthropic`]   — Claude family (`api.anthropic.com`).
//! 2. [`Provider::Gemini`]      — Google Gemini (`generativelanguage.googleapis.com`).
//! 3. [`Provider::Antigravity`] — Vertex AI Antigravity preview.
//!
//! This is **policy**, not an oversight: see the project memory
//! `project_aphrody_providers_3only`. Any pull request adding a fourth
//! variant (openai, mistral, cohere, grok, deepseek, groq, together,
//! perplexity, azure, …) must first revoke that memory.
//!
//! ## Why a dedicated micro-crate
//!
//! The same enum used to be duplicated verbatim in `aphrody-router`,
//! `aphrody-cost`, and `aphrody-rateguard`. They all agreed on the variant
//! set + lowercase wire format, but each carried slightly different helpers
//! (one had a strict custom `Deserialize`, two had `Display`, two had
//! `all()`). The duplication violated the workspace policy
//! `project_no_duplication_max_cohesion`. This crate is the single canonical
//! definition; the three consumers re-export it.
//!
//! ## Wire shape
//!
//! Serde serialises and deserialises [`Provider`] as a lowercase string —
//! `"anthropic"`, `"gemini"`, `"antigravity"`. Deserialisation is **strict**:
//! any other string is rejected with [`ProviderError::Unsupported`] surfaced
//! through `serde::de::Error::custom`. This means JSON / TOML / YAML payloads
//! containing `"openai"` fail to decode at the field boundary, never reach
//! application code, and produce a self-explanatory error message.
//!
//! ## Zero-cost composition
//!
//! `Provider` is `Copy + Eq + Hash + 'static`. Use it freely as a `HashMap`
//! key, a match scrutinee, a const expression, or an array index.

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Provider — three-only whitelist
// ---------------------------------------------------------------------------

/// The three LLM providers aphrody supports.
///
/// The enum is **closed**: see crate-level docs for the rationale. Adding a
/// new variant requires revoking the `project_aphrody_providers_3only`
/// project memory and updating every consumer crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    /// Anthropic Messages API — Claude family (Opus / Sonnet / Haiku).
    Anthropic,
    /// Google Gemini public API — `gemini-2.5-{pro,flash}` etc.
    Gemini,
    /// Google Cloud / Vertex AI Antigravity preview surface.
    Antigravity,
}

impl Provider {
    /// Canonical lowercase identifier used in JSON, CLI flags, log lines,
    /// and `HashMap` keys throughout the workspace.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Anthropic => "anthropic",
            Self::Gemini => "gemini",
            Self::Antigravity => "antigravity",
        }
    }

    /// All three variants in canonical order (Anthropic, Gemini, Antigravity).
    ///
    /// Useful for `for provider in Provider::all()` iteration in tests,
    /// pricing-table seeding, quota-bundle bootstrap, and CLI completion.
    #[must_use]
    pub fn all() -> [Self; 3] {
        [Self::Anthropic, Self::Gemini, Self::Antigravity]
    }

    /// Parse a provider name from a free-form string, rejecting anything
    /// outside the three-provider whitelist.
    ///
    /// The input is trimmed and ASCII-lowercased before comparison, so
    /// `" Anthropic "`, `"ANTHROPIC"`, and `"anthropic"` all succeed.
    ///
    /// # Errors
    /// - [`ProviderError::Unsupported`] for any value other than
    ///   `"anthropic"`, `"gemini"`, or `"antigravity"` (case-insensitive,
    ///   leading/trailing whitespace ignored).
    pub fn parse(name: &str) -> Result<Self, ProviderError> {
        match name.trim().to_ascii_lowercase().as_str() {
            "anthropic" => Ok(Self::Anthropic),
            "gemini" => Ok(Self::Gemini),
            "antigravity" => Ok(Self::Antigravity),
            other => Err(ProviderError::Unsupported(other.to_owned())),
        }
    }
}

impl std::fmt::Display for Provider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Provider {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::parse(&raw).map_err(serde::de::Error::custom)
    }
}

// ---------------------------------------------------------------------------
// ProviderError
// ---------------------------------------------------------------------------

/// Recoverable failures emitted by this crate.
///
/// The error message format is part of the public API: downstream tests
/// (`aphrody-router`, `aphrody-cost`, `aphrody-rateguard`, plus the CLI
/// fact-check skill) assert on the literal `"Unsupported provider:"` prefix.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum ProviderError {
    /// Caller passed a provider name that is not on the three-only whitelist.
    /// The inner string holds the rejected identifier as supplied (trimmed
    /// and lowercased) so the error message is deterministic across locales.
    #[error("Unsupported provider: {0} (aphrody allows only anthropic, gemini, antigravity)")]
    Unsupported(String),
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_str_roundtrip() {
        for variant in Provider::all() {
            let s = variant.as_str();
            let parsed = Provider::parse(s).expect("canonical str must parse");
            assert_eq!(parsed, variant, "as_str → parse round-trip failed for {variant:?}");
        }
    }

    #[test]
    fn parse_accepts_whitelist() {
        // Exact canonical form.
        assert_eq!(Provider::parse("anthropic").unwrap(), Provider::Anthropic);
        assert_eq!(Provider::parse("gemini").unwrap(), Provider::Gemini);
        assert_eq!(Provider::parse("antigravity").unwrap(), Provider::Antigravity);

        // Case-insensitive.
        assert_eq!(Provider::parse("ANTHROPIC").unwrap(), Provider::Anthropic);
        assert_eq!(Provider::parse("Gemini").unwrap(), Provider::Gemini);
        assert_eq!(Provider::parse("AntiGravity").unwrap(), Provider::Antigravity);

        // Whitespace trimming.
        assert_eq!(Provider::parse("  anthropic  ").unwrap(), Provider::Anthropic);
        assert_eq!(Provider::parse("\tgemini\n").unwrap(), Provider::Gemini);
    }

    #[test]
    fn parse_rejects_openai_with_message() {
        let err = Provider::parse("openai").expect_err("openai must be rejected");
        match err {
            ProviderError::Unsupported(name) => assert_eq!(name, "openai"),
        }
        // The Display message names the rejected identifier verbatim and
        // documents the allowed set — downstream callers grep for this prefix.
        let msg = Provider::parse("openai").unwrap_err().to_string();
        assert!(
            msg.starts_with("Unsupported provider: openai"),
            "unexpected error prefix: {msg}"
        );
        assert!(
            msg.contains("anthropic, gemini, antigravity"),
            "error must list the allowed set: {msg}"
        );
    }

    #[test]
    fn parse_rejects_unknown() {
        for bad in [
            "xai", "perplexity", "groq", "together", "mistral", "cohere",
            "deepseek", "azure", "openai-compat", "ollama", "",
        ] {
            let err = Provider::parse(bad).expect_err(&format!("must reject {bad}"));
            let ProviderError::Unsupported(name) = err;
            assert_eq!(name, bad.trim().to_ascii_lowercase());
        }
    }

    #[test]
    fn serde_serialize_lowercase() {
        let payload = serde_json::to_string(&Provider::Anthropic).unwrap();
        assert_eq!(payload, "\"anthropic\"");
        let payload = serde_json::to_string(&Provider::Gemini).unwrap();
        assert_eq!(payload, "\"gemini\"");
        let payload = serde_json::to_string(&Provider::Antigravity).unwrap();
        assert_eq!(payload, "\"antigravity\"");
    }

    #[test]
    fn serde_deserialize_accepts_whitelist() {
        let p: Provider = serde_json::from_str("\"anthropic\"").unwrap();
        assert_eq!(p, Provider::Anthropic);
        let p: Provider = serde_json::from_str("\"gemini\"").unwrap();
        assert_eq!(p, Provider::Gemini);
        let p: Provider = serde_json::from_str("\"antigravity\"").unwrap();
        assert_eq!(p, Provider::Antigravity);
    }

    #[test]
    fn serde_deserialize_rejects_openai() {
        let err = serde_json::from_str::<Provider>("\"openai\"").expect_err("must reject");
        let msg = err.to_string();
        assert!(
            msg.contains("Unsupported"),
            "error must contain 'Unsupported' marker, got: {msg}"
        );
        assert!(
            msg.contains("openai"),
            "error must name the rejected provider, got: {msg}"
        );
    }

    #[test]
    fn serde_deserialize_rejects_other_vendors() {
        for bad in ["xai", "perplexity", "groq", "together", "mistral", "cohere"] {
            let raw = format!("\"{bad}\"");
            let err = serde_json::from_str::<Provider>(&raw)
                .expect_err(&format!("must reject {bad}"));
            assert!(
                err.to_string().contains(bad),
                "{bad}: error must name the provider, got: {err}"
            );
        }
    }

    #[test]
    fn display_matches_as_str() {
        for variant in Provider::all() {
            assert_eq!(variant.to_string(), variant.as_str());
        }
        assert_eq!(format!("{}", Provider::Anthropic), "anthropic");
        assert_eq!(format!("{}", Provider::Gemini), "gemini");
        assert_eq!(format!("{}", Provider::Antigravity), "antigravity");
    }

    #[test]
    fn all_has_three_variants_in_order() {
        let all = Provider::all();
        assert_eq!(all.len(), 3);
        assert_eq!(all[0], Provider::Anthropic);
        assert_eq!(all[1], Provider::Gemini);
        assert_eq!(all[2], Provider::Antigravity);
    }

    #[test]
    fn provider_is_copy_and_hash() {
        // Smoke: verifies the derives stay intact at the type-system level.
        fn assert_copy<T: Copy>() {}
        fn assert_hash<T: std::hash::Hash + Eq>() {}
        assert_copy::<Provider>();
        assert_hash::<Provider>();
        let _ = Provider::Anthropic;
        let _ = Provider::Anthropic; // still usable after "move"
    }

    #[test]
    fn provider_error_message_is_stable() {
        // The exact wording is part of the public contract — downstream
        // crates (rate guards, budget exceeded handlers) grep this prefix.
        let e = ProviderError::Unsupported("openai".to_string());
        assert_eq!(
            e.to_string(),
            "Unsupported provider: openai (aphrody allows only anthropic, gemini, antigravity)"
        );
    }
}
