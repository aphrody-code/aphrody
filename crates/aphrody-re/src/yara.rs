// SPDX-License-Identifier: Apache-2.0
//! YARA-X pattern-matching scanner (`aphrody re yara`).
//!
//! Wraps the [`yara_x`] crate (pure Rust, BSD-3-Clause) to provide a
//! zero-copy rule compiler + scanner surface that fits the `aphrody-re`
//! JSON-report convention.
//!
//! ## Usage
//!
//! ```no_run
//! use aphrody_re::yara::scan;
//!
//! let rules_src = r#"rule hello { strings: $a = "MZ" condition: $a }"#;
//! let data: &[u8] = b"MZ\x90\x00the rest of the binary";
//! let report = scan(rules_src, data).expect("scan must not fail on valid rules");
//! assert_eq!(report.matches.len(), 1);
//! assert_eq!(report.matches[0].rule_name, "hello");
//! ```
//!
//! ## Feature gate
//!
//! This module is only compiled when the crate is built with the `yara`
//! Cargo feature. The CLI arm `aphrody re yara` checks for the feature at
//! compile time and returns a human-readable rebuild instruction when the
//! feature is absent.

use serde::{Deserialize, Serialize};
use thiserror::Error;
use yara_x::Compiler;

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors produced by [`scan`] and helpers.
#[derive(Debug, Error)]
pub enum YaraError {
    /// The YARA rule source could not be compiled.
    ///
    /// The inner string is `yara_x`'s human-readable compile error with rule
    /// name + line/column information included.
    #[error("YARA rule compile error: {0}")]
    Compile(String),

    /// The scanner failed to scan the data slice.
    ///
    /// This is rare (yara-x only errors on scan if an internal invariant is
    /// violated); included for exhaustive error handling.
    #[error("YARA scan error: {0}")]
    Scan(String),
}

// ---------------------------------------------------------------------------
// Report DTOs
// ---------------------------------------------------------------------------

/// One string pattern occurrence within a matching rule.
///
/// Analogous to YARA's `match` concept: the identifier (`$a`, `$str0`) and
/// the byte offset within the scanned data where the pattern was found.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YaraStringMatch {
    /// The pattern identifier as declared in the YARA rule (e.g. `"$a"`,
    /// `"$magic"`).
    pub identifier: String,
    /// Byte offset within the scanned data where this pattern instance begins.
    pub offset: usize,
    /// Length in bytes of the matched bytes at `offset`.
    pub length: usize,
    /// Hex-encoded bytes at the match site (up to the first 64 bytes of the
    /// pattern match — avoids bloating the report for long patterns).
    pub bytes_hex: String,
}

/// One rule match returned by [`scan`].
///
/// Corresponds to a single YARA rule whose condition evaluated to `true`
/// against the scanned data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YaraRuleMatch {
    /// The rule name as declared in the YARA source (e.g. `"detect_upx"`).
    pub rule_name: String,
    /// The namespace the rule was declared in. Defaults to `"default"` when
    /// no explicit namespace is used in the source.
    pub namespace: String,
    /// Tags declared on the rule (e.g. `rule foo : tag1 tag2 { … }`).
    pub tags: Vec<String>,
    /// All pattern occurrences (strings) found by the matching rule.
    pub matched_strings: Vec<YaraStringMatch>,
}

/// Full YARA scan report returned by [`scan`].
///
/// Serialises to a compact JSON object suitable for consumption by `jq` or
/// downstream tools. The top-level `matches` array is empty when no rule
/// matched (not `null`), making null-free indexing safe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YaraReport {
    /// Total size (in bytes) of the scanned data.
    pub scanned_bytes: usize,
    /// Number of rules that matched.
    pub match_count: usize,
    /// All matching rules, in the order they were evaluated by the engine.
    pub matches: Vec<YaraRuleMatch>,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Compile YARA rules from source text and scan `data`.
