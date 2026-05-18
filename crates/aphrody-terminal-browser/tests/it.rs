// SPDX-License-Identifier: Apache-2.0
//! Integration tests for aphrody-terminal-browser.
//!
//! OSC parser tests are pure (no I/O) and run on every platform.
//! Backend tests that require external binaries are marked `#[ignore]` with
//! a precise reason so CI can gate them appropriately.

use aphrody_terminal_browser::{
    Active, BrowserRequest, RecordState, ScreenshotArea, osc::parse_aphrody_browser_osc,
};
use base64::{Engine as _, engine::general_purpose::STANDARD as B64};

// ---------------------------------------------------------------------------
// OSC parser tests — no external binaries required
// ---------------------------------------------------------------------------

/// `aphrody-browser-nav` with a plain URL (BEL-terminated).
#[test]
fn osc_parse_nav_minimal() {
    // Full framing: ESC ] ... BEL
    let seq = b"\x1b]aphrody-browser-nav;https://example.com\x07";
    let req = parse_aphrody_browser_osc(seq).expect("should parse nav OSC sequence");
    match req {
        BrowserRequest::Nav { url } => {
            assert_eq!(url, "https://example.com");
        },
        other => panic!("expected Nav, got {other:?}"),
    }
}

/// `aphrody-browser-eval` with a base64-encoded JS payload.
#[test]
fn osc_parse_eval_base64() {
    let js = "document.title";
    let b64 = B64.encode(js.as_bytes());
    let seq = format!("aphrody-browser-eval;{b64}");
    let req = parse_aphrody_browser_osc(seq.as_bytes()).expect("should parse eval OSC sequence");
    match req {
        BrowserRequest::Eval { src } => {
            assert_eq!(src, js, "decoded JS must match original");
        },
        other => panic!("expected Eval, got {other:?}"),
    }
}

/// `aphrody-browser-dom` with a base64-encoded CSS selector.
#[test]
fn osc_parse_dom_base64() {
    let selector = "#content > p.lead";
    let b64 = B64.encode(selector.as_bytes());
    let seq = format!("aphrody-browser-dom;{b64}");
    let req = parse_aphrody_browser_osc(seq.as_bytes()).expect("should parse dom OSC sequence");
    match req {
        BrowserRequest::Dom { selector: s } => {
            assert_eq!(s, selector);
        },
        other => panic!("expected Dom, got {other:?}"),
    }
}

/// `aphrody-browser-screenshot` with `element:<sel>` area variant.
#[test]
fn osc_parse_screenshot_element() {
    let seq = b"aphrody-browser-screenshot;element:#hero-banner";
    let req = parse_aphrody_browser_osc(seq).expect("should parse screenshot element OSC");
    match req {
        BrowserRequest::Screenshot { area: ScreenshotArea::Element { selector } } => {
            assert_eq!(selector, "#hero-banner");
        },
        other => panic!("expected Screenshot(Element), got {other:?}"),
    }
}

/// `aphrody-browser-screenshot` with `fullpage` area variant.
#[test]
fn osc_parse_screenshot_fullpage() {
    let seq = b"aphrody-browser-screenshot;fullpage";
    let req = parse_aphrody_browser_osc(seq).expect("should parse screenshot fullpage OSC");
    assert!(
        matches!(req, BrowserRequest::Screenshot { area: ScreenshotArea::Fullpage }),
        "expected Fullpage variant"
    );
}

/// `aphrody-browser-record` — both `start` and `stop` states.
#[test]
fn osc_parse_record_states() {
    // start
    let start_seq = b"aphrody-browser-record;session-42;start";
    let start = parse_aphrody_browser_osc(start_seq).expect("should parse record start");
    match start {
        BrowserRequest::Record { id, state } => {
            assert_eq!(id, "session-42");
            assert_eq!(state, RecordState::Start);
        },
        other => panic!("expected Record, got {other:?}"),
    }

    // stop
    let stop_seq = b"aphrody-browser-record;session-42;stop";
    let stop = parse_aphrody_browser_osc(stop_seq).expect("should parse record stop");
    match stop {
        BrowserRequest::Record { id, state } => {
            assert_eq!(id, "session-42");
            assert_eq!(state, RecordState::Stop);
        },
        other => panic!("expected Record, got {other:?}"),
    }
}

