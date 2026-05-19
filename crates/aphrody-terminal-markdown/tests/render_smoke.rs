// SPDX-License-Identifier: Apache-2.0
//! Smoke tests for the themed [`render`] function, the OSC `aphrody-md-*`
//! writer, and the markdown→ANSI pipeline.
//!
//! Test names match the dispatch verify table:
//! - render_commonmark_basic_paragraph
//! - render_heading_emits_color
//! - code_block_highlight_rust
//! - code_block_unknown_lang_falls_back
//! - osc_aphrody_md_emit_format
//! - render_link_underlines

use aphrody_terminal_markdown::{MdEvent, MdTheme, emit_md_event, fg_sgr, render};

#[test]
fn render_commonmark_basic_paragraph() {
    let theme = MdTheme::from_m3_dark();
    let out = render("Hello **world**", &theme);
    // SGR 1 = bold opener wrapping the strong run.
    assert!(out.contains("\x1b[1m"), "missing bold SGR: {out:?}");
    // Reset must close the document.
    assert!(out.ends_with("\x1b[0m"), "missing trailing reset: {out:?}");
    // Body text preserved verbatim.
    assert!(out.contains("world"), "missing 'world' substring: {out:?}");
}

#[test]
fn render_heading_emits_color() {
    let theme = MdTheme::from_m3_dark();
    let out = render("# Title", &theme);
    let expected_color = fg_sgr(theme.fg_heading);
    assert!(
        out.contains(&expected_color),
        "heading missing 24-bit SGR {expected_color:?} in output: {out:?}"
    );
    assert!(out.contains("Title"), "heading missing text: {out:?}");
}

#[test]
fn code_block_highlight_rust() {
    let theme = MdTheme::from_m3_dark();
    let src = "```rust\nfn main() {}\n```\n";
    let out = render(src, &theme);
    // syntect emits 24-bit foreground SGR for every highlighted token.
    assert!(
        out.contains("\x1b[38;2;"),
        "rust block missing truecolor SGR: {out:?}"
    );
    // The body must still be present.
    assert!(out.contains("main"), "rust block dropped 'main' token: {out:?}");
}

#[test]
fn code_block_unknown_lang_falls_back() {
    let theme = MdTheme::from_m3_dark();
    let src = "```foobarlang\ncontent\n```\n";
    let out = render(src, &theme);
    // Renderer must not panic and must preserve the body.
    assert!(out.contains("content"), "unknown-lang block lost body: {out:?}");
    // Reset envelope closes the document.
    assert!(out.ends_with("\x1b[0m"), "missing trailing reset: {out:?}");
}

#[test]
fn osc_aphrody_md_emit_format() {
    let mut buf: Vec<u8> = Vec::new();
    emit_md_event(
        &mut buf,
        MdEvent::Rendered { bytes_in: 100, bytes_out: 250, fence_count: 1 },
    )
    .expect("write to Vec never fails");
    let s = String::from_utf8(buf).expect("emitter produces UTF-8");
    assert!(
        s.contains("\x1b]aphrody-md-rendered;"),
        "missing aphrody-md-rendered op prefix: {s:?}"
    );
    assert!(s.contains('\x07'), "missing BEL terminator: {s:?}");
}

#[test]
fn render_link_underlines() {
    let theme = MdTheme::from_m3_dark();
    let out = render("[click](https://example.com)", &theme);
    // SGR 4 = underline.
    assert!(out.contains("\x1b[4m"), "link missing underline SGR: {out:?}");
    // Anchor text preserved.
    assert!(out.contains("click"), "link dropped anchor text: {out:?}");
}
