// SPDX-License-Identifier: Apache-2.0
//! End-to-end smoke tests for `aphrody-prompts`.
//!
//! These exercise the public surface (registry + scrubber) the way the
//! aphrody runtime does: register templates and partials, render against
//! JSON contexts, validate ahead of time, and scrub PII out of arbitrary
//! prompt fragments.

use aphrody_prompts::{
    DEFAULT_PII_PATTERNS, PromptError, PromptRegistry, PromptScrubber, PromptTemplate,
};
use serde_json::json;

#[test]
fn register_and_render_template() {
    let mut reg = PromptRegistry::new();
    reg.register(PromptTemplate::new(
        "greet",
        "Hello, {{ name }}! You are {{ role }}.",
        ["name", "role"],
    ))
    .expect("register");

    let out = reg
        .render("greet", &json!({"name": "Aphrody", "role": "engineer"}))
        .expect("render");
    assert_eq!(out, "Hello, Aphrody! You are engineer.");
}

#[test]
fn missing_var_returns_error_listing_missing() {
    let mut reg = PromptRegistry::new();
    reg.register(PromptTemplate::new(
        "ask",
        "{{ question }} ({{ context }})",
        ["question", "context", "user"],
    ))
    .unwrap();

    let err = reg
        .render("ask", &json!({"question": "why?"}))
        .expect_err("must reject");
    match err {
        PromptError::MissingVars(missing) => {
            assert_eq!(missing, vec!["context".to_string(), "user".to_string()]);
        }
        other => panic!("unexpected error variant: {other:?}"),
    }
}

#[test]
fn partial_include_works() {
    let mut reg = PromptRegistry::new();
    reg.register_partial("header", "=== SYSTEM ===")
        .expect("partial");
    reg.register(PromptTemplate::new(
        "framed",
        "{% include 'header' %}\nbody: {{ msg }}",
        ["msg"],
    ))
    .expect("register");

    let out = reg.render("framed", &json!({"msg": "ok"})).unwrap();
    assert_eq!(out, "=== SYSTEM ===\nbody: ok");
}

#[test]
fn scrub_redacts_email() {
    let scrub = PromptScrubber::with_defaults().expect("defaults");
    let res = scrub.scrub("ping alice@example.com please");
    assert!(
        res.redacted.contains("[REDACTED:email]"),
        "redacted={}",
        res.redacted
    );
    assert!(res.matches.iter().any(|m| m.kind == "email"));
}

#[test]
fn scrub_redacts_multiple_pii_types() {
    let scrub = PromptScrubber::with_defaults().expect("defaults");
    // email + IPv4 + phone in a single string.
    let input = "Mail bob@acme.io from 10.0.0.5, call +14155552671.";
    let res = scrub.scrub(input);

    let kinds: std::collections::BTreeSet<_> =
        res.matches.iter().map(|m| m.kind.as_str()).collect();
    assert!(kinds.contains("email"), "kinds={kinds:?}");
    assert!(kinds.contains("ipv4"), "kinds={kinds:?}");
    assert!(kinds.contains("phone"), "kinds={kinds:?}");
    // Original PII tokens must be gone.
    assert!(!res.redacted.contains("bob@acme.io"));
    assert!(!res.redacted.contains("10.0.0.5"));
}

#[test]
fn validate_rejects_unbalanced_jinja() {
    let mut reg = PromptRegistry::new();
    // `register` itself should reject — but we also assert via validate
    // if a malformed template ever sneaks into the map.
    let err = reg
        .register(PromptTemplate::new(
            "broken",
            "Hello, {{ name ",
            ["name"],
        ))
        .expect_err("must fail to register");
    match err {
        PromptError::JinjaError(_) => {}
        other => panic!("unexpected error: {other:?}"),
    }

    // And validate on a non-existent template also surfaces a clean error.
    let err2 = reg.validate("broken").unwrap_err();
    match err2 {
        PromptError::TemplateNotFound(name) => assert_eq!(name, "broken"),
        other => panic!("unexpected error: {other:?}"),
    }
}

#[test]
fn template_with_loop_renders_list() {
    let mut reg = PromptRegistry::new();
    reg.register(PromptTemplate::new(
        "bulleted",
        "{% for x in items %}- {{ x }}\n{% endfor %}",
        ["items"],
    ))
    .unwrap();

    let out = reg
        .render("bulleted", &json!({"items": ["one", "two", "three"]}))
        .unwrap();
    assert_eq!(out, "- one\n- two\n- three\n");
}

#[test]
fn pii_scrubber_preserves_non_pii_text() {
    let scrub = PromptScrubber::with_defaults().unwrap();
    let input = "the quick brown fox jumps over a lazy dog";
    let res = scrub.scrub(input);
    assert_eq!(res.redacted, input);
    assert!(res.matches.is_empty());
}

#[test]
fn default_pii_pattern_set_is_at_least_five() {
    assert!(DEFAULT_PII_PATTERNS.len() >= 5);
}
