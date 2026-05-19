// SPDX-License-Identifier: Apache-2.0
//! Integration tests for `aphrody_permissions::PermissionEngine`.
//!
//! Each test exercises one slice of the public contract:
//!
//! * settings.json parsing (`from_settings`)
//! * deny > allow precedence
//! * glob matchers (Bash + path tools)
//! * bare rules (no matcher) matching every invocation
//! * three-layer merge (local > project > user)
//! * mode short-circuits (`BypassPermissions`, `Plan`)
//! * rule parse / glob validation errors

use std::path::PathBuf;

use aphrody_permissions::{
    PermissionConfig, PermissionDecision, PermissionEngine, PermissionError, PermissionMode,
    ToolRule,
};
use serde_json::json;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn engine_from(allow: &[&str], ask: &[&str], deny: &[&str], mode: PermissionMode) -> PermissionEngine {
    let cfg = PermissionConfig {
        mode,
        default_mode: PermissionMode::Default,
        allow: allow.iter().map(|s| (*s).to_string()).collect(),
        ask: ask.iter().map(|s| (*s).to_string()).collect(),
        deny: deny.iter().map(|s| (*s).to_string()).collect(),
        additional_directories: vec![],
    };
    PermissionEngine::from_config(cfg).expect("engine builds")
}

// ---------------------------------------------------------------------------
// 1. Parse a realistic settings.json block end-to-end.
// ---------------------------------------------------------------------------

#[test]
fn parse_settings_block_full() {
    let raw = json!({
        "permissions": {
            "mode": "default",
            "defaultMode": "default",
            "allow": ["Read", "Bash(git status)"],
            "ask":   ["WebFetch"],
            "deny":  ["Bash(rm -rf /*)", "Read(/etc/shadow)"],
            "additionalDirectories": ["/tmp/aphrody-work"]
        }
    });
    let engine = PermissionEngine::from_settings(&raw).expect("settings parse");
    assert_eq!(engine.mode(), PermissionMode::Default);
    assert_eq!(engine.default_mode(), PermissionMode::Default);
    assert_eq!(
        engine.additional_directories(),
        &[PathBuf::from("/tmp/aphrody-work")]
    );

    // Bash(git status) — allowed.
    let dec = engine.evaluate("Bash", &json!({"command": "git status"}));
    assert!(matches!(dec, PermissionDecision::Allow { .. }), "got {dec:?}");

    // WebFetch — ask.
    let dec = engine.evaluate("WebFetch", &json!({"url": "https://example.com"}));
    assert!(matches!(dec, PermissionDecision::Ask { .. }), "got {dec:?}");

    // Deny: glob match on Bash(rm -rf /*).
    let dec = engine.evaluate("Bash", &json!({"command": "rm -rf /tmp/data"}));
    assert!(matches!(dec, PermissionDecision::Deny { .. }), "got {dec:?}");
}

// ---------------------------------------------------------------------------
// 2. deny beats allow when both rule lists match.
// ---------------------------------------------------------------------------

#[test]
fn deny_overrides_allow() {
    let engine = engine_from(
        &["Read", "Read(/etc/**)"],
        &[],
        &["Read(/etc/shadow)"],
        PermissionMode::Default,
    );
    let dec = engine.evaluate("Read", &json!({"file_path": "/etc/shadow"}));
    assert!(matches!(dec, PermissionDecision::Deny { .. }), "deny must win, got {dec:?}");
}

// ---------------------------------------------------------------------------
// 3. Glob matcher matches concrete Bash invocations.
// ---------------------------------------------------------------------------

#[test]
fn rule_with_glob_matches_path() {
    let engine = engine_from(&[], &[], &["Bash(rm -rf /*)"], PermissionMode::Default);
    let dec = engine.evaluate("Bash", &json!({"command": "rm -rf /var/log"}));
    assert!(dec.is_deny(), "expected deny, got {dec:?}");

    // A command that doesn't match the glob falls through to default_mode=Ask.
    let dec = engine.evaluate("Bash", &json!({"command": "ls -la"}));
    assert!(dec.is_ask(), "expected ask fallback, got {dec:?}");
}

// ---------------------------------------------------------------------------
// 4. Bare rule (no matcher) matches every invocation of that tool.
// ---------------------------------------------------------------------------

#[test]
fn rule_no_matcher_matches_all_invocations() {
    let engine = engine_from(&["WebFetch"], &[], &[], PermissionMode::Default);
    for url in ["https://a.example", "https://b.example/path?q=1"] {
        let dec = engine.evaluate("WebFetch", &json!({"url": url}));
        assert!(dec.is_allow(), "WebFetch must be auto-allowed for {url}, got {dec:?}");
    }
    // Different tool — no match, falls through to Ask.
    let dec = engine.evaluate("Read", &json!({"file_path": "src/lib.rs"}));
    assert!(dec.is_ask(), "Read should fall through, got {dec:?}");
}

