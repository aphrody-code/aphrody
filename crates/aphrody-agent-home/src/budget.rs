// SPDX-License-Identifier: Apache-2.0
//! Bootstrap budget + streaming truncation (AH-6).
//!
//! Faithful port of `var/openclaw/src/agents/bootstrap-budget.ts`, pushed past
//! it with a single-pass [`BudgetWriter`] that truncates on grapheme-cluster
//! boundaries (so a multi-byte glyph is never split) without intermediate
//! allocations. The analysis ([`BudgetAnalysis`]), signature
//! ([`TruncationReport::signature`]) and warning dedup (off / once / always)
//! reproduce the upstream outputs so existing fixtures continue to match.
//!
//! Defaults mirror openclaw exactly:
//! * per-file cap `12_000` chars
//! * total cap `60_000` chars
//! * near-limit ratio `0.85`
//! * `AGENTS.md` carries a special truncation warning line.

use serde::{Deserialize, Serialize};
use unicode_segmentation::UnicodeSegmentation;

/// openclaw `DEFAULT_BOOTSTRAP_NEAR_LIMIT_RATIO`.
pub const DEFAULT_NEAR_LIMIT_RATIO: f32 = 0.85;
/// openclaw default per-file char cap.
pub const DEFAULT_MAX_CHARS: usize = 12_000;
/// openclaw default total char cap.
pub const DEFAULT_TOTAL_MAX_CHARS: usize = 60_000;
/// openclaw `DEFAULT_BOOTSTRAP_PROMPT_WARNING_MAX_FILES`.
pub const DEFAULT_WARNING_MAX_FILES: usize = 3;
/// openclaw `DEFAULT_BOOTSTRAP_PROMPT_WARNING_SIGNATURE_HISTORY_MAX`.
pub const DEFAULT_SIGNATURE_HISTORY_MAX: usize = 32;

/// Why a file was truncated (openclaw `BootstrapTruncationCause`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TruncationCause {
    /// Hit the per-file limit.
    PerFileLimit,
    /// Hit the total budget limit.
    TotalLimit,
}

impl TruncationCause {
    /// Short warning label (openclaw `formatWarningCause`).
    #[must_use]
    pub const fn warning_label(self) -> &'static str {
        match self {
            TruncationCause::PerFileLimit => "max/file",
            TruncationCause::TotalLimit => "max/total",
        }
    }
}

/// Dedup mode for the truncation warning (openclaw `BootstrapPromptWarningMode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum WarningMode {
    /// Never show the warning.
    Off,
    /// Show once per distinct signature (the default).
    #[default]
    Once,
    /// Always show.
    Always,
}

/// Budget knobs (openclaw `BootstrapBudget`).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BootstrapBudget {
    /// Per-file char cap.
    pub max_chars: usize,
    /// Total char cap across all injected files.
    pub total_max_chars: usize,
    /// Near-limit ratio in `(0, 1)`.
    pub near_limit_ratio: f32,
}

impl Default for BootstrapBudget {
    fn default() -> Self {
        Self {
            max_chars: DEFAULT_MAX_CHARS,
            total_max_chars: DEFAULT_TOTAL_MAX_CHARS,
            near_limit_ratio: DEFAULT_NEAR_LIMIT_RATIO,
        }
    }
}

impl BootstrapBudget {
    /// Normalise a limit to a positive integer (openclaw `normalizePositiveLimit`).
    fn norm_limit(value: usize) -> usize {
        value.max(1)
    }

    /// Effective near-limit ratio (openclaw clamps to `(0,1)`, else default).
    fn norm_ratio(&self) -> f32 {
        if self.near_limit_ratio.is_finite()
            && self.near_limit_ratio > 0.0
            && self.near_limit_ratio < 1.0
        {
            self.near_limit_ratio
        } else {
            DEFAULT_NEAR_LIMIT_RATIO
        }
    }
}