///
/// This is the primary entry point for the YARA scanner. It is intentionally
/// synchronous — the caller is responsible for offloading to
/// `tokio::task::spawn_blocking` when calling from async context.
///
/// # Arguments
///
/// - `rules_src`: the YARA rule source as a UTF-8 string (may contain
///   multiple rules, optionally with explicit namespace declarations).
/// - `data`: the raw binary bytes to scan.
///
/// # Errors
///
/// Returns [`YaraError::Compile`] if the rules fail to compile, or
/// [`YaraError::Scan`] if the scanner returns an internal error (rare).
///
/// # Examples
///
/// ```
/// use aphrody_re::yara::scan;
///
/// // A rule that matches the DOS MZ magic header.
/// let rules = r#"rule mz_header {
///     strings:
///         $magic = { 4D 5A }
///     condition:
///         $magic at 0
/// }"#;
///
/// // Craft minimal MZ-like bytes.
/// let data: Vec<u8> = {
///     let mut v = b"MZ".to_vec();
///     v.extend_from_slice(&[0u8; 30]);
///     v
/// };
///
/// let report = scan(rules, &data).expect("compile + scan must succeed");
/// assert_eq!(report.match_count, 1);
/// assert_eq!(report.matches[0].rule_name, "mz_header");
/// ```
pub fn scan(rules_src: &str, data: &[u8]) -> Result<YaraReport, YaraError> {
    // --- Compile ---
    let mut compiler = Compiler::new();
    compiler
        .add_source(rules_src)
        .map_err(|e| YaraError::Compile(e.to_string()))?;
    let rules = compiler.build();

    // --- Scan ---
    let mut scanner = yara_x::Scanner::new(&rules);
    let results = scanner.scan(data).map_err(|e| YaraError::Scan(e.to_string()))?;

    // --- Collect matches ---
    let mut matches: Vec<YaraRuleMatch> = Vec::new();

    for matching_rule in results.matching_rules() {
        let rule_name = matching_rule.identifier().to_owned();
        let namespace = matching_rule.namespace().to_owned();
        let tags: Vec<String> = matching_rule.tags().map(|t| t.identifier().to_owned()).collect();

        let matched_strings: Vec<YaraStringMatch> = matching_rule
            .patterns()
            .flat_map(|pattern| {
                let identifier = pattern.identifier().to_owned();
                pattern.matches().map(move |m| {
                    let offset = m.range().start;
                    let length = m.range().len();
                    // Cap the hex preview at 64 bytes to keep reports concise.
                    let end = (offset + length).min(offset + 64);
                    let raw_slice = data.get(offset..end).unwrap_or(&[]);
                    YaraStringMatch {
                        identifier: identifier.clone(),
                        offset,
                        length,
                        bytes_hex: hex::encode(raw_slice),
                    }
                })
            })
            .collect();

        matches.push(YaraRuleMatch { rule_name, namespace, tags, matched_strings });
    }

    Ok(YaraReport {
        scanned_bytes: data.len(),
        match_count: matches.len(),
        matches,
    })
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Simplest possible match: rule triggers on the `"MZ"` string at any
    /// offset. Test data starts with `MZ` so the rule must match.
    #[test]
    fn scan_mz_rule_matches_mz_prefix() {
        let rules = r#"rule test_mz {
            strings:
                $a = "MZ"
            condition:
                $a
        }"#;

        // Minimal DOS/PE-like header — starts with MZ.
        let mut data = b"MZ".to_vec();
        data.extend_from_slice(&[0u8; 62]);

        let report = scan(rules, &data).expect("scan must succeed on valid rules + data");

        assert_eq!(report.scanned_bytes, data.len());
        assert_eq!(report.match_count, 1, "exactly one rule must match: {report:?}");
        let m = &report.matches[0];
        assert_eq!(m.rule_name, "test_mz");
        assert_eq!(m.namespace, "default");
        assert!(m.tags.is_empty(), "rule has no tags");
        assert_eq!(m.matched_strings.len(), 1, "exactly one pattern occurrence");
        assert_eq!(m.matched_strings[0].identifier, "$a");
        assert_eq!(m.matched_strings[0].offset, 0);
        assert_eq!(m.matched_strings[0].length, 2);
        assert_eq!(m.matched_strings[0].bytes_hex, "4d5a");
    }

    /// Non-matching data: the rule must NOT fire.
    #[test]
    fn scan_mz_rule_does_not_match_non_mz_data() {
        let rules = r#"rule test_mz {
            strings:
                $a = "MZ"
            condition:
                $a
        }"#;

        let data: &[u8] = b"\x7FELF\x00\x00\x00\x00this is an ELF binary prefix";

        let report = scan(rules, data).expect("scan must succeed");
        assert_eq!(report.match_count, 0, "ELF magic must not match MZ rule");
        assert!(report.matches.is_empty());
    }

    /// Multiple rules: only the matching one fires; the non-matching rule is
    /// silent. Verifies that multi-rule sources compile and evaluate correctly.
    #[test]
    fn scan_multiple_rules_only_matching_fires() {
        let rules = r#"
            rule detect_mz {
                strings:
                    $magic = "MZ"
                condition:
                    $magic
            }
            rule detect_elf {
                strings:
                    $magic = { 7F 45 4C 46 }
                condition:
                    $magic
            }
        "#;

        let mut mz_data = b"MZ".to_vec();
        mz_data.extend_from_slice(&[0u8; 62]);

        let report = scan(rules, &mz_data).expect("scan must succeed");
        assert_eq!(report.match_count, 1, "only detect_mz should match: {report:?}");
        assert_eq!(report.matches[0].rule_name, "detect_mz");
    }

    /// Invalid rule source must return `YaraError::Compile`.
    #[test]
    fn scan_invalid_rule_returns_compile_error() {
        let bad_rules = "rule oops { this is not valid yara syntax !!!}";
        let result = scan(bad_rules, b"anything");
        assert!(
            matches!(result, Err(YaraError::Compile(_))),
            "expected YaraError::Compile, got: {result:?}"
        );
    }

    /// The report JSON shape must be stable: `scanned_bytes`, `match_count`,
    /// `matches` always present as number / number / array.
    #[test]
    fn scan_report_serializes_to_stable_json_shape() {
        let rules = r#"rule test_json {
            strings: $a = "MZ"
            condition: $a
        }"#;

        let mut data = b"MZ".to_vec();
        data.extend_from_slice(&[0u8; 30]);

        let report = scan(rules, &data).expect("scan ok");
        let json = serde_json::to_value(&report).expect("serialize");

        assert!(json["scanned_bytes"].is_number());
        assert!(json["match_count"].is_number());
        assert!(json["matches"].is_array(), "matches must always be an array");
        let first = &json["matches"][0];
        assert!(first["rule_name"].is_string());
        assert!(first["namespace"].is_string());
        assert!(first["tags"].is_array());
        assert!(first["matched_strings"].is_array());
    }
}
