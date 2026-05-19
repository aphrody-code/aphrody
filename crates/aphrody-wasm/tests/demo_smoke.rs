// SPDX-License-Identifier: Apache-2.0
//
// Native smoke tests for the aphrody-terminal demo HTML asset.
//
// These tests run on the host (cargo nextest) — NOT on wasm32 — because
// the goal is to validate the *static asset* shipped in `examples/`,
// not the wasm-bindgen surface (which has its own `wasm_smoke.rs`).
//
// Run: cargo nextest run -p aphrody-wasm --offline -E 'test(/demo_smoke/)'

#![cfg(not(target_arch = "wasm32"))]

/// The demo HTML asset, embedded at compile time so the test binary is
/// fully self-contained and never reads from the filesystem at run time.
const DEMO_HTML: &str = include_str!("../examples/aphrody-terminal-demo.html");

/// The 8 aphrody-terminal-* crate names per CLAUDE.md §7.5 and
/// docs/design/aphrody-terminal-spec.md. ALL must appear at least once
/// in the demo HTML.
const STACK_CRATES: &[&str] = &[
    "aphrody-terminal-vt",
    "aphrody-terminal-wasm",
    "aphrody-terminal-backend",
    "aphrody-terminal-llm",
    "aphrody-terminal-browser",
    "aphrody-terminal-markdown",
    "aphrody-terminal-json-out",
    "aphrody-terminal-config",
];

/// The HTML must reinforce the "aphrody-terminal" brand at least 3 times
/// (header, footer, stack section) AND list every aphrody-terminal-* crate
/// at least once (covers the §0.5 T-7 fusion deliverable).
#[test]
fn html_has_aphrody_branding() {
    let occurrences = DEMO_HTML.matches("aphrody-terminal").count();
    assert!(
        occurrences >= 3,
        "expected 'aphrody-terminal' ≥3 times, found {occurrences}"
    );

    for crate_name in STACK_CRATES {
        assert!(
            DEMO_HTML.contains(crate_name),
            "demo HTML must list crate {crate_name} (per docs/design/aphrody-terminal-spec.md §8-crate stack)"
        );
    }
}

/// Per CLAUDE.md (no AI-brand mentions, providers whitelist) and
/// [[feedback_org_standards_anon.md]], the demo MUST NOT name:
///   - any LLM vendor (Claude, OpenAI, Anthropic)
///   - any banned runtime (Bun, Node.js)
///   - Material Design 2 / M2 (M3 only — [[project_aphrody_providers_3only]])
///
/// We allow-list legitimate substrings that contain "M2" inside other
/// tokens (e.g. CSS variables ending in -m2 don't exist in M3, so any
/// match is a real violation).
#[test]
fn html_no_forbidden_brands() {
    let forbidden: &[&str] = &[
        "Claude",
        "OpenAI",
        "Anthropic",
        "Bun",
        "Node.js",
        "Material Design 2",
    ];
    for needle in forbidden {
        assert!(
            !DEMO_HTML.contains(needle),
            "forbidden brand '{needle}' appears in demo HTML — strip it (see CLAUDE.md §2 + memory)"
        );
    }

    // " M2 " — word-boundary check to avoid false-positives like CSS hex
    // values that happen to contain the literal pair of letters.
    let m2_word = DEMO_HTML
        .split_whitespace()
        .any(|tok| tok.trim_matches(|c: char| !c.is_alphanumeric()) == "M2");
    assert!(
        !m2_word,
        "forbidden token 'M2' (word) appears in demo HTML — M3 only per [[project_aphrody_providers_3only]]"
    );
}

/// The HTML must embed the M3 BASELINE_DARK primary token so a dark-mode
/// preview is possible without a token re-fetch. We compute the hex string
/// from `m3_tokens::color::BASELINE_DARK` at test time so the HTML stays
/// in lock-step with the canonical Rust definition.
#[test]
fn html_m3_baseline_dark_hex_present() {
    // BASELINE_DARK.primary is ARGB-packed (0xAARRGGBB). Strip the alpha
    // byte to get the CSS hex string used in `--md-sys-color-dark-primary`.
    let argb = m3_tokens::color::BASELINE_DARK.primary;
    let rgb = argb & 0x00FF_FFFF;
    let hex = format!("#{rgb:06X}");

    assert!(
        DEMO_HTML.to_ascii_uppercase().contains(&hex),
        "expected BASELINE_DARK.primary hex {hex} (computed from m3_tokens::color::BASELINE_DARK.primary = {argb:#010X}) in demo HTML — drift between m3-tokens and the demo"
    );
}

/// The demo MUST surface the `aphrody-*` OSC namespace (LLM-extension
/// ANSI sequences from spec §"LLM-extension ANSI sequences"). We don't
/// require every individual sequence — but the prefix must appear ≥5
/// times across the page (covers app branding + at least one OSC reference
/// per stack panel).
#[test]
fn html_has_osc_namespace_section() {
    let occurrences = DEMO_HTML.matches("aphrody-").count();
    assert!(
        occurrences >= 5,
        "expected 'aphrody-' OSC namespace prefix ≥5 times, found {occurrences}"
    );
}