/// Single-pass, allocation-light truncating writer.
///
/// Appends grapheme clusters until the per-call char budget is exhausted, then
/// stops. "Chars" are counted as Unicode scalar values (`char`) to match the
/// openclaw semantics of `.length` on already-normalised content while never
/// splitting a grapheme cluster mid-way.
#[derive(Debug)]
pub struct BudgetWriter {
    out: String,
    /// Remaining chars before the per-file cap.
    remaining: usize,
    /// Chars consumed so far.
    consumed: usize,
    /// Whether the last write was truncated.
    truncated: bool,
}

impl BudgetWriter {
    /// New writer bounded to `max_chars` scalar values.
    #[must_use]
    pub fn new(max_chars: usize) -> Self {
        Self {
            out: String::new(),
            remaining: max_chars,
            consumed: 0,
            truncated: false,
        }
    }

    /// Append as much of `text` as fits, on grapheme-cluster boundaries.
    /// Returns the number of scalar values actually written.
    pub fn write_truncated(&mut self, text: &str) -> usize {
        let mut written = 0usize;
        for grapheme in text.graphemes(true) {
            let g_chars = grapheme.chars().count();
            if g_chars > self.remaining {
                // The whole cluster does not fit -> stop on the boundary.
                self.truncated = true;
                break;
            }
            self.out.push_str(grapheme);
            self.remaining -= g_chars;
            self.consumed += g_chars;
            written += g_chars;
        }
        written
    }

    /// Consume the writer, yielding the accumulated string.
    #[must_use]
    pub fn into_string(self) -> String {
        self.out
    }

    /// Borrow the accumulated string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.out
    }

    /// Chars consumed so far.
    #[must_use]
    pub fn consumed(&self) -> usize {
        self.consumed
    }

    /// True if any append was truncated.
    #[must_use]
    pub fn truncated(&self) -> bool {
        self.truncated
    }
}

/// Per-file analysis (openclaw `BootstrapAnalyzedFile`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnalyzedFile {
    /// Bootstrap file name (e.g. `SOUL.md`).
    pub name: String,
    /// On-disk path string (or the name when no path).
    pub path: String,
    /// Whether the file was missing (no content).
    pub missing: bool,
    /// Raw char count (trailing whitespace trimmed, like openclaw `trimEnd`).
    pub raw_chars: usize,
    /// Char count actually injected after truncation.
    pub injected_chars: usize,
    /// Whether the file was truncated.
    pub truncated: bool,
    /// Whether the file is near the per-file limit.
    pub near_limit: bool,
    /// Truncation causes (sorted, deterministic).
    pub causes: Vec<TruncationCause>,
}

/// Whole-budget analysis (openclaw `BootstrapBudgetAnalysis`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BudgetAnalysis {
    /// Per-file results in input order.
    pub files: Vec<AnalyzedFile>,
    /// Sum of raw chars (non-missing).
    pub raw_chars: usize,
    /// Sum of injected chars (non-missing).
    pub injected_chars: usize,
    /// Total chars removed.
    pub truncated_chars: usize,
    /// Effective per-file cap.
    pub max_chars: usize,
    /// Effective total cap.
    pub total_max_chars: usize,
    /// Effective near-limit ratio.
    pub near_limit_ratio: f32,
    /// Whether the total is near its limit.
    pub total_near_limit: bool,
    /// Whether any file was truncated.
    pub has_truncation: bool,
}

impl BudgetAnalysis {
    /// Files that were truncated.
    #[must_use]
    pub fn truncated_files(&self) -> Vec<&AnalyzedFile> {
        self.files.iter().filter(|f| f.truncated).collect()
    }

    /// Files near their per-file limit.
    #[must_use]
    pub fn near_limit_files(&self) -> Vec<&AnalyzedFile> {
        self.files.iter().filter(|f| f.near_limit).collect()
    }
}

