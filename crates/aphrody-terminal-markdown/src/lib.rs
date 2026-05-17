// SPDX-License-Identifier: Apache-2.0
//! Inline CommonMark renderer for `aphrody-terminal`.
//!
//! Surface:
//! - [`MarkdownRenderer`] — parses CommonMark via `comrak`, walks the AST,
//!   and emits ANSI text (SGR bold/italic/underline for inline styling,
//!   24-bit truecolor for fenced code via `syntect`).
//! - [`OscMdDetector`] — recognises the `aphrody-md;<base64>` OSC payload
//!   emitted by `aphrody-terminal-llm` (`crates/aphrody-terminal-llm/src/osc.rs`)
//!   and returns the decoded markdown source.
//!
//! The renderer is deterministic and pure (no I/O, no globals beyond a
//! `OnceCell`-cached `SyntaxSet`/`ThemeSet`). All public types are `Send + Sync`.

#![forbid(unsafe_code)]

mod code;
mod heading;

use base64::{Engine as _, engine::general_purpose::STANDARD as B64};
use comrak::nodes::{AstNode, NodeValue};
use comrak::{Arena, Options, parse_document};

pub use code::{DEFAULT_THEME, render_code_block_ansi};
pub use heading::heading_ansi_prefix;

/// ANSI SGR helpers.
const SGR_RESET: &str = "\x1b[0m";
const SGR_BOLD: &str = "\x1b[1m";
const SGR_ITALIC: &str = "\x1b[3m";
const SGR_UNDERLINE: &str = "\x1b[4m";
const SGR_REVERSE: &str = "\x1b[7m";

/// Inline CommonMark -> ANSI renderer.
///
/// Use [`MarkdownRenderer::new`] for the default `base16-ocean.dark` theme,
/// or [`MarkdownRenderer::with_theme`] to pick another bundled syntect theme.
#[derive(Clone)]
pub struct MarkdownRenderer {
    theme: String,
}

impl std::fmt::Debug for MarkdownRenderer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MarkdownRenderer")
            .field("theme", &self.theme)
            .finish()
    }
}

impl Default for MarkdownRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl MarkdownRenderer {
    /// Construct with the default syntect theme ([`DEFAULT_THEME`]).
    #[must_use]
    pub fn new() -> Self {
        Self {
            theme: DEFAULT_THEME.to_owned(),
        }
    }

    /// Construct with an explicit bundled syntect theme name
    /// (e.g. `"base16-eighties.dark"`, `"InspiredGitHub"`,
    /// `"Solarized (light)"`). Unknown themes fall back silently to
    /// [`DEFAULT_THEME`] at render time.
    #[must_use]
    pub fn with_theme(theme: &str) -> Self {
        Self {
            theme: theme.to_owned(),
        }
    }

    /// Render a CommonMark source to an ANSI-coloured `String`.
    ///
    /// Block boundaries are separated by a single `\n`. The output always
    /// ends with `SGR_RESET` to avoid leaking styling into surrounding text.
    #[must_use]
    pub fn render_commonmark(&self, src: &str) -> String {
        let arena = Arena::new();
        let opts = Options::default();
        let root = parse_document(&arena, src, &opts);

        let mut out = String::with_capacity(src.len() + 32);
        for child in root.children() {
            self.render_block(child, &mut out);
        }
        if !out.ends_with(SGR_RESET) {
            out.push_str(SGR_RESET);
        }
        out
    }

    /// Render an isolated fenced code block.
    ///
    /// `lang` is the info-string (e.g. `"rust"`, `"py"`, `""`).
    /// If empty/unknown, the body is returned verbatim (no highlighting).
    #[must_use]
    pub fn render_code_block(&self, code: &str, lang: &str) -> String {
        render_code_block_ansi(code, lang, &self.theme)
    }

