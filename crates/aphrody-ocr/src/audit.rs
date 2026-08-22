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
///
/// Not `Eq`: one variant carries a measured share, and a float has no total
/// equality to offer.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
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
    /// The model's structured output survived into the text instead of being
    /// parsed.
    ///
    /// Four plates of the deposited corpus carry raw `{"bbox": [...],
    /// "category": ...}` where their transcription should be. Rare, and
    /// blocking without hesitation: what is in the database is not a bad
    /// reading of the plate, it is not a reading at all.
    RawJson {
        /// A sample of the offending text.
        sample: String,
    },
    /// The text has the shape of Japanese but forms almost no words: a plate
    /// the model looked at and improvised over.
    ///
    /// This is the defect none of the others can see. A stuck generation
    /// repeats, an empty page is empty, markup looks like markup — but a page
    /// of invented kana looks exactly like a page of real ones. Only a
    /// dictionary tells them apart.
    Charabia {
        /// Japanese characters examined.
        caracteres: usize,
        /// Share of them falling inside a morpheme the dictionary does not
        /// know, between 0 and 1.
        part_inconnue: f64,
    },
}

impl Finding {
    /// Whether this finding should block a deposit.
    ///
    /// Control tokens, loops and markup are always defects. A watermark is
    /// noise rather than corruption: worth reporting, not worth refusing a
    /// batch over.
    ///
    /// Gibberish is reported without blocking, which is a deliberately
    /// cautious choice rather than a judgement that it matters less — it
    /// matters more. The threshold behind it has been checked against
    /// hand-written cases, not yet against the corpus, and a heuristic that
    /// refuses four hundred plates has to earn that power on measured data
    /// first. Until then it points a reader at the pages worth opening.
    #[must_use]
    pub const fn is_blocking(&self) -> bool {
        !matches!(self, Self::Watermark { .. } | Self::Charabia { .. })
    }
}

/// One page's findings.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct PageFindings {
    /// The image the transcription came from.
    pub image: PathBuf,
    /// What was found.
    pub findings: Vec<Finding>,
}

/// The result of auditing a batch.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
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

    if let Some(at) = text.find("\"bbox\"") {
        // No databook prints that string; the model's JSON did.
        let fin = text.len().min(at + 120);
        let fin = (at..=fin).rev().find(|i| text.is_char_boundary(*i)).unwrap_or(at);
        findings.push(Finding::RawJson { sample: text[at..fin].to_owned() });
    }

    for line in text.lines() {
        let trimmed = line.trim();
        if crate::doctags::strip_watermarks(trimmed).is_empty() && !trimmed.is_empty() {
            findings.push(Finding::Watermark { line: trimmed.to_owned() });
        }
    }

    findings
}

/// Examine one transcription for defects only a Japanese dictionary can see.
///
/// Separate from [`audit_text`] because it needs IPADIC, which [`audit_text`]
/// deliberately does not: a caller auditing a French or English batch should
/// not pay for a dictionary it will never consult.
///
/// # Panics
///
/// Never: the comparison guards against a non-finite share.
#[cfg(feature = "japanese")]
#[must_use]
pub fn audit_japonais(text: &str, analyseur: &crate::japonais::Analyseur) -> Vec<Finding> {
    let mesure = analyseur.confiance(text);
    if !mesure.charabia() {
        return Vec::new();
    }
    vec![Finding::Charabia {
        caracteres: mesure.caracteres,
        part_inconnue: mesure.part_inconnue(),
    }]
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
    fn du_json_brut_dans_une_transcription_est_bloquant() {
        // Cas réel, planche 4/p91 : la sortie structurée du modèle est arrivée
        // telle quelle en base, à la place du texte.
        let texte = "た。\"}, {\"bbox\": [799, 405, 978, 627], \"category\": \"Picture\"}";
        let trouve = audit_text(texte);
        let json: Vec<&Finding> =
            trouve.iter().filter(|f| matches!(f, Finding::RawJson { .. })).collect();
        assert_eq!(json.len(), 1, "{trouve:?}");
        assert!(json[0].is_blocking(), "du JSON en base doit refuser le dépôt");
    }

    #[test]
    fn une_transcription_ordinaire_ne_declenche_pas_le_detecteur_de_json() {
        for texte in [
            "孫悟空は界王拳を使った。",
            "戦闘力は42000です",
            "Invoice 2026-08-21, total 1337.42 EUR",
        ] {
            assert!(
                !audit_text(texte).iter().any(|f| matches!(f, Finding::RawJson { .. })),
                "{texte}"
            );
        }
    }

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