/// A file's raw content as presented to the budget.
#[derive(Debug, Clone)]
pub struct BudgetInput {
    /// File name (e.g. `SOUL.md`).
    pub name: String,
    /// On-disk path (or name).
    pub path: String,
    /// Content, or `None` when the file is missing.
    pub content: Option<String>,
}

fn is_agents_name(name: &str) -> bool {
    name.eq_ignore_ascii_case("agents.md")
}

/// Apply the budget to `inputs`, producing the injected text per file plus a
/// full [`BudgetAnalysis`]. Single pass over the inputs; each file is
/// truncated by a [`BudgetWriter`] bounded by the smaller of the remaining
/// per-file and total budgets.
#[must_use]
pub fn apply(budget: &BootstrapBudget, inputs: &[BudgetInput]) -> (Vec<(String, String)>, BudgetAnalysis) {
    let max_chars = BootstrapBudget::norm_limit(budget.max_chars);
    let total_max = BootstrapBudget::norm_limit(budget.total_max_chars);
    let ratio = budget.norm_ratio();

    let mut injected: Vec<(String, String)> = Vec::with_capacity(inputs.len());
    let mut files: Vec<AnalyzedFile> = Vec::with_capacity(inputs.len());
    let mut total_consumed = 0usize;
    let mut sum_raw = 0usize;

    for input in inputs {
        let Some(content) = &input.content else {
            files.push(AnalyzedFile {
                name: input.name.clone(),
                path: input.path.clone(),
                missing: true,
                raw_chars: 0,
                injected_chars: 0,
                truncated: false,
                near_limit: false,
                causes: Vec::new(),
            });
            injected.push((input.name.clone(), String::new()));
            continue;
        };

        let trimmed = content.trim_end();
        let raw_chars = trimmed.chars().count();
        sum_raw += raw_chars;

        let total_remaining = total_max.saturating_sub(total_consumed);
        let per_call = max_chars.min(total_remaining);
        let mut writer = BudgetWriter::new(per_call);
        writer.write_truncated(trimmed);
        let injected_chars = writer.consumed();
        total_consumed += injected_chars;

        let truncated = injected_chars < raw_chars;
        let per_file_over = raw_chars > max_chars;
        // openclaw: total over-limit is computed against the *final* injected
        // total, but per-file cause attribution uses the running over-limit
        // signal. We mirror its semantics: a file is total-limited when the
        // injected stream was cut short by the total budget (i.e. per_call was
        // the binding constraint and it came from the total budget).
        let total_over = truncated && per_call == total_remaining && per_call < max_chars;

        let mut causes: Vec<TruncationCause> = Vec::new();
        if truncated {
            if per_file_over {
                causes.push(TruncationCause::PerFileLimit);
            }
            if total_over {
                causes.push(TruncationCause::TotalLimit);
            }
            // Guarantee at least one cause is attributed when truncated.
            if causes.is_empty() {
                causes.push(TruncationCause::TotalLimit);
            }
        }
        causes.sort();
        causes.dedup();

        let near_limit = raw_chars >= ceil_mul(max_chars, ratio);

        files.push(AnalyzedFile {
            name: input.name.clone(),
            path: if input.path.is_empty() {
                input.name.clone()
            } else {
                input.path.clone()
            },
            missing: false,
            raw_chars,
            injected_chars,
            truncated,
            near_limit,
            causes,
        });
        injected.push((input.name.clone(), writer.into_string()));
    }

    let total_near_limit = total_consumed >= ceil_mul(total_max, ratio);
    let has_truncation = files.iter().any(|f| f.truncated);

    let analysis = BudgetAnalysis {
        files,
        raw_chars: sum_raw,
        injected_chars: total_consumed,
        truncated_chars: sum_raw.saturating_sub(total_consumed),
        max_chars,
        total_max_chars: total_max,
        near_limit_ratio: ratio,
        total_near_limit,
        has_truncation,
    };
    (injected, analysis)
}

