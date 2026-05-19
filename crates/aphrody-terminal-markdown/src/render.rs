// SPDX-License-Identifier: Apache-2.0
//! Themed CommonMark → ANSI renderer.
//!
//! Complements the bare-bones [`crate::MarkdownRenderer`] (which emits only
//! SGR weight / underline / reverse-video) by layering a [`crate::MdTheme`]
//! on top: headings get the theme's `fg_heading`, links get `fg_link`,
//! emphasis gets `fg_emphasis`, code blocks get a `bg_code_block` swatch
//! when no syntect language match is found.
//!
//! The themed renderer reuses the unthemed walker for everything else
//! (lists, blockquotes, thematic breaks, paragraphs) by appending output to
//! a shared `String` buffer.

use comrak::{
    Arena, Options,
    nodes::{AstNode, NodeValue},
    parse_document,
};

use crate::{
    code::render_code_block_ansi,
    heading::heading_ansi_prefix,
    theme::{MdTheme, bg_sgr, fg_sgr},
};

const SGR_RESET: &str = "\x1b[0m";
const SGR_BOLD: &str = "\x1b[1m";
const SGR_ITALIC: &str = "\x1b[3m";
const SGR_UNDERLINE: &str = "\x1b[4m";
const SGR_REVERSE: &str = "\x1b[7m";

/// Render CommonMark `input` to themed ANSI text.
///
/// The output always ends with `SGR_RESET` to prevent style bleed.
#[must_use]
pub fn render(input: &str, theme: &MdTheme) -> String {
    let arena = Arena::new();
    let opts = Options::default();
    let root = parse_document(&arena, input, &opts);

    let mut out = String::with_capacity(input.len() + 64);
    for child in root.children() {
        render_block(child, theme, &mut out);
    }
    if !out.ends_with(SGR_RESET) {
        out.push_str(SGR_RESET);
    }
    out
}

fn render_block<'a>(node: &'a AstNode<'a>, theme: &MdTheme, out: &mut String) {
    let value = &node.data.borrow().value;
    match value {
        NodeValue::Heading(h) => {
            let prefix = heading_ansi_prefix(h.level);
            out.push_str(prefix);
            out.push_str(&fg_sgr(theme.fg_heading));
            render_inlines(node, theme, out);
            out.push_str(SGR_RESET);
            out.push('\n');
        },
        NodeValue::Paragraph => {
            render_inlines(node, theme, out);
            out.push('\n');
        },
        NodeValue::CodeBlock(cb) => {
            let lang = cb.info.split_whitespace().next().unwrap_or("");
            // For known languages, defer to syntect (its truecolor SGR
            // already encodes the colour). For unknown ones, paint the
            // body with the theme's code colours so the block still pops.
            let highlighted = render_code_block_ansi(&cb.literal, lang, crate::DEFAULT_THEME);
            if highlighted == cb.literal {
                // Verbatim passthrough -> apply theme.
                out.push_str(&bg_sgr(theme.bg_code_block));
                out.push_str(&fg_sgr(theme.fg_code));
                out.push_str(&cb.literal);
                out.push_str(SGR_RESET);
                if !cb.literal.ends_with('\n') {
                    out.push('\n');
                }
            } else {
                out.push_str(&highlighted);
                if !highlighted.ends_with('\n') {
                    out.push('\n');
                }
            }
        },
        NodeValue::ThematicBreak => {
            out.push_str("---\n");
        },
        NodeValue::BlockQuote => {
            let mut inner = String::new();
            for child in node.children() {
                render_block(child, theme, &mut inner);
            }
            for line in inner.lines() {
                out.push_str("> ");
                out.push_str(line);
                out.push('\n');
            }
        },
        NodeValue::List(_) => {
            for child in node.children() {
                render_block(child, theme, out);
            }
        },
        NodeValue::Item(_) => {
            out.push_str("\u{2022} ");
            let mut inner = String::new();
            for child in node.children() {
                render_block(child, theme, &mut inner);
            }
            out.push_str(inner.trim_end_matches('\n'));
            out.push('\n');
        },
        _ => {
            render_inlines(node, theme, out);
            out.push('\n');
        },
    }
}

fn render_inlines<'a>(node: &'a AstNode<'a>, theme: &MdTheme, out: &mut String) {
    for child in node.children() {
        let v = &child.data.borrow().value;
        match v {
            NodeValue::Text(t) => out.push_str(t),
            NodeValue::SoftBreak => out.push(' '),
            NodeValue::LineBreak => out.push('\n'),
            NodeValue::Strong => {
                out.push_str(SGR_BOLD);
                out.push_str(&fg_sgr(theme.fg_emphasis));
                render_inlines(child, theme, out);
                out.push_str(SGR_RESET);
            },
            NodeValue::Emph => {
                out.push_str(SGR_ITALIC);
                out.push_str(&fg_sgr(theme.fg_emphasis));
                render_inlines(child, theme, out);
                out.push_str(SGR_RESET);
            },
            NodeValue::Strikethrough => {
                out.push_str("\x1b[9m");
                render_inlines(child, theme, out);
                out.push_str(SGR_RESET);
            },
            NodeValue::Underline => {
                out.push_str(SGR_UNDERLINE);
                render_inlines(child, theme, out);
                out.push_str(SGR_RESET);
            },
            NodeValue::Code(c) => {
                out.push_str(SGR_REVERSE);
                out.push_str(&fg_sgr(theme.fg_code));
                out.push(' ');
                out.push_str(&c.literal);
                out.push(' ');
                out.push_str(SGR_RESET);
            },
            NodeValue::Link(l) => {
                out.push_str(SGR_UNDERLINE);
                out.push_str(&fg_sgr(theme.fg_link));
                render_inlines(child, theme, out);
                out.push_str(SGR_RESET);
                if !l.url.is_empty() {
                    out.push_str(" (");
                    out.push_str(&l.url);
                    out.push(')');
                }
            },
            NodeValue::Image(l) => {
                out.push_str("[image]");
                if !l.url.is_empty() {
                    out.push('(');
                    out.push_str(&l.url);
                    out.push(')');
                }
            },
            NodeValue::HtmlInline(s) => out.push_str(s),
            _ => render_inlines(child, theme, out),
        }
    }
}
