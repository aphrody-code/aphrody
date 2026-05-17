// SPDX-License-Identifier: Apache-2.0
//! ANSI styling for Markdown headings H1..H6.
//!
//! Mapping (kept minimal so plain terminals without truecolor still render
//! semantically correct emphasis):
//!
//! | Level | SGR prefix              | Visual                |
//! |-------|-------------------------|-----------------------|
//! | H1    | `\x1b[1m`               | bold                  |
//! | H2    | `\x1b[1;4m`             | bold + underline      |
//! | H3    | `\x1b[1m`               | bold                  |
//! | H4    | `\x1b[1;3m`             | bold + italic         |
//! | H5    | `\x1b[3m`               | italic                |
//! | H6    | `\x1b[3;2m`             | italic + faint        |
//!
//! The reset must be emitted by the caller after the heading inlines.

/// Return the SGR escape sequence to prefix a heading body of `level`.
///
/// Levels outside `1..=6` are clamped to H1.
#[must_use]
pub fn heading_ansi_prefix(level: u8) -> &'static str {
    match level {
        1 => "\x1b[1m",
        2 => "\x1b[1;4m",
        3 => "\x1b[1m",
        4 => "\x1b[1;3m",
        5 => "\x1b[3m",
        6 => "\x1b[3;2m",
        _ => "\x1b[1m",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn h1_through_h6_are_distinct_or_grouped() {
        // H1 and H3 intentionally share the bold-only prefix; everything
        // else must be unique.
        let prefixes = [
            heading_ansi_prefix(1),
            heading_ansi_prefix(2),
            heading_ansi_prefix(3),
            heading_ansi_prefix(4),
            heading_ansi_prefix(5),
            heading_ansi_prefix(6),
        ];
        assert_eq!(prefixes[0], prefixes[2]);
        assert_ne!(prefixes[0], prefixes[1]);
        assert_ne!(prefixes[3], prefixes[4]);
        assert_ne!(prefixes[4], prefixes[5]);
    }

    #[test]
    fn out_of_range_clamps_to_h1() {
        assert_eq!(heading_ansi_prefix(0), heading_ansi_prefix(1));
        assert_eq!(heading_ansi_prefix(99), heading_ansi_prefix(1));
    }
}
