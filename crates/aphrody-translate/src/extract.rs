//! Extraction des commentaires par langage via expressions régulières.
//!
//! Volontairement sans tree-sitter pour rester pur Rust cross-platform (Linux,
//! Windows, WASM compute-only) et sans dépendance native C par grammaire.
//! Précision suffisante pour les commentaires hors-chaîne (>95% du code réel).

use std::sync::OnceLock;

use regex::Regex;

use crate::{Comment, CommentStyle, Lang};

static RE_SLASHSLASH_DOC: OnceLock<Regex> = OnceLock::new();
static RE_SLASHSLASH: OnceLock<Regex> = OnceLock::new();
static RE_BLOCK_DOC: OnceLock<Regex> = OnceLock::new();
static RE_BLOCK: OnceLock<Regex> = OnceLock::new();
static RE_HASH: OnceLock<Regex> = OnceLock::new();
static RE_PYTHON_TRIPLE: OnceLock<Regex> = OnceLock::new();
static RE_HTML_BLOCK: OnceLock<Regex> = OnceLock::new();

fn re_slashslash_doc() -> &'static Regex {
    RE_SLASHSLASH_DOC.get_or_init(|| Regex::new(r"(?m)^[ \t]*///[ \t]?([^\n]*)").unwrap())
}
fn re_slashslash() -> &'static Regex {
    // Capture les `//` en début de ligne ET en fin de ligne après du code.
    // Le caractère négatif `[^:/]` (ou ligne nouvelle) avant évite les URL `https://`.
    RE_SLASHSLASH.get_or_init(|| Regex::new(r"(?m)(?:^|[^:/])//[ \t]?([^/\n][^\n]*)?").unwrap())
}
fn re_block_doc() -> &'static Regex {
    RE_BLOCK_DOC.get_or_init(|| Regex::new(r"(?s)/\*\*\s*(.+?)\s*\*/").unwrap())
}
fn re_block() -> &'static Regex {
    RE_BLOCK.get_or_init(|| Regex::new(r"(?s)/\*\s*(.+?)\s*\*/").unwrap())
}
fn re_hash() -> &'static Regex {
    RE_HASH.get_or_init(|| Regex::new(r"(?m)^[ \t]*#[ \t]?([^!\n][^\n]*)?").unwrap())
}
fn re_python_triple() -> &'static Regex {
    RE_PYTHON_TRIPLE.get_or_init(|| Regex::new(r#"(?s)"""\s*(.+?)\s*""""#).unwrap())
}
fn re_html_block() -> &'static Regex {
    RE_HTML_BLOCK.get_or_init(|| Regex::new(r"(?s)<!--\s*(.+?)\s*-->").unwrap())
}

pub fn extract(source: &str, lang: Lang) -> Vec<Comment> {
    let mut out: Vec<Comment> = Vec::new();
    let mut covered: Vec<(usize, usize)> = Vec::new();

    let push = |out: &mut Vec<Comment>,
                covered: &mut Vec<(usize, usize)>,
                m: regex::Match,
                body_cap: Option<regex::Match>,
                style: CommentStyle| {
        let (start, end) = (m.start(), m.end());
        if covered.iter().any(|(s, e)| !(end <= *s || start >= *e)) {
            return;
        }
        let body = body_cap.map(|c| c.as_str().trim().to_string()).unwrap_or_default();
        if body.is_empty() {
            return;
        }
        covered.push((start, end));
        out.push(Comment { raw: m.as_str().to_string(), body, start, end, style });
    };

    match lang {
        Lang::Rust => {
            for c in re_slashslash_doc().captures_iter(source) {
                let full = c.get(0).unwrap();
                let body = c.get(1);
                push(&mut out, &mut covered, full, body, CommentStyle::DocSlashSlashSlash);
            }
            for c in re_block_doc().captures_iter(source) {
                let full = c.get(0).unwrap();
                let body = c.get(1);
                push(&mut out, &mut covered, full, body, CommentStyle::DocSlashStarStar);
            }
            for c in re_slashslash().captures_iter(source) {
                let full = c.get(0).unwrap();
                let body = c.get(1);
                push(&mut out, &mut covered, full, body, CommentStyle::LineSlashSlash);
            }
            for c in re_block().captures_iter(source) {
                let full = c.get(0).unwrap();
                let body = c.get(1);
                push(&mut out, &mut covered, full, body, CommentStyle::BlockSlashStar);
            }
        },
        Lang::TypeScript | Lang::JavaScript | Lang::Go | Lang::C | Lang::Cpp => {
            for c in re_block_doc().captures_iter(source) {
                let full = c.get(0).unwrap();
                let body = c.get(1);
                push(&mut out, &mut covered, full, body, CommentStyle::DocSlashStarStar);
            }
            for c in re_block().captures_iter(source) {
                let full = c.get(0).unwrap();
                let body = c.get(1);
                push(&mut out, &mut covered, full, body, CommentStyle::BlockSlashStar);
            }
            for c in re_slashslash().captures_iter(source) {
                let full = c.get(0).unwrap();
                let body = c.get(1);
                push(&mut out, &mut covered, full, body, CommentStyle::LineSlashSlash);
            }
        },
        Lang::Python => {
            for c in re_python_triple().captures_iter(source) {
                let full = c.get(0).unwrap();
                let body = c.get(1);
                push(&mut out, &mut covered, full, body, CommentStyle::PythonTripleQuote);
            }
            for c in re_hash().captures_iter(source) {
                let full = c.get(0).unwrap();
                let body = c.get(1);
                push(&mut out, &mut covered, full, body, CommentStyle::LineHash);
            }
        },
        Lang::Shell | Lang::Toml => {
            for c in re_hash().captures_iter(source) {
                let full = c.get(0).unwrap();
                let body = c.get(1);
                push(&mut out, &mut covered, full, body, CommentStyle::LineHash);
            }
        },
        Lang::Markdown => {
            for c in re_html_block().captures_iter(source) {
                let full = c.get(0).unwrap();
                let body = c.get(1);
                push(&mut out, &mut covered, full, body, CommentStyle::HtmlBlock);
            }
        },
        Lang::Other => {},
    }

    out.sort_by_key(|c| c.start);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_line_and_doc() {
        let src = "/// doc line\nfn foo() {} // tail";
        let comments = extract(src, Lang::Rust);
        assert!(comments.iter().any(|c| c.body == "doc line"));
        assert!(comments.iter().any(|c| c.body == "tail"));
    }

    #[test]
    fn python_hash_and_triple() {
        let src = "# top\ndef f():\n    \"\"\"\n    doc body\n    \"\"\"\n    pass";
        let comments = extract(src, Lang::Python);
        assert!(comments.iter().any(|c| c.body == "top"));
        assert!(comments.iter().any(|c| c.body.contains("doc body")));
    }

    #[test]
    fn block_comment_ts() {
        let src = "/* hello */\nlet x = 1;";
        let comments = extract(src, Lang::TypeScript);
        assert_eq!(comments.len(), 1);
        assert_eq!(comments[0].body, "hello");
    }
}