/// `ceil(value * ratio)` in integer space (openclaw `Math.ceil(x * r)`).
///
/// openclaw computes this with an f64 ratio literal (`0.85`), so for the
/// canonical caps the product lands on an exact integer. aphrody stores the
/// ratio as `f32` (per the public API), and widening `f32 -> f64` introduces a
/// tiny representation error (e.g. `0.85f32` widens to `0.850000023…`) that
/// would spuriously bump `ceil(100 * 0.85)` from 85 to 86. We absorb that by
/// snapping the product to the nearest integer when it is within a small
/// epsilon, preserving parity with the upstream f64 result.
///
/// The casts are bounded: `value` is a char budget (well under 2^52) and the
/// result is non-negative, so the precision/sign lints are not reachable here.
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn ceil_mul(value: usize, ratio: f32) -> usize {
    let product = (value as f64) * f64::from(ratio);
    let rounded = product.round();
    // The f32->f64 widening error is bounded by ~value * f32::EPSILON; use a
    // generous multiple of that as the snap tolerance.
    let tolerance = (value as f64).mul_add(f64::from(f32::EPSILON), f64::EPSILON) * 4.0;
    if (product - rounded).abs() <= tolerance {
        return rounded as usize;
    }
    product.ceil() as usize
}

/// Rounded percentage of `raw` chars removed, given `injected` survived.
/// Bounded to `[0, 100]`; the casts are over small char counts (< 2^52).
#[allow(clippy::cast_precision_loss, clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn percent_removed(raw: usize, injected: usize) -> u64 {
    if raw == 0 || injected >= raw {
        return 0;
    }
    let pct = ((raw - injected) as f64 / raw as f64) * 100.0;
    pct.round() as u64
}

/// Truncation report attached to a [`crate::SystemPromptView`] (openclaw
/// `BootstrapTruncationReportMeta` + signature + lines).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TruncationReport {
    /// Stable signature of the truncation set (openclaw
    /// `buildBootstrapTruncationSignature`).
    pub signature: String,
    /// Whether a warning should be shown given the dedup mode + history.
    pub warning_shown: bool,
    /// Human-readable warning lines (empty when `!warning_shown`).
    pub lines: Vec<String>,
    /// Count of truncated files.
    pub truncated_files: usize,
    /// Count of near-limit files.
    pub near_limit_files: usize,
    /// Whether the total budget is near its limit.
    pub total_near_limit: bool,
    /// Signature history after recording this report (for once-mode dedup).
    pub signatures_seen: Vec<String>,
}

impl TruncationReport {
    /// Build a report from an analysis + dedup state. Returns `None` when
    /// there is no truncation (openclaw returns `undefined` signature).
    #[must_use]
    pub fn build(
        analysis: &BudgetAnalysis,
        mode: WarningMode,
        seen_signatures: &[String],
        max_files: usize,
    ) -> Option<Self> {
        if !analysis.has_truncation {
            return None;
        }
        let signature = build_signature(analysis);
        let mut seen = normalize_seen(seen_signatures);
        let already_seen = seen.contains(&signature);
        let warning_shown = match mode {
            WarningMode::Off => false,
            WarningMode::Always => true,
            WarningMode::Once => !already_seen,
        };
        if mode != WarningMode::Off {
            append_seen(&mut seen, &signature);
        }
        let lines = if warning_shown {
            format_warning_lines(analysis, max_files)
        } else {
            Vec::new()
        };
        Some(Self {
            signature,
            warning_shown,
            lines,
            truncated_files: analysis.truncated_files().len(),
            near_limit_files: analysis.near_limit_files().len(),
            total_near_limit: analysis.total_near_limit,
            signatures_seen: seen,
        })
    }
}

