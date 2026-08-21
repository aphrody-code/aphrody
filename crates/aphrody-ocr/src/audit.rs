// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//
// Quality audit over a batch of transcriptions.
//
// The filters in `doctags` run per page, at read time. This module asks the
// question a whole batch raises: is anything here obviously wrong before it is
// deposited into a corpus that other people will read?
//
// It exists because a deposit is hard to undo. Writing four hundred plates of
// degenerate output into a public database costs far more than the second it
// takes to check them, and "the filters ran" is not the same claim as "the
// output is clean" — a filter that silently stopped matching would leave both
// statements looking identical from the outside.
//
// Every finding names its page, so a caller can act on one plate rather than
// discarding a batch.

use std::path::PathBuf;

/// What is wrong with one transcription.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case", tag = "kind")]
#[non_exhaustive]
pub enum Finding {
    /// A chat control token survived into the text.
    ControlToken {
        /// The token that was found.
        token: String,
    },
    /// The same word repeats far past anything natural: a generation that got
    /// stuck and was not caught at read time.
    Loop {
        /// The repeated word.
        word: String,
        /// How many times in a row.
        repeats: usize,
    },
    /// Markup survived into what should be plain text.
    Markup {
        /// A sample of the offending text.
        sample: String,
    },
    /// A line that is nothing but a URL — a watermark that got through.
    Watermark {
        /// The line.
        line: String,
    },
}

impl Finding {
    /// Whether this finding should block a deposit.
    ///
    /// Control tokens, loops and markup are always defects. A watermark is
    /// noise rather than corruption: worth reporting, not worth refusing a
    /// batch over.
    #[must_use]
    pub const fn is_blocking(&self) -> bool {
        !matches!(self, Self::Watermark { .. })
    }
}

/// One page's findings.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct PageFindings {
    /// The image the transcription came from.
    pub image: PathBuf,
    /// What was found.
    pub findings: Vec<Finding>,
}

/// The result of auditing a batch.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize)]
pub struct AuditReport {
    /// Pages carrying text that were examined.
    pub examined: usize,
    /// Pages reported as having no text.
    pub textless: usize,
    /// Pages with at least one finding.
    pub flagged: Vec<PageFindings>,
}

impl AuditReport {
    /// Whether anything blocking was found.
    #[must_use]
    pub fn has_blocking(&self) -> bool {
        self.flagged.iter().any(|p| p.findings.iter().any(Finding::is_blocking))
    }

    /// Total findings across every page.
    #[must_use]
    pub fn finding_count(&self) -> usize {
        self.flagged.iter().map(|p| p.findings.len()).sum()
    }
}

/// Examine one transcription.
///
/// Returns every defect found, in a stable order so a report does not churn
/// between runs over the same input.
#[must_use]
pub fn audit_text(text: &str) -> Vec<Finding> {
    let mut findings = Vec::new();

    for token in ["<|endofassistant|>", "<|im_end|>", "<|end|>", "<|eot_id|>"] {
        if text.contains(token) {
            findings.push(Finding::ControlToken { token: token.to_owned() });
        }
    }

    if let Some((word, repeats)) = longest_run(text) {
        // Above five, no natural text repeats one word in a row.
        if repeats > 5 {
            findings.push(Finding::Loop { word, repeats });
        }
    }

    if let Some(sample) = markup_sample(text) {
        findings.push(Finding::Markup { sample });
    }

    for line in text.lines() {
        let trimmed = line.trim();
        if crate::doctags::strip_watermarks(trimmed).is_empty() && !trimmed.is_empty() {
            findings.push(Finding::Watermark { line: trimmed.to_owned() });
        }
    }

    findings
}

/// Audit a whole batch of `(image, text)` pairs.
///
/// `None` text means the page was reported textless, which is a verdict rather
/// than a defect and is only counted.
pub fn audit_batch<'a>(
    pages: impl IntoIterator<Item = (PathBuf, Option<&'a str>)>,
) -> AuditReport {
    let mut report = AuditReport::default();
    for (image, text) in pages {
        let Some(text) = text else {
            report.textless += 1;
            continue;
        };
        report.examined += 1;
        let findings = audit_text(text);
        if !findings.is_empty() {
            report.flagged.push(PageFindings { image, findings });
        }
    }
    report
}

/// The longest run of one repeated word, and its length.
fn longest_run(text: &str) -> Option<(String, usize)> {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut best: Option<(&str, usize)> = None;
    let mut run = 1_usize;

    for index in 1..words.len() {
        if words[index] == words[index - 1] {
            run += 1;
            if best.is_none_or(|(_, longest)| run > longest) {
                best = Some((words[index], run));
            }
        } else {
            run = 1;
        }
    }
    best.map(|(word, count)| (word.to_owned(), count))
}