    fn render_block<'a>(&self, node: &'a AstNode<'a>, out: &mut String) {
        let value = &node.data.borrow().value;
        match value {
            NodeValue::Heading(h) => {
                let level = h.level;
                out.push_str(heading_ansi_prefix(level));
                self.render_inlines(node, out);
                out.push_str(SGR_RESET);
                out.push('\n');
            }
            NodeValue::Paragraph => {
                self.render_inlines(node, out);
                out.push('\n');
            }
            NodeValue::CodeBlock(cb) => {
                let lang = cb.info.split_whitespace().next().unwrap_or("");
                let rendered = self.render_code_block(&cb.literal, lang);
                out.push_str(&rendered);
                if !rendered.ends_with('\n') {
                    out.push('\n');
                }
            }
            NodeValue::ThematicBreak => {
                out.push_str("---\n");
            }
            NodeValue::BlockQuote => {
                let mut inner = String::new();
                for child in node.children() {
                    self.render_block(child, &mut inner);
                }
                for line in inner.lines() {
                    out.push_str("> ");
                    out.push_str(line);
                    out.push('\n');
                }
            }
            NodeValue::List(_) => {
                for child in node.children() {
                    self.render_block(child, out);
                }
            }
            NodeValue::Item(_) => {
                out.push_str("- ");
                let mut inner = String::new();
                for child in node.children() {
                    self.render_block(child, &mut inner);
                }
                // Trim the trailing newline emitted by the inner block so the
                // bullet line stays compact.
                out.push_str(inner.trim_end_matches('\n'));
                out.push('\n');
            }
            _ => {
                // Fallback: render whatever inline text is reachable so we
                // never silently swallow content.
                self.render_inlines(node, out);
                out.push('\n');
            }
        }
    }

    fn render_inlines<'a>(&self, node: &'a AstNode<'a>, out: &mut String) {
        for child in node.children() {
            let v = &child.data.borrow().value;
            match v {
                NodeValue::Text(t) => out.push_str(t),
                NodeValue::SoftBreak => out.push(' '),
                NodeValue::LineBreak => out.push('\n'),
                NodeValue::Strong => {
                    out.push_str(SGR_BOLD);
                    self.render_inlines(child, out);
                    out.push_str(SGR_RESET);
                }
                NodeValue::Emph => {
                    out.push_str(SGR_ITALIC);
                    self.render_inlines(child, out);
                    out.push_str(SGR_RESET);
                }
                NodeValue::Strikethrough => {
                    // SGR 9 = crossed-out.
                    out.push_str("\x1b[9m");
                    self.render_inlines(child, out);
                    out.push_str(SGR_RESET);
                }
                NodeValue::Underline => {
                    out.push_str(SGR_UNDERLINE);
                    self.render_inlines(child, out);
                    out.push_str(SGR_RESET);
                }
                NodeValue::Code(c) => {
                    // Inline code: reverse video so it visually pops in
                    // every terminal palette without relying on truecolor.
                    out.push_str(SGR_REVERSE);
                    out.push(' ');
                    out.push_str(&c.literal);
                    out.push(' ');
                    out.push_str(SGR_RESET);
                }
                NodeValue::Link(l) => {
                    out.push_str(SGR_UNDERLINE);
                    self.render_inlines(child, out);
                    out.push_str(SGR_RESET);
                    if !l.url.is_empty() {
                        out.push_str(" (");
                        out.push_str(&l.url);
                        out.push(')');
                    }
                }
                NodeValue::Image(l) => {
                    out.push_str("[image]");
                    if !l.url.is_empty() {
                        out.push('(');
                        out.push_str(&l.url);
                        out.push(')');
                    }
                }
                NodeValue::HtmlInline(s) => out.push_str(s),
                _ => self.render_inlines(child, out),
            }
        }
    }
}

/// Detector for the `aphrody-md;<base64>` OSC payload.
///
/// The detector is stateless aside from being a value type; [`feed`] returns
/// the decoded markdown source on the first valid call and `None` otherwise.
/// Both the bare body (`"aphrody-md;<b64>"`) and the full OSC framing
/// (`"\x1b]aphrody-md;<b64>\x07"` / ST terminator) are accepted, so callers
/// may forward either the raw VT slice or the already-stripped OSC payload
/// emitted by `aphrody-terminal-vt`.
///
/// [`feed`]: OscMdDetector::feed
#[derive(Debug, Default, Clone, Copy)]
pub struct OscMdDetector;

impl OscMdDetector {
    /// Construct a fresh detector.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Try to decode an `aphrody-md` OSC payload. Returns `Some(markdown)`
    /// on success, `None` if the prefix is wrong or the base64 / UTF-8 fails.
    #[must_use]
    pub fn feed(&mut self, osc_payload: &str) -> Option<String> {
        let body = strip_osc_framing(osc_payload);
        let after = body.strip_prefix("aphrody-md;")?;
        let bytes = B64.decode(after.as_bytes()).ok()?;
        String::from_utf8(bytes).ok()
    }
}

/// Strip an optional `ESC ]` prefix and `BEL` / `ESC \` suffix so the caller
/// can pass either the framed sequence or just the payload body.
fn strip_osc_framing(s: &str) -> &str {
    let mut out = s;
    if let Some(rest) = out.strip_prefix("\x1b]") {
        out = rest;
    }
    if let Some(rest) = out.strip_suffix("\x07") {
        out = rest;
    } else if let Some(rest) = out.strip_suffix("\x1b\\") {
        out = rest;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detector_handles_framed_and_bare() {
        let mut d = OscMdDetector::new();
        // bare body
        assert_eq!(d.feed("aphrody-md;SGVsbG8=").as_deref(), Some("Hello"));
        // framed with BEL
        assert_eq!(
            d.feed("\x1b]aphrody-md;SGVsbG8=\x07").as_deref(),
            Some("Hello")
        );
        // framed with ST
        assert_eq!(
            d.feed("\x1b]aphrody-md;SGVsbG8=\x1b\\").as_deref(),
            Some("Hello")
        );
    }

    #[test]
    fn detector_rejects_wrong_prefix_and_bad_b64() {
        let mut d = OscMdDetector::new();
        assert!(d.feed("other;SGVsbG8=").is_none());
        assert!(d.feed("aphrody-md;!!!").is_none());
    }
}