/// Build the stable truncation signature (openclaw
/// `buildBootstrapTruncationSignature`): JSON of the sorted truncated files +
/// the caps.
fn build_signature(analysis: &BudgetAnalysis) -> String {
    #[derive(Serialize)]
    struct SigFile<'a> {
        path: &'a str,
        #[serde(rename = "rawChars")]
        raw_chars: usize,
        #[serde(rename = "injectedChars")]
        injected_chars: usize,
        causes: Vec<&'static str>,
    }
    #[derive(Serialize)]
    struct Sig<'a> {
        #[serde(rename = "bootstrapMaxChars")]
        bootstrap_max_chars: usize,
        #[serde(rename = "bootstrapTotalMaxChars")]
        bootstrap_total_max_chars: usize,
        files: Vec<SigFile<'a>>,
    }

    let mut sig_files: Vec<SigFile> = analysis
        .truncated_files()
        .into_iter()
        .map(|f| {
            let mut causes: Vec<&'static str> =
                f.causes.iter().map(|c| serde_cause(*c)).collect();
            causes.sort_unstable();
            SigFile {
                path: if f.path.is_empty() { &f.name } else { &f.path },
                raw_chars: f.raw_chars,
                injected_chars: f.injected_chars,
                causes,
            }
        })
        .collect();
    sig_files.sort_by(|a, b| {
        a.path
            .cmp(b.path)
            .then(a.raw_chars.cmp(&b.raw_chars))
            .then(a.injected_chars.cmp(&b.injected_chars))
            .then(a.causes.join("+").cmp(&b.causes.join("+")))
    });

    let sig = Sig {
        bootstrap_max_chars: analysis.max_chars,
        bootstrap_total_max_chars: analysis.total_max_chars,
        files: sig_files,
    };
    serde_json::to_string(&sig).unwrap_or_default()
}

/// kebab-case cause name as serialised (matches openclaw cause strings).
fn serde_cause(c: TruncationCause) -> &'static str {
    match c {
        TruncationCause::PerFileLimit => "per-file-limit",
        TruncationCause::TotalLimit => "total-limit",
    }
}

/// Warning lines (openclaw `formatBootstrapTruncationWarningLines`).
fn format_warning_lines(analysis: &BudgetAnalysis, max_files: usize) -> Vec<String> {
    let max_files = if max_files == 0 {
        DEFAULT_WARNING_MAX_FILES
    } else {
        max_files
    };
    let truncated = analysis.truncated_files();
    let mut name_counts: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    for f in &truncated {
        *name_counts.entry(f.name.as_str()).or_insert(0) += 1;
    }
    let mut lines: Vec<String> = Vec::new();
    let top = truncated.iter().take(max_files);
    for f in top {
        let pct = percent_removed(f.raw_chars, f.injected_chars);
        let cause_text = if f.causes.is_empty() {
            String::new()
        } else {
            f.causes
                .iter()
                .map(|c| c.warning_label())
                .collect::<Vec<_>>()
                .join(", ")
        };
        let name_label = if name_counts.get(f.name.as_str()).copied().unwrap_or(0) > 1
            && !f.path.trim().is_empty()
        {
            format!("{} ({})", f.name, f.path)
        } else {
            f.name.clone()
        };
        let cause_suffix = if cause_text.is_empty() {
            String::new()
        } else {
            format!("; {cause_text}")
        };
        lines.push(format!(
            "{name_label}: {} raw -> {} injected (~{pct}% removed{cause_suffix}).",
            f.raw_chars, f.injected_chars,
        ));
    }
    if truncated.len() > max_files {
        lines.push(format!(
            "+{} more truncated file(s).",
            truncated.len() - max_files
        ));
    }
    if truncated.iter().any(|f| is_agents_name(&f.name)) {
        lines.push(
            "AGENTS.md was truncated; read the full AGENTS.md before relying on scoped policy."
                .to_string(),
        );
    }
    lines.push(
        "If unintentional, raise the bootstrap per-file and/or total char budget.".to_string(),
    );
    lines
}

fn normalize_seen(signatures: &[String]) -> Vec<String> {
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Vec::new();
    for s in signatures {
        let v = s.trim();
        if v.is_empty() || !seen.insert(v.to_string()) {
            continue;
        }
        out.push(v.to_string());
    }
    out
}

