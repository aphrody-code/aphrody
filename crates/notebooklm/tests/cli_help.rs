// SPDX-License-Identifier: Apache-2.0
//! Smoke check on the CLI surface: ensures the public `prelude` module
//! re-exports everything the CLI wires up, so a `aphrody notebooklm --help`
//! invocation never panics on a missing type at build time.
//!
//! We deliberately skip booting the actual `aphrody` binary here: the
//! integration is covered end-to-end by `cli/tests/*` in the umbrella crate.
//! Keeping this test in-crate guarantees the public surface stays callable
//! from a downstream consumer with a single `use notebooklm::prelude::*;`.

use notebooklm::prelude::*;

#[test]
fn prelude_exposes_artifact_kind_variants() {
    let _k = ArtifactKind::Audio;
    let _k = ArtifactKind::Report;
    let _k = ArtifactKind::Video;
    let _k = ArtifactKind::Quiz;
    let _k = ArtifactKind::MindMap;
    let _k = ArtifactKind::Flashcards;
    let _k = ArtifactKind::Infographic;
    let _k = ArtifactKind::SlideDeck;
    let _k = ArtifactKind::DataTable;
}

#[test]
fn prelude_exposes_research_mode() {
    let _ = ResearchMode::Fast;
    let _ = ResearchMode::Deep;
}

#[test]
fn prelude_exposes_source_kind() {
    let _ = SourceKind::Url;
    let _ = SourceKind::Text;
    let _ = SourceKind::File;
    let _ = SourceKind::YouTube;
}

#[test]
fn auth_from_env_errors_without_credentials() {
    // Snapshot + clear the env vars under test; restore them after so the
    // assertion does not leak into sibling tests running in parallel.
    let prior_token = std::env::var("NOTEBOOKLM_OAUTH_TOKEN").ok();
    let prior_cookies = std::env::var("NOTEBOOKLM_COOKIES").ok();
    // SAFETY: env mutations in tests are racy; we restore both vars below
    // before returning so the parent harness sees a stable environment.
    unsafe {
        std::env::remove_var("NOTEBOOKLM_OAUTH_TOKEN");
        std::env::remove_var("NOTEBOOKLM_COOKIES");
    }

    let err = Auth::from_env().expect_err("no creds = error");
    let msg = err.to_string();
    assert!(msg.contains("NOTEBOOKLM"), "unexpected message: {msg}");

    unsafe {
        if let Some(v) = prior_token {
            std::env::set_var("NOTEBOOKLM_OAUTH_TOKEN", v);
        }
        if let Some(v) = prior_cookies {
            std::env::set_var("NOTEBOOKLM_COOKIES", v);
        }
    }
}

#[test]
fn cookie_export_round_trip() {
    let json = r#"[
        {"name":"SAPISID","value":"xyz","domain":".google.com"},
        {"name":"__Secure-1PSID","value":"abc","domain":".google.com","path":"/","secure":true}
    ]"#;
    let auth = Auth::from_chromium_export(json).expect("valid cookie export");
    match &auth {
        Auth::Cookies(jar) => {
            assert!(jar.cookies.contains_key("SAPISID"));
            assert!(jar.cookies.contains_key("__Secure-1PSID"));
            let header = jar.header_value();
            assert!(header.contains("SAPISID=xyz"));
        },
        Auth::OAuthAccessToken(_) => panic!("expected cookie auth"),
    }
    assert_eq!(auth.flavour(), "cookies");
}

#[test]
fn cookie_export_rejects_missing_required_cookie() {
    let json = r#"[{"name":"HSID","value":"x","domain":".google.com"}]"#;
    let err = Auth::from_chromium_export(json).expect_err("missing SAPISID");
    let msg = err.to_string();
    assert!(msg.contains("SAPISID"), "unexpected message: {msg}");
}
