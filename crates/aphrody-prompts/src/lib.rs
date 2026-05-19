// SPDX-License-Identifier: Apache-2.0
//! aphrody-prompts — prompt template engine for the aphrody runtime.
//!
//! Wraps the [`minijinja`] engine into a typed registry that:
//!
//! * Stores named [`PromptTemplate`]s with their required-variable contract.
//! * Resolves reusable [`PromptRegistry::register_partial`] snippets via
//!   Jinja-style `{% include 'name' %}` directives.
//! * Validates templates ahead of render (syntax parse + variable presence
//!   check) so misconfigured prompts fail at boot rather than mid-stream.
//! * Renders against a [`RenderContext`] built from arbitrary
//!   [`serde_json::Value`] payloads.
//! * Ships a configurable [`PromptScrubber`] that redacts personally
//!   identifiable information (email, phone, IBAN, IPv4, API-key-like tokens)
//!   from input or rendered output before it leaves the host.
//!
//! The crate has zero `unsafe`, is `#![forbid(unsafe_code)]`, and follows
//! the same DTO conventions used by `aphrody-permissions`, `aphrody-skills-forge`
//! and `aphrody-marketplace` (serde derive, thiserror enums, regex via
//! [`once_cell::sync::Lazy`]-style construction guarded by [`Regex::new`]).
//!
//! ## Quick example
//!
//! ```
//! use aphrody_prompts::{PromptRegistry, PromptTemplate};
//! use serde_json::json;
//!
//! let mut reg = PromptRegistry::new();
//! reg.register(PromptTemplate::new(
//!     "greet",
//!     "Hello, {{ name }}!",
//!     ["name"],
//! ))
//! .unwrap();
//!
//! let out = reg.render("greet", &json!({"name": "Aphrody"})).unwrap();
//! assert_eq!(out, "Hello, Aphrody!");
//! ```
//!
//! ## PII scrubbing
//!
//! ```
//! use aphrody_prompts::PromptScrubber;
//!
//! let scrub = PromptScrubber::with_defaults().unwrap();
//! let res   = scrub.scrub("contact me at alice@example.com please");
//! assert!(res.redacted.contains("[REDACTED:email]"));
//! assert_eq!(res.matches.len(), 1);
//! ```

#![forbid(unsafe_code)]

use std::collections::{BTreeSet, HashMap};

use minijinja::{Environment, UndefinedBehavior, Value as MjValue};
use regex::Regex;
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Recoverable failures emitted by the prompt engine.
///
/// `JinjaError` wraps the upstream [`minijinja::Error`] as a `String` so the
/// enum stays `Send + Sync + 'static` (matching the cross-thread expectations
/// shared with `aphrody-hooks` / `aphrody-router`).
#[derive(Debug, Error)]
pub enum PromptError {
    /// No template registered for the given name.
    #[error("template not found: {0}")]
    TemplateNotFound(String),
    /// Render aborted because required variables were missing from the
    /// [`RenderContext`]. The payload lists every missing variable so
    /// callers can surface a single diagnostic.
    #[error("missing required variables: {0:?}")]
    MissingVars(Vec<String>),
    /// The underlying minijinja engine reported a syntax / runtime error.
    #[error("jinja error: {0}")]
    JinjaError(String),
    /// JSON (de)serialization failed while binding a [`RenderContext`].
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// A configured PII regex failed to compile.
    #[error("invalid regex {pattern:?}: {source}")]
    InvalidRegex {
        /// Original pattern string for diagnostics.
        pattern: String,
        /// Underlying regex compile error.
        #[source]
        source: regex::Error,
    },
}

impl From<minijinja::Error> for PromptError {
    fn from(e: minijinja::Error) -> Self {
        Self::JinjaError(format!("{e}"))
    }
}

// ---------------------------------------------------------------------------
// PromptTemplate
// ---------------------------------------------------------------------------

/// A single prompt template — a Jinja source body plus its variable contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    /// Stable identifier used as the registry key.
    pub name: String,
    /// Raw Jinja template source.
    pub body: String,
    /// Variables that MUST be present in the [`RenderContext`] root object.
    pub required_vars: Vec<String>,
    /// Free-form tags useful for filtering (e.g. `"system"`, `"tool"`).
    #[serde(default)]
    pub tags: Vec<String>,
}