/// A sample of surviving markup, if any.
///
/// Looks for a complete `<tag>` rather than a bare `<`, so a legitimate
/// `5 < 10` in a transcription is not reported.
fn markup_sample(text: &str) -> Option<String> {
    let start = text.find('<')?;
    let rest = &text[start + 1..];
    let end = rest.find('>')?;
    let tag = &rest[..end];
    // A tag name has no spaces; `a < b > c` is arithmetic, not markup.
    if tag.is_empty() || tag.contains(char::is_whitespace) {
        return None;
    }
    // `<|endofassistant|>` is a chat control token and already has its own
    // finding; reporting it twice would double-count one defect.
    if tag.starts_with('|') {
        return None;
    }
    Some(format!("<{tag}>"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_japanese_text_raises_nothing() {
        // A real transcription from the databook corpus.
        let text = "宿敵ベジータが悟空を認める!\nブウと闘う悟空を見て、素直に悟空が上だと認めるベジータ。";
        assert!(audit_text(text).is_empty(), "{:?}", audit_text(text));
    }

    #[test]
    fn a_surviving_control_token_is_blocking() {
        let findings = audit_text("読めた<|endofassistant|>");
        assert_eq!(findings.len(), 1);
        assert!(findings[0].is_blocking());
        assert!(matches!(&findings[0], Finding::ControlToken { token } if token.contains("endofassistant")));
    }

    #[test]
    fn a_stuck_generation_is_caught_with_its_word_and_count() {
        let findings = audit_text("bon début ふるさ ふるさ ふるさ ふるさ ふるさ ふるさ ふるさ");
        let loop_finding =
            findings.iter().find(|f| matches!(f, Finding::Loop { .. })).expect("no loop found");
        let Finding::Loop { word, repeats } = loop_finding else { unreachable!() };
        assert_eq!(word, "ふるさ");
        assert!(*repeats > 5, "{repeats}");
        assert!(loop_finding.is_blocking());
    }

    #[test]
    fn ordinary_repetition_is_not_a_loop() {
        // A table with a repeated value is normal text.
        let text = "Nom Goku Nom Vegeta Nom Piccolo Nom Krilin Nom Yamcha Nom Tenshinhan";
        assert!(!audit_text(text).iter().any(|f| matches!(f, Finding::Loop { .. })));
    }

    #[test]
    fn surviving_markup_is_reported() {
        let findings = audit_text("<td>cellule</td>");
        assert!(findings.iter().any(|f| matches!(f, Finding::Markup { .. })), "{findings:?}");
    }

    #[test]
    fn arithmetic_is_not_markup() {
        // `5 < 10 > 3` must not be read as a tag.
        assert!(!audit_text("5 < 10 > 3 toujours").iter().any(|f| matches!(f, Finding::Markup { .. })));
    }

    #[test]
    fn a_watermark_is_reported_but_does_not_block() {
        let findings = audit_text("宿敵\ncapsulecommentary.com");
        let watermark = findings
            .iter()
            .find(|f| matches!(f, Finding::Watermark { .. }))
            .expect("watermark not found");
        // Noise, not corruption: worth saying, not worth refusing a batch for.
        assert!(!watermark.is_blocking());
    }

    #[test]
    fn a_batch_separates_textless_pages_from_defects() {
        let report = audit_batch(vec![
            (PathBuf::from("a.jpg"), Some("texte propre")),
            (PathBuf::from("b.jpg"), None),
            (PathBuf::from("c.jpg"), Some("cassé<|im_end|>")),
        ]);
        assert_eq!(report.examined, 2);
        assert_eq!(report.textless, 1);
        assert_eq!(report.flagged.len(), 1);
        assert_eq!(report.flagged[0].image, PathBuf::from("c.jpg"));
        assert!(report.has_blocking());
        assert_eq!(report.finding_count(), 1);
    }

    #[test]
    fn a_clean_batch_has_nothing_blocking() {
        let report = audit_batch(vec![
            (PathBuf::from("a.jpg"), Some("DRAGON BALL 大全集")),
            (PathBuf::from("b.jpg"), None),
        ]);
        assert!(!report.has_blocking());
        assert_eq!(report.finding_count(), 0);
        assert!(report.flagged.is_empty());
    }

    #[test]
    fn an_empty_batch_audits_cleanly() {
        let report = audit_batch(Vec::new());
        assert_eq!(report.examined, 0);
        assert!(!report.has_blocking());
    }
}
