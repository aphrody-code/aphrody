// SPDX-License-Identifier: Apache-2.0
//! End-to-end tests for `aphrody-terminal-markdown`.
//!
//! Test names match the verify-table in the dispatch spec:
//! - render_commonmark_h1_to_h3
//! - render_commonmark_bold_italic
//! - render_commonmark_code_inline
//! - code_block_highlight_rust
//! - code_block_highlight_no_lang
//! - osc_aphrody_md_emit
//! - osc_aphrody_md_invalid_base64
//! - osc_aphrody_md_wrong_prefix

use aphrody_terminal_markdown::{MarkdownRenderer, OscMdDetector};

#[test]
fn render_commonmark_h1_to_h3() {
    let r = MarkdownRenderer::new();
    let h1 = r.render_commonmark("# foo\n");
    let h2 = r.render_commonmark("## bar\n");
    let h3 = r.render_commonmark("### baz\n");

    // H1 -> bold (SGR 1).
    assert!(h1.contains("\x1b[1m"), "h1 missing bold SGR: {h1:?}");
    assert!(h1.contains("foo"));

    // H2 -> bold + underline (SGR 1;4).
    assert!(
        h2.contains("\x1b[1;4m"),
        "h2 missing bold+underline SGR: {h2:?}"
    );
    assert!(h2.contains("bar"));

    // H3 -> bold (SGR 1).
    assert!(h3.contains("\x1b[1m"), "h3 missing bold SGR: {h3:?}");
    assert!(h3.contains("baz"));

    // Every rendering must end with a reset to avoid leaking style.
    assert!(h1.ends_with("\x1b[0m"));
    assert!(h2.ends_with("\x1b[0m"));
    assert!(h3.ends_with("\x1b[0m"));
}

#[test]
fn render_commonmark_bold_italic() {
    let r = MarkdownRenderer::new();
    let bold = r.render_commonmark("**bold**\n");
    let italic = r.render_commonmark("*italic*\n");

    // SGR 1 = bold.
    assert!(bold.contains("\x1b[1m"), "bold missing SGR 1: {bold:?}");
    assert!(bold.contains("bold"));

    // SGR 3 = italic.
    assert!(
        italic.contains("\x1b[3m"),
        "italic missing SGR 3: {italic:?}"
    );
    assert!(italic.contains("italic"));
}

#[test]
fn render_commonmark_code_inline() {
    let r = MarkdownRenderer::new();
    let out = r.render_commonmark("`code`\n");
    // SGR 7 = reverse video (used for inline code; works on any palette).
    assert!(out.contains("\x1b[7m"), "inline code missing reverse: {out:?}");
    assert!(out.contains("code"));
}

#[test]
fn code_block_highlight_rust() {
    let r = MarkdownRenderer::new();
    let src = "```rust\nfn main() {}\n```\n";
    let out = r.render_commonmark(src);
    // 24-bit truecolor SGR introducer for the `fn` keyword (or any other
    // highlighted token).
    assert!(
        out.contains("\x1b[38;2;"),
        "rust code block missing truecolor SGR: {out:?}"
    );
    assert!(out.contains("main"));
}

#[test]
fn code_block_highlight_no_lang() {
    let r = MarkdownRenderer::new();
    let src = "```\nplain body line 1\nplain body line 2\n```\n";
    let out = r.render_commonmark(src);
    // No language -> no truecolor SGR should be emitted by the highlighter.
    assert!(
        !out.contains("\x1b[38;2;"),
        "unlanged block leaked truecolor SGR: {out:?}"
    );
    assert!(out.contains("plain body line 1"));
    assert!(out.contains("plain body line 2"));
}

#[test]
fn osc_aphrody_md_emit() {
    let mut d = OscMdDetector::new();
    // base64("Hello") == "SGVsbG8="
    assert_eq!(d.feed("aphrody-md;SGVsbG8=").as_deref(), Some("Hello"));
}

#[test]
fn osc_aphrody_md_invalid_base64() {
    let mut d = OscMdDetector::new();
    assert_eq!(d.feed("aphrody-md;!!!"), None);
}

#[test]
fn osc_aphrody_md_wrong_prefix() {
    let mut d = OscMdDetector::new();
    assert_eq!(d.feed("other;SGVsbG8="), None);
}