/// `aphrody-browser-intercept` with a base64-encoded JSON rule.
#[test]
fn osc_parse_intercept_base64_json() {
    let rule = r#"{"url_pattern":"*api.example.com*","action":"block"}"#;
    let b64 = B64.encode(rule.as_bytes());
    let seq = format!("aphrody-browser-intercept;{b64}");
    let req = parse_aphrody_browser_osc(seq.as_bytes()).expect("should parse intercept OSC");
    match req {
        BrowserRequest::Intercept { rule: r } => {
            assert_eq!(r["url_pattern"], "*api.example.com*");
            assert_eq!(r["action"], "block");
        },
        other => panic!("expected Intercept, got {other:?}"),
    }
}

/// `aphrody-browser-extract` with a base64-encoded JSON schema.
#[test]
fn osc_parse_extract_base64_json() {
    let schema = r#"{"type":"object","properties":{"price":{"type":"number"}}}"#;
    let b64 = B64.encode(schema.as_bytes());
    let seq = format!("aphrody-browser-extract;{b64}");
    let req = parse_aphrody_browser_osc(seq.as_bytes()).expect("should parse extract OSC");
    match req {
        BrowserRequest::Extract { schema: s } => {
            assert_eq!(s["type"], "object");
        },
        other => panic!("expected Extract, got {other:?}"),
    }
}

/// Unknown prefix must not match.
#[test]
fn osc_parse_unknown_prefix_returns_none() {
    let seq = b"aphrody-md;SGVsbG8=";
    assert!(parse_aphrody_browser_osc(seq).is_none(), "non-browser OSC must return None");
}

/// Unknown op (after `aphrody-browser-`) must return None.
#[test]
fn osc_parse_unknown_op_returns_none() {
    let seq = b"aphrody-browser-bogus;payload";
    assert!(parse_aphrody_browser_osc(seq).is_none(), "unknown op must return None");
}

/// ST-terminated OSC must parse identically to BEL-terminated.
#[test]
fn osc_parse_st_terminator() {
    // ST = ESC backslash
    let seq = b"\x1b]aphrody-browser-nav;https://st.example.com\x1b\\";
    let req = parse_aphrody_browser_osc(seq).expect("ST-terminated OSC must parse");
    assert!(matches!(req, BrowserRequest::Nav { url } if url == "https://st.example.com"));
}

// ---------------------------------------------------------------------------
// Active::probe smoke test — verifies that probe() either returns a working
// backend or an Unsupported error; it must NOT panic or hang.
// ---------------------------------------------------------------------------

/// Smoke test: `Active::probe()` must complete and return some backend or a
/// clean `Unsupported` error.  We do not assert which backend is selected
/// because that depends on what binaries are installed on the test machine.
#[tokio::test]
async fn active_probe_returns_some_backend_or_unsupported() {
    match Active::probe().await {
        Ok(active) => {
            // Verify the name is one of the three known backends.
            let name = active.name();
            assert!(
                matches!(name, "bxc" | "agent-browser" | "edge"),
                "unexpected backend name: {name}"
            );
        },
        Err(aphrody_terminal_browser::BrowserError::Unsupported { .. }) => {
            // Expected on machines with no browser binaries installed.
        },
        Err(e) => {
            panic!("probe() returned unexpected error variant: {e}");
        },
    }
}

// ---------------------------------------------------------------------------
// Edge backend integration test — Windows-only, requires msedge on PATH
// ---------------------------------------------------------------------------

/// Spawn `msedge --headless=new --dump-dom about:blank` and verify it exits
/// cleanly with non-empty output.
///
/// Skipped on non-Windows because msedge is not available there, and skipped
/// at runtime if msedge is not on PATH (even on Windows).
#[cfg_attr(not(target_os = "windows"), ignore = "msedge is only available on Windows")]
#[tokio::test]
async fn edge_backend_dump_dom_about_blank() {
    use aphrody_terminal_browser::backend::edge::msedge_available;

    if !msedge_available() {
        eprintln!("msedge not on PATH, skipping edge_backend_dump_dom_about_blank");
        return;
    }

    let html = aphrody_terminal_browser::backend::edge::dump_dom_raw("about:blank")
        .await
        .expect("msedge --dump-dom about:blank must succeed");

    assert!(!html.is_empty(), "msedge --dump-dom about:blank produced empty output");
    // about:blank renders a minimal HTML document.
    assert!(
        html.contains("<html") || html.contains("<!DOCTYPE"),
        "expected HTML document in msedge --dump-dom output, got: {html:.200}"
    );
}
