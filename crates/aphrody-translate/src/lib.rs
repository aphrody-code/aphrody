//! Aphrody Translate — bibliothèque de réécriture des commentaires source.
//!
//! Quatre étages :
//!   1. extraction  → trouve les commentaires par langage (regex purs Rust)
//!   2. ai_patterns → détecte/nettoie les marqueurs IA et émoji
//!   3. translate   → appelle MyMemory (gratuit) avec cache JSON persistant
//!   4. aphrodify   → applique le style Aphrody (sobre, impersonnel, focus code)

pub mod ai_patterns;
pub mod aphrodify;
pub mod extract;
pub mod translate;

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Go,
    C,
    Cpp,
    Shell,
    Toml,
    Markdown,
    Other,
}

impl Lang {
    pub fn from_path(p: &Path) -> Self {
        let ext = p.extension().and_then(|s| s.to_str()).unwrap_or("");
        match ext {
            "rs" => Lang::Rust,
            "ts" | "tsx" => Lang::TypeScript,
            "js" | "jsx" | "mjs" | "cjs" => Lang::JavaScript,
            "py" | "pyi" => Lang::Python,
            "go" => Lang::Go,
            "c" | "h" => Lang::C,
            "cc" | "cpp" | "cxx" | "hpp" | "hxx" => Lang::Cpp,
            "sh" | "bash" | "zsh" => Lang::Shell,
            "toml" => Lang::Toml,
            "md" | "markdown" => Lang::Markdown,
            _ => Lang::Other,
        }
    }

    pub fn supports_comments(&self) -> bool {
        !matches!(self, Lang::Other)
    }
}

#[derive(Debug, Clone)]
pub struct Comment {
    pub raw: String,
    pub body: String,
    pub start: usize,
    pub end: usize,
    pub style: CommentStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentStyle {
    LineSlashSlash,
    LineHash,
    BlockSlashStar,
    DocSlashSlashSlash,
    DocSlashStarStar,
    PythonTripleQuote,
    HtmlBlock,
}

impl CommentStyle {
    pub fn rewrap(&self, body: &str) -> String {
        match self {
            CommentStyle::LineSlashSlash => format!("// {body}"),
            CommentStyle::LineHash => format!("# {body}"),
            CommentStyle::DocSlashSlashSlash => format!("/// {body}"),
            CommentStyle::BlockSlashStar => format!("/* {body} */"),
            CommentStyle::DocSlashStarStar => format!("/** {body} */"),
            CommentStyle::PythonTripleQuote => format!("\"\"\"{body}\"\"\""),
            CommentStyle::HtmlBlock => format!("<!-- {body} -->"),
        }
    }
}