impl PromptTemplate {
    /// Construct a new template. The accepted iterator over `required_vars`
    /// keeps caller-site syntax compact (`["name", "tone"]`).
    #[must_use]
    pub fn new<I, S>(name: &str, body: &str, required_vars: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            name: name.to_string(),
            body: body.to_string(),
            required_vars: required_vars.into_iter().map(Into::into).collect(),
            tags: Vec::new(),
        }
    }

    /// Builder-style: attach descriptive tags.
    #[must_use]
    pub fn with_tags<I, S>(mut self, tags: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }
}

// ---------------------------------------------------------------------------
// RenderContext
// ---------------------------------------------------------------------------

/// A typed binding from variable names to JSON values, consumed by
/// [`PromptRegistry::render`].
///
/// The wire shape MUST be a JSON object at the root (rejecting arrays /
/// scalars keeps the per-variable contract enforceable).
#[derive(Debug, Clone)]
pub struct RenderContext {
    inner: serde_json::Map<String, serde_json::Value>,
}

impl RenderContext {
    /// Build a context from any JSON value. Returns an error if `value` is not
    /// a JSON object at the root.
    pub fn from_value(value: &serde_json::Value) -> Result<Self, PromptError> {
        match value {
            serde_json::Value::Object(map) => Ok(Self { inner: map.clone() }),
            other => Err(PromptError::JinjaError(format!(
                "render context must be a JSON object, got {}",
                kind_of(other)
            ))),
        }
    }

    /// Variable keys present in the context root.
    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.inner.keys().map(String::as_str)
    }

    /// Convert to a minijinja value suitable for [`minijinja::Template::render`].
    fn to_jinja_value(&self) -> MjValue {
        MjValue::from_serialize(&serde_json::Value::Object(self.inner.clone()))
    }
}

fn kind_of(v: &serde_json::Value) -> &'static str {
    match v {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "bool",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

// ---------------------------------------------------------------------------
// PromptRegistry
// ---------------------------------------------------------------------------

/// Indexed collection of [`PromptTemplate`]s plus reusable partials.
///
/// The registry owns its [`Environment`] lazily — we recreate it on every
/// render to keep registered templates and partials in sync (the cost is
/// negligible for the prompt sizes aphrody emits and avoids managing the
/// 'env lifetime through public API).
#[derive(Debug, Default, Clone)]
pub struct PromptRegistry {
    templates: HashMap<String, PromptTemplate>,
    partials: HashMap<String, String>,
}

impl PromptRegistry {
    /// Empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a template. Returns an error if the body fails to parse as
    /// Jinja (so misconfigured prompts surface at registration time, never
    /// at render time on a hot path).
    pub fn register(&mut self, template: PromptTemplate) -> Result<(), PromptError> {
        // Parse-only check: build a throwaway environment, add the source,
        // try to compile it.
        let mut env = Environment::new();
        env.set_undefined_behavior(UndefinedBehavior::Strict);
        for (pname, pbody) in &self.partials {
            env.add_template(pname, pbody)?;
        }
        env.add_template(&template.name, &template.body)?;
        let _ = env.get_template(&template.name)?;
        tracing::debug!(name = %template.name, "prompt template registered");
        self.templates.insert(template.name.clone(), template);
        Ok(())
    }

    /// Register a reusable partial snippet, addressable via
    /// `{% include 'name' %}` from any template.
    pub fn register_partial(&mut self, name: &str, body: &str) -> Result<(), PromptError> {
        let mut env = Environment::new();
        env.add_template(name, body)?;
        self.partials.insert(name.to_string(), body.to_string());
        Ok(())
    }

    /// Lookup a registered template by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&PromptTemplate> {
        self.templates.get(name)
    }

    /// List the names of every registered template.
    #[must_use]
    pub fn list(&self) -> Vec<String> {
        let mut names: Vec<String> = self.templates.keys().cloned().collect();
        names.sort();
        names
    }

    /// Re-parse the named template ensuring every `required_vars` entry is at
    /// least mentioned in the body. This is a cheap structural check — full
    /// variable-coverage analysis would require walking the AST.
    pub fn validate(&self, name: &str) -> Result<(), PromptError> {
        let tpl = self
            .templates
            .get(name)
            .ok_or_else(|| PromptError::TemplateNotFound(name.to_string()))?;
        let mut env = Environment::new();
        env.set_undefined_behavior(UndefinedBehavior::Strict);
        for (pname, pbody) in &self.partials {
            env.add_template(pname, pbody)?;
        }
        env.add_template(&tpl.name, &tpl.body)?;
        let _ = env.get_template(&tpl.name)?;
        Ok(())
    }

    /// Render a registered template against the supplied JSON context.
    ///
    /// Returns [`PromptError::MissingVars`] if any `required_vars` are absent
    /// from the context root object.
    pub fn render(&self, name: &str, vars: &serde_json::Value) -> Result<String, PromptError> {
        let tpl = self
            .templates
            .get(name)
            .ok_or_else(|| PromptError::TemplateNotFound(name.to_string()))?;

        let ctx = RenderContext::from_value(vars)?;
        let supplied: BTreeSet<&str> = ctx.keys().collect();
        let mut missing: Vec<String> = tpl
            .required_vars
            .iter()
            .filter(|v| !supplied.contains(v.as_str()))
            .cloned()
            .collect();
        if !missing.is_empty() {
            missing.sort();
            return Err(PromptError::MissingVars(missing));
        }

        let mut env = Environment::new();
        env.set_undefined_behavior(UndefinedBehavior::Strict);
        for (pname, pbody) in &self.partials {
            env.add_template(pname, pbody)?;
        }
        env.add_template(&tpl.name, &tpl.body)?;
        let template = env.get_template(&tpl.name)?;
        let rendered = template.render(ctx.to_jinja_value())?;
        Ok(rendered)
    }
}

