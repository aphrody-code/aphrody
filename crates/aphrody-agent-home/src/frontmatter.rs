// SPDX-License-Identifier: Apache-2.0
//! Tiny YAML-ish frontmatter parser shared by the typed docs (soul / identity
//! / user / tools).
//!
//! Deliberately not `serde_yaml` (the workspace dropped `serde_yml` over
//! RUSTSEC-2025-0068, and `serde_yaml` is archived). The persona schema only
//! needs `key: scalar`, inline flow lists `[a, b]`, and block lists
//! (`key:` then `  - item`). Anything richer belongs in the markdown body.

/// One frontmatter value: a scalar or a list of scalars.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FmValue {
    /// A single scalar string (already unquoted).
    Scalar(String),
    /// A list of scalar strings (already unquoted).
    List(Vec<String>),
}

/// Split a markdown document into its `---` frontmatter block and the body.
///
/// Returns `(Some(frontmatter), body)` when the document opens with a `---`
/// line and has a closing `---` line, else `(None, full_document)`. Mirrors
/// `workspace.ts::stripFrontMatter` but keeps the block so it can be typed.
/// A leading UTF-8 BOM is tolerated.
#[must_use]
pub fn split(input: &str) -> (Option<&str>, &str) {
    let trimmed = input.trim_start_matches('\u{feff}');
    if !trimmed.starts_with("---") {
        return (None, input);
    }
    let after_open = match trimmed.split_once('\n') {
        Some((first, rest)) if first.trim_end() == "---" => rest,
        _ => return (None, input),
    };
    let mut idx = 0usize;
    for line in after_open.split_inclusive('\n') {
        if line.trim_end() == "---" {
            let fm = &after_open[..idx];
            let body_start = idx + line.len();
            let body = after_open.get(body_start..).unwrap_or("").trim_start();
            return (Some(fm), body);
        }
        idx += line.len();
    }
    (None, input)
}

/// Parse a frontmatter block into ordered key -> value pairs. Tolerant:
/// comment lines (`#`) and malformed lines are skipped; block lists are folded
/// into the preceding key.
#[must_use]
pub fn parse(fm: &str) -> Vec<(String, FmValue)> {
    let mut out: Vec<(String, FmValue)> = Vec::new();
    let lines: Vec<&str> = fm.lines().collect();
    let mut i = 0usize;
    while i < lines.len() {
        let trimmed = lines[i].trim_end();
        if trimmed.trim().is_empty() || trimmed.trim_start().starts_with('#') {
            i += 1;
            continue;
        }
        let Some((raw_key, raw_val)) = trimmed.split_once(':') else {
            i += 1;
            continue;
        };
        let key = raw_key.trim().to_string();
        if key.is_empty() {
            i += 1;
            continue;
        }
        let val = raw_val.trim();
        if val.is_empty() {
            let mut items: Vec<String> = Vec::new();
            let mut j = i + 1;
            while j < lines.len() {
                let item = lines[j].trim_start();
                if let Some(rest) = item.strip_prefix("- ") {
                    items.push(unquote(rest.trim()));
                    j += 1;
                } else if item.is_empty() {
                    j += 1;
                } else {
                    break;
                }
            }
            if items.is_empty() {
                out.push((key, FmValue::Scalar(String::new())));
                i += 1;
            } else {
                out.push((key, FmValue::List(items)));
                i = j;
            }
        } else if let Some(inline) = parse_inline_list(val) {
            out.push((key, FmValue::List(inline)));
            i += 1;
        } else {
            out.push((key, FmValue::Scalar(unquote(val))));
            i += 1;
        }
    }
    out
}

/// Parse an inline flow list `[a, b, c]`; returns `None` when not bracketed.
fn parse_inline_list(val: &str) -> Option<Vec<String>> {
    let inner = val.strip_prefix('[')?.strip_suffix(']')?;
    if inner.trim().is_empty() {
        return Some(Vec::new());
    }
    Some(inner.split(',').map(|s| unquote(s.trim())).collect())
}

/// Strip a single matched pair of surrounding quotes.
#[must_use]
pub fn unquote(s: &str) -> String {
    let bytes = s.as_bytes();
    if bytes.len() >= 2
        && ((bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[bytes.len() - 1] == b'\''))
    {
        s[1..s.len() - 1].to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_returns_none_without_frontmatter() {
        let (fm, body) = split("no frontmatter here\n");
        assert!(fm.is_none());
        assert_eq!(body, "no frontmatter here\n");
    }

    #[test]
    fn split_extracts_block_and_trims_body() {
        let (fm, body) = split("---\nkey: val\n---\n\n  body line\n");
        assert_eq!(fm, Some("key: val\n"));
        assert_eq!(body, "body line\n");
    }

    #[test]
    fn split_tolerates_bom() {
        let (fm, _body) = split("\u{feff}---\nk: v\n---\nx");
        assert_eq!(fm, Some("k: v\n"));
    }

    #[test]
    fn parse_handles_scalar_block_and_inline() {
        let pairs = parse("a: 1\nb:\n  - x\n  - y\nc: [m, n]\n");
        assert_eq!(pairs[0], ("a".into(), FmValue::Scalar("1".into())));
        assert_eq!(pairs[1], ("b".into(), FmValue::List(vec!["x".into(), "y".into()])));
        assert_eq!(pairs[2], ("c".into(), FmValue::List(vec!["m".into(), "n".into()])));
    }

    #[test]
    fn parse_skips_comments_and_blanks() {
        let pairs = parse("# comment\n\nk: v\n");
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].0, "k");
    }

    #[test]
    fn unquote_strips_matched_quotes_only() {
        assert_eq!(unquote("\"hi\""), "hi");
        assert_eq!(unquote("'hi'"), "hi");
        assert_eq!(unquote("hi"), "hi");
        assert_eq!(unquote("\"hi"), "\"hi");
    }
}
