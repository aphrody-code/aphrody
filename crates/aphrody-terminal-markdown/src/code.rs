// SPDX-License-Identifier: Apache-2.0
//! Fenced code-block highlighter backed by `syntect`.
//!
//! `SyntaxSet` and `ThemeSet` are loaded once per process via [`once_cell`]
//! to amortise the ~5-10ms bundled-default load cost across many blocks.
//! Output uses 24-bit truecolor SGR (`ESC [ 38;2;R;G;B m`), which every
//! modern terminal handles (xterm, iTerm2, Windows Terminal, vscode,
//! Alacritty, etc.).
//!
//! Unknown languages fall back to verbatim passthrough — no styling — so
//! the body is never mangled.

use once_cell::sync::Lazy;
use syntect::{
    easy::HighlightLines,
    highlighting::{Style, Theme, ThemeSet},
    parsing::SyntaxSet,
    util::{LinesWithEndings, as_24_bit_terminal_escaped},
};

/// Default theme name. `base16-ocean.dark` ships with syntect's
/// `default-themes` and is the same default `bat` uses.
pub const DEFAULT_THEME: &str = "base16-ocean.dark";

static SYNTAXES: Lazy<SyntaxSet> = Lazy::new(SyntaxSet::load_defaults_newlines);
static THEMES: Lazy<ThemeSet> = Lazy::new(ThemeSet::load_defaults);

/// Render a single fenced code block as ANSI-coloured text.
///
/// * `code` — raw block body (may end with a trailing newline).
/// * `lang` — info-string (`"rust"`, `"py"`, `"json"`, ...). Empty or unknown languages return
///   `code` verbatim.
/// * `theme_name` — bundled syntect theme; falls back to [`DEFAULT_THEME`] if not found.
#[must_use]
pub fn render_code_block_ansi(code: &str, lang: &str, theme_name: &str) -> String {
    if lang.is_empty() {
        return code.to_owned();
    }
    let syntax = match SYNTAXES
        .find_syntax_by_token(lang)
        .or_else(|| SYNTAXES.find_syntax_by_extension(lang))
        .or_else(|| SYNTAXES.find_syntax_by_name(lang))
    {
        Some(s) => s,
        None => return code.to_owned(),
    };

    let theme: &Theme = THEMES
        .themes
        .get(theme_name)
        .or_else(|| THEMES.themes.get(DEFAULT_THEME))
        .expect("syntect bundled themes must include the default theme");

    let mut highlighter = HighlightLines::new(syntax, theme);
    let mut out = String::with_capacity(code.len() * 2);
    for line in LinesWithEndings::from(code) {
        // On a parse error, fall back to the raw line to avoid losing data.
        let ranges: Vec<(Style, &str)> = highlighter
            .highlight_line(line, &SYNTAXES)
            .unwrap_or_else(|_| vec![(Style::default(), line)]);
        out.push_str(&as_24_bit_terminal_escaped(&ranges[..], false));
    }
    out.push_str("\x1b[0m");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_lang_passthrough() {
        let src = "hello world\n";
        assert_eq!(render_code_block_ansi(src, "", DEFAULT_THEME), src);
        assert_eq!(render_code_block_ansi(src, "no-such-lang-xyz", DEFAULT_THEME), src);
    }

    #[test]
    fn rust_emits_truecolor_sgr() {
        let out = render_code_block_ansi("fn main() {}\n", "rust", DEFAULT_THEME);
        // 24-bit foreground SGR introducer.
        assert!(out.contains("\x1b[38;2;"), "expected truecolor SGR: {out:?}");
    }
}