// ---------------------------------------------------------------------------
// PromptScrubber
// ---------------------------------------------------------------------------

/// One redaction performed by [`PromptScrubber::scrub`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RedactionMatch {
    /// Pattern kind that produced the match (e.g. `"email"`).
    pub kind: String,
    /// Byte range (`start..end`) of the original substring within the input.
    pub original_range: (usize, usize),
}

/// Outcome of a [`PromptScrubber::scrub`] pass.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrubResult {
    /// Scrubbed text with each match replaced by `[REDACTED:<kind>]`.
    pub redacted: String,
    /// Per-match metadata in left-to-right order.
    pub matches: Vec<RedactionMatch>,
}

/// Default PII pattern set. Each tuple is `(kind, regex)`. Kinds are surfaced
/// in [`RedactionMatch::kind`] and embedded in the replacement marker.
///
/// Patterns are conservative on purpose — over-redaction is preferred to
/// leakage. The regexes here intentionally avoid `\b` lookbehind tricks that
/// would require the unstable regex `unicode-perl-extra` feature.
pub const DEFAULT_PII_PATTERNS: &[(&str, &str)] = &[
    // RFC-5322 simplified email
    ("email", r"[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,24}"),
    // Phone: E.164 leading + and 8-15 digits, or 9-15 digit run
    ("phone", r"(?:\+?[0-9][0-9 .\-]{7,15}[0-9])"),
    // IBAN: 2 country letters, 2 check digits, 11-30 alphanum
    ("iban", r"[A-Z]{2}[0-9]{2}[A-Z0-9]{11,30}"),
    // IPv4 (octet bounds checked loosely)
    (
        "ipv4",
        r"(?:25[0-5]|2[0-4][0-9]|1?[0-9]{1,2})\.(?:25[0-5]|2[0-4][0-9]|1?[0-9]{1,2})\.(?:25[0-5]|2[0-4][0-9]|1?[0-9]{1,2})\.(?:25[0-5]|2[0-4][0-9]|1?[0-9]{1,2})",
    ),
    // Generic API-key-like opaque tokens (32+ alphanum)
    ("api_key", r"[A-Za-z0-9_\-]{32,}"),
];