// ---------------------------------------------------------------------------
// 5. Merge precedence: local > project > user, and deny is preserved
//    across layers.
// ---------------------------------------------------------------------------

#[test]
fn merge_precedence_local_over_project_over_user() {
    let user = engine_from(&["Read"], &[], &[], PermissionMode::Default);
    let project = engine_from(&[], &[], &["Bash(rm -rf /*)"], PermissionMode::AcceptEdits);
    let local = engine_from(&["WebFetch"], &[], &[], PermissionMode::Plan);

    let merged = PermissionEngine::merge(local, project, user);
    // Mode: local wins (Plan).
    assert_eq!(merged.mode(), PermissionMode::Plan);

    // Read fallback: Plan mode treats non-write tools via default_mode (Default
    // here) → Ask.
    let dec = merged.evaluate("Read", &json!({"file_path": "src/lib.rs"}));
    assert!(dec.is_allow(), "user allow rule for Read must survive merge, got {dec:?}");

    // Project's Bash deny survives.
    let dec = merged.evaluate("Bash", &json!({"command": "rm -rf /home/user"}));
    assert!(dec.is_deny(), "project deny must survive merge, got {dec:?}");

    // Local's WebFetch allow survives.
    let dec = merged.evaluate("WebFetch", &json!({}));
    assert!(dec.is_allow(), "local allow must survive merge, got {dec:?}");
}

// ---------------------------------------------------------------------------
// 6. BypassPermissions short-circuits everything to Allow.
// ---------------------------------------------------------------------------

#[test]
fn bypass_permissions_mode_allows_everything() {
    let engine = engine_from(
        &[],
        &[],
        &["Bash(*)", "Write(*)", "WebFetch"],
        PermissionMode::BypassPermissions,
    );
    for (tool, args) in [
        ("Bash", json!({"command": "rm -rf /"})),
        ("Write", json!({"file_path": "/etc/passwd"})),
        ("WebFetch", json!({"url": "https://evil.example"})),
    ] {
        let dec = engine.evaluate(tool, &args);
        assert!(
            dec.is_allow(),
            "bypass mode must allow {tool}, got {dec:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// 7. Plan mode denies write-class tools by default.
// ---------------------------------------------------------------------------

#[test]
fn plan_mode_denies_write_tools_by_default() {
    let engine = engine_from(&[], &[], &[], PermissionMode::Plan);
    for tool in ["Edit", "Write", "NotebookEdit", "MultiEdit"] {
        let dec = engine.evaluate(tool, &json!({"file_path": "src/lib.rs"}));
        assert!(
            dec.is_deny(),
            "plan mode must deny write tool {tool}, got {dec:?}"
        );
    }
    // Read remains askable.
    let dec = engine.evaluate("Read", &json!({"file_path": "src/lib.rs"}));
    assert!(dec.is_ask(), "Read should be ask in plan mode, got {dec:?}");
}

// ---------------------------------------------------------------------------
// 8. Invalid glob inside a rule surfaces a structured error.
// ---------------------------------------------------------------------------

#[test]
fn invalid_glob_returns_error() {
    // Unbalanced character class `[abc` is rejected by glob::Pattern.
    let cfg = PermissionConfig {
        mode: PermissionMode::Default,
        default_mode: PermissionMode::Default,
        allow: vec!["Read([abc)".to_string()],
        ask: vec![],
        deny: vec![],
        additional_directories: vec![],
    };
    let err = PermissionEngine::from_config(cfg).expect_err("must reject invalid glob");
    assert!(matches!(err, PermissionError::InvalidGlob { .. }), "got {err:?}");

    // Bare-( prefix is rejected as an invalid rule shape.
    let err = ToolRule::parse("(Read)").expect_err("bare paren prefix invalid");
    assert!(matches!(err, PermissionError::InvalidRule(_)), "got {err:?}");
}

// ---------------------------------------------------------------------------
// 9. evaluate_path() mirrors evaluate() for path-bearing tools.
// ---------------------------------------------------------------------------

#[test]
fn evaluate_path_mirrors_json_evaluate() {
    let engine = engine_from(
        &["Read(src/**)"],
        &[],
        &["Read(src/secret.rs)"],
        PermissionMode::Default,
    );
    let dec = engine.evaluate_path("Read", std::path::Path::new("src/lib.rs"));
    assert!(dec.is_allow(), "src/lib.rs should be allowed, got {dec:?}");
    let dec = engine.evaluate_path("Read", std::path::Path::new("src/secret.rs"));
    assert!(dec.is_deny(), "src/secret.rs should be denied, got {dec:?}");
}