fn append_seen(signatures: &mut Vec<String>, signature: &str) {
    if signature.trim().is_empty() || signatures.iter().any(|s| s == signature) {
        return;
    }
    signatures.push(signature.to_string());
    if signatures.len() > DEFAULT_SIGNATURE_HISTORY_MAX {
        let overflow = signatures.len() - DEFAULT_SIGNATURE_HISTORY_MAX;
        signatures.drain(0..overflow);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(name: &str, content: &str) -> BudgetInput {
        BudgetInput {
            name: name.to_string(),
            path: name.to_string(),
            content: Some(content.to_string()),
        }
    }

    #[test]
    fn defaults_match_openclaw() {
        let b = BootstrapBudget::default();
        assert_eq!(b.max_chars, 12_000);
        assert_eq!(b.total_max_chars, 60_000);
        assert!((b.near_limit_ratio - 0.85).abs() < f32::EPSILON);
    }

    #[test]
    fn writer_truncates_on_grapheme_boundary() {
        // "a" + family-emoji ZWJ sequence (one grapheme, several scalar values).
        let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}"; // man+zwj+woman+zwj+girl
        let text = format!("a{family}b");
        // Budget of 1 scalar: only "a" fits; the emoji cluster (>1 scalar) is
        // dropped whole, never split.
        let mut w = BudgetWriter::new(1);
        w.write_truncated(&text);
        assert_eq!(w.as_str(), "a");
        assert!(w.truncated());
    }

    #[test]
    fn writer_keeps_full_grapheme_when_it_fits() {
        let family = "\u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}";
        let mut w = BudgetWriter::new(10);
        w.write_truncated(family);
        assert_eq!(w.as_str(), family);
        assert!(!w.truncated());
    }

    #[test]
    fn per_file_limit_truncates_and_attributes_cause() {
        let budget = BootstrapBudget {
            max_chars: 5,
            total_max_chars: 1000,
            near_limit_ratio: 0.85,
        };
        let (injected, analysis) = apply(&budget, &[input("SOUL.md", "0123456789")]);
        assert_eq!(injected[0].1, "01234");
        let f = &analysis.files[0];
        assert!(f.truncated);
        assert_eq!(f.injected_chars, 5);
        assert!(f.causes.contains(&TruncationCause::PerFileLimit));
    }

    #[test]
    fn total_limit_truncates_second_file() {
        let budget = BootstrapBudget {
            max_chars: 1000,
            total_max_chars: 8,
            near_limit_ratio: 0.85,
        };
        let (injected, analysis) =
            apply(&budget, &[input("A.md", "12345"), input("B.md", "67890")]);
        assert_eq!(injected[0].1, "12345");
        // Only 3 chars of total budget remain for B.
        assert_eq!(injected[1].1, "678");
        assert!(analysis.files[1].truncated);
        assert!(analysis.files[1].causes.contains(&TruncationCause::TotalLimit));
        assert_eq!(analysis.injected_chars, 8);
    }

    #[test]
    fn missing_file_contributes_nothing() {
        let budget = BootstrapBudget::default();
        let missing = BudgetInput {
            name: "USER.md".into(),
            path: "USER.md".into(),
            content: None,
        };
        let (injected, analysis) = apply(&budget, &[missing]);
        assert_eq!(injected[0].1, "");
        assert!(analysis.files[0].missing);
        assert_eq!(analysis.raw_chars, 0);
        assert!(!analysis.has_truncation);
    }

    #[test]
    fn trailing_whitespace_is_trimmed_for_raw_count() {
        let budget = BootstrapBudget::default();
        let (_inj, analysis) = apply(&budget, &[input("X.md", "abc\n\n  \n")]);
        assert_eq!(analysis.files[0].raw_chars, 3);
    }

    #[test]
    fn near_limit_flag_trips_at_ratio() {
        let budget = BootstrapBudget {
            max_chars: 100,
            total_max_chars: 1000,
            near_limit_ratio: 0.85,
        };
        // 85 chars == ceil(100*0.85) -> near limit.
        let content = "x".repeat(85);
        let (_i, a) = apply(&budget, &[input("X.md", &content)]);
        assert!(a.files[0].near_limit);
        // 84 chars -> not near.
        let content = "x".repeat(84);
        let (_i, a) = apply(&budget, &[input("X.md", &content)]);
        assert!(!a.files[0].near_limit);
    }

    #[test]
    fn signature_is_stable_and_order_independent() {
        let budget = BootstrapBudget {
            max_chars: 3,
            total_max_chars: 1000,
            near_limit_ratio: 0.85,
        };
        let (_i1, a1) = apply(&budget, &[input("A.md", "aaaaaa"), input("B.md", "bbbbbb")]);
        let (_i2, a2) = apply(&budget, &[input("B.md", "bbbbbb"), input("A.md", "aaaaaa")]);
        let s1 = build_signature(&a1);
        let s2 = build_signature(&a2);
        assert_eq!(s1, s2, "signature must be order-independent");
        assert!(s1.contains("bootstrapMaxChars"));
    }

    #[test]
    fn warning_once_mode_dedups() {
        let budget = BootstrapBudget {
            max_chars: 3,
            total_max_chars: 1000,
            near_limit_ratio: 0.85,
        };
        let (_i, analysis) = apply(&budget, &[input("SOUL.md", "abcdef")]);
        let r1 = TruncationReport::build(&analysis, WarningMode::Once, &[], 3).unwrap();
        assert!(r1.warning_shown);
        // Re-run with the seen history -> suppressed.
        let r2 =
            TruncationReport::build(&analysis, WarningMode::Once, &r1.signatures_seen, 3).unwrap();
        assert!(!r2.warning_shown);
        assert!(r2.lines.is_empty());
    }

    #[test]
    fn warning_always_mode_repeats() {
        let budget = BootstrapBudget {
            max_chars: 3,
            total_max_chars: 1000,
            near_limit_ratio: 0.85,
        };
        let (_i, analysis) = apply(&budget, &[input("SOUL.md", "abcdef")]);
        let r1 = TruncationReport::build(&analysis, WarningMode::Always, &[], 3).unwrap();
        let r2 = TruncationReport::build(
            &analysis,
            WarningMode::Always,
            &r1.signatures_seen,
            3,
        )
        .unwrap();
        assert!(r1.warning_shown && r2.warning_shown);
    }

    #[test]
    fn agents_md_truncation_adds_special_line() {
        let budget = BootstrapBudget {
            max_chars: 3,
            total_max_chars: 1000,
            near_limit_ratio: 0.85,
        };
        let (_i, analysis) = apply(&budget, &[input("AGENTS.md", "abcdef")]);
        let report = TruncationReport::build(&analysis, WarningMode::Once, &[], 3).unwrap();
        assert!(report.lines.iter().any(|l| l.contains("read the full AGENTS.md")));
    }

    #[test]
    fn no_truncation_yields_no_report() {
        let budget = BootstrapBudget::default();
        let (_i, analysis) = apply(&budget, &[input("X.md", "short")]);
        assert!(TruncationReport::build(&analysis, WarningMode::Once, &[], 3).is_none());
    }

    #[test]
    fn off_mode_never_shows_and_does_not_seed_history() {
        let budget = BootstrapBudget {
            max_chars: 3,
            total_max_chars: 1000,
            near_limit_ratio: 0.85,
        };
        let (_i, analysis) = apply(&budget, &[input("SOUL.md", "abcdef")]);
        let r = TruncationReport::build(&analysis, WarningMode::Off, &[], 3).unwrap();
        assert!(!r.warning_shown);
        assert!(r.signatures_seen.is_empty());
    }
}
