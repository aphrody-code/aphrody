// SPDX-License-Identifier: Apache-2.0
//! Fuzzy line-sequence search used to locate where an update chunk applies.
//!
//! Matches are attempted with decreasing strictness: exact, trailing-whitespace
//! insensitive, full-trim insensitive, then Unicode-normalised (so a diff
//! authored with ASCII `-` still matches source lines containing typographic
//! dashes, fancy quotes, or non-breaking spaces). When `eof` is set the search
//! starts at the tail of the haystack so end-of-file chunks anchor correctly.

/// Find `needle` within `haystack` at or after `start`.
///
/// Returns the starting index of the first match, or `None` when the needle
/// cannot be located (including when it is longer than the haystack). An empty
/// needle is a no-op match and returns `Some(start)`.
#[must_use]
pub fn seek_sequence(haystack: &[String], needle: &[String], start: usize) -> Option<usize> {
    seek_sequence_eof(haystack, needle, start, false)
}

/// Like [`seek_sequence`] but, when `eof` is true, prefers matches anchored at
/// the end of `haystack`.
#[must_use]
pub fn seek_sequence_eof(
    haystack: &[String],
    needle: &[String],
    start: usize,
    eof: bool,
) -> Option<usize> {
    if needle.is_empty() {
        return Some(start);
    }
    if needle.len() > haystack.len() {
        return None;
    }

    let search_start = if eof && haystack.len() >= needle.len() {
        haystack.len() - needle.len()
    } else {
        start
    };
    let last = haystack.len() - needle.len();
    if search_start > last {
        return None;
    }

    // Pass 1: exact match.
    for i in search_start..=last {
        if haystack[i..i + needle.len()] == *needle {
            return Some(i);
        }
    }
    // Pass 2: ignore trailing whitespace.
    if let Some(i) = scan(haystack, needle, search_start, last, |a, b| {
        a.trim_end() == b.trim_end()
    }) {
        return Some(i);
    }
    // Pass 3: ignore leading and trailing whitespace.
    if let Some(i) = scan(haystack, needle, search_start, last, |a, b| {
        a.trim() == b.trim()
    }) {
        return Some(i);
    }
    // Pass 4: Unicode-normalised comparison (dashes, quotes, exotic spaces).
    scan(haystack, needle, search_start, last, |a, b| {
        normalise(a) == normalise(b)
    })
}

/// Scan `haystack[search_start..=last]` for a window matching `needle` under
/// the supplied line equality predicate.
fn scan(
    haystack: &[String],
    needle: &[String],
    search_start: usize,
    last: usize,
    eq: impl Fn(&str, &str) -> bool,
) -> Option<usize> {
    for i in search_start..=last {
        if needle
            .iter()
            .enumerate()
            .all(|(j, pat)| eq(&haystack[i + j], pat))
        {
            return Some(i);
        }
    }
    None
}

/// Normalise common Unicode punctuation to ASCII equivalents and trim, so that
/// ASCII-authored diffs match typographically formatted source.
fn normalise(s: &str) -> String {
    s.trim()
        .chars()
        .map(|c| match c {
            // Dash / hyphen code points -> ASCII '-'. Includes EN DASH
            // (\u{2013}) and NON-BREAKING HYPHEN (\u{2011}).
            '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2013}' | '\u{2014}' | '\u{2015}'
            | '\u{2212}' => '-',
            // Fancy single quotes -> '\''.
            '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{201B}' => '\'',
            // Fancy double quotes -> '"'.
            '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{201F}' => '"',
            // Non-breaking and exotic spaces -> ' '.
            '\u{00A0}' | '\u{2002}' | '\u{2003}' | '\u{2004}' | '\u{2005}' | '\u{2006}'
            | '\u{2007}' | '\u{2008}' | '\u{2009}' | '\u{200A}' | '\u{202F}' | '\u{205F}'
            | '\u{3000}' => ' ',
            other => other,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::string::ToString;

    fn v(strings: &[&str]) -> Vec<String> {
        strings.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn exact_match() {
        assert_eq!(seek_sequence(&v(&["foo", "bar", "baz"]), &v(&["bar", "baz"]), 0), Some(1));
    }

    #[test]
    fn rstrip_match() {
        assert_eq!(seek_sequence(&v(&["foo   ", "bar\t\t"]), &v(&["foo", "bar"]), 0), Some(0));
    }

    #[test]
    fn trim_match() {
        assert_eq!(seek_sequence(&v(&["    foo   ", "   bar\t"]), &v(&["foo", "bar"]), 0), Some(0));
    }

    #[test]
    fn needle_longer_than_haystack() {
        assert_eq!(seek_sequence(&v(&["only"]), &v(&["a", "b", "c"]), 0), None);
    }

    #[test]
    fn empty_needle_is_noop() {
        assert_eq!(seek_sequence(&v(&["a", "b"]), &[], 1), Some(1));
    }

    #[test]
    fn unicode_dash_normalised() {
        // Source has EN DASH and NON-BREAKING HYPHEN; needle uses ASCII '-'.
        let haystack = v(&["a \u{2013} b", "c\u{2011}d"]);
        let needle = v(&["a - b", "c-d"]);
        assert_eq!(seek_sequence(&haystack, &needle, 0), Some(0));
    }

    #[test]
    fn eof_anchor_prefers_tail() {
        let haystack = v(&["x", "x", "x"]);
        let needle = v(&["x"]);
        assert_eq!(seek_sequence_eof(&haystack, &needle, 0, true), Some(2));
    }
}