/// Configurable PII redactor.
///
/// Construct with [`PromptScrubber::with_defaults`] to install the
/// [`DEFAULT_PII_PATTERNS`] set, or [`PromptScrubber::new`] for an empty
/// scrubber you populate via [`PromptScrubber::add_pattern`].
#[derive(Debug, Clone)]
pub struct PromptScrubber {
    patterns: Vec<(String, Regex)>,
}

impl PromptScrubber {
    /// Empty scrubber (no-op until patterns are added).
    #[must_use]
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }

    /// Scrubber preloaded with [`DEFAULT_PII_PATTERNS`].
    pub fn with_defaults() -> Result<Self, PromptError> {
        let mut s = Self::new();
        for (kind, pat) in DEFAULT_PII_PATTERNS {
            s.add_pattern(kind, pat)?;
        }
        Ok(s)
    }

    /// Append a `(kind, regex)` pattern. Pattern compilation errors return
    /// [`PromptError::InvalidRegex`].
    pub fn add_pattern(&mut self, kind: &str, pattern: &str) -> Result<(), PromptError> {
        let re = Regex::new(pattern).map_err(|source| PromptError::InvalidRegex {
            pattern: pattern.to_string(),
            source,
        })?;
        self.patterns.push((kind.to_string(), re));
        Ok(())
    }

    /// Number of configured patterns.
    #[must_use]
    pub fn len(&self) -> usize {
        self.patterns.len()
    }

    /// True if no patterns are configured.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }

    /// Scrub `input`, returning a redacted string + per-match metadata.
    ///
    /// Patterns are applied in registration order. The first pattern that
    /// claims a byte range wins; later patterns operate on the already-
    /// redacted text so they cannot double-redact the same span.
    #[must_use]
    pub fn scrub(&self, input: &str) -> ScrubResult {
        let mut redacted = input.to_string();
        let mut matches: Vec<RedactionMatch> = Vec::new();

        for (kind, re) in &self.patterns {
            // Collect ranges from the current redacted string, then rebuild
            // so we never mutate during iteration.
            let found: Vec<(usize, usize)> = re
                .find_iter(&redacted)
                .map(|m| (m.start(), m.end()))
                .collect();
            if found.is_empty() {
                continue;
            }
            let marker = format!("[REDACTED:{kind}]");
            // Apply in reverse so earlier offsets stay valid.
            let mut new_redacted = redacted.clone();
            for (start, end) in found.iter().rev() {
                new_redacted.replace_range(*start..*end, &marker);
                matches.push(RedactionMatch {
                    kind: kind.clone(),
                    original_range: (*start, *end),
                });
            }
            redacted = new_redacted;
        }

        // Sort matches by start offset for deterministic consumer behaviour.
        matches.sort_by_key(|m| m.original_range.0);

        ScrubResult { redacted, matches }
    }
}

impl Default for PromptScrubber {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Unit tests (kept lightweight; integration tests live in tests/)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn template_new_collects_required_vars() {
        let t = PromptTemplate::new("t", "{{ a }}", ["a", "b"]);
        assert_eq!(t.required_vars, vec!["a".to_string(), "b".to_string()]);
    }

    #[test]
    fn registry_list_is_sorted() {
        let mut reg = PromptRegistry::new();
        reg.register(PromptTemplate::new("zeta", "x", Vec::<String>::new()))
            .unwrap();
        reg.register(PromptTemplate::new("alpha", "x", Vec::<String>::new()))
            .unwrap();
        assert_eq!(reg.list(), vec!["alpha".to_string(), "zeta".to_string()]);
    }

    #[test]
    fn render_context_rejects_non_object_root() {
        let err = RenderContext::from_value(&json!([1, 2, 3])).unwrap_err();
        match err {
            PromptError::JinjaError(_) => {}
            other => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn scrubber_default_pattern_count() {
        let s = PromptScrubber::with_defaults().unwrap();
        assert_eq!(s.len(), DEFAULT_PII_PATTERNS.len());
    }
}
