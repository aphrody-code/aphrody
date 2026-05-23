// SPDX-License-Identifier: Apache-2.0
//! System-prompt assembler (AH-5).
//!
//! Turns the loaded workspace files into a [`SystemPromptView`] whose sections
//! borrow (`Cow::Borrowed`) the cached bytes wherever the budget did not
//! truncate them, falling back to `Cow::Owned` only for the truncated tail.
//! The section order is fixed by [`crate::filenames::BootstrapFile::ALL`]
//! (operating rules first, persona next, then identity/user/tools, then the
//! optional checklists, then the one-shot ritual, then memory) so the rendered
//! prompt is byte-stable across runs — which keeps the model's prompt cache
//! warm (the determinism rule in CLAUDE.md / openclaw AGENTS.md "Prompt
//! cache").
//!
//! This is the borrowed-view advantage the plan calls out: openclaw rebuilds a
//! full `String` per session; aphrody hands back a view that points straight
//! into the shared `Arc<Mmap>` for the common (untruncated) case.

use std::borrow::Cow;

use crate::budget::{apply, BootstrapBudget, BudgetInput, TruncationReport, WarningMode};
use crate::filenames::BootstrapFile;

/// A single rendered section of the system prompt.
#[derive(Debug, Clone)]
pub struct PromptSection<'a> {
    /// Which bootstrap file produced this section.
    pub file: BootstrapFile,
    /// Section title rendered as a heading.
    pub title: &'static str,
    /// Section body — borrowed from the cache when untruncated, owned when the
    /// budget cut it.
    pub body: Cow<'a, str>,
    /// Whether the budget truncated this section.
    pub truncated: bool,
}

/// The assembled system prompt as a borrowed view over the cache.
#[derive(Debug, Clone)]
pub struct SystemPromptView<'a> {
    /// Sections in deterministic injection order.
    pub sections: Vec<PromptSection<'a>>,
    /// Truncation report when any section was cut, else `None`.
    pub truncation: Option<TruncationReport>,
    /// The budget that produced this view.
    pub budget: BootstrapBudget,
}

/// One loaded file fed to the assembler: the file kind, its on-disk path
/// string, and a borrowed view of its content (from the cache / mmap).
#[derive(Debug, Clone)]
pub struct LoadedFile<'a> {
    /// Which bootstrap file this is.
    pub file: BootstrapFile,
    /// On-disk path string (for diagnostics / signature stability).
    pub path: String,
    /// Borrowed content; `None` when the file is missing.
    pub content: Option<&'a str>,
}

impl<'a> SystemPromptView<'a> {
    /// Assemble a view from loaded files + a budget.
    ///
    /// `files` may be in any order; the assembler reorders them into
    /// [`BootstrapFile::ALL`] order. Missing files are skipped from the output
    /// (but still counted by the budget analysis so the totals are honest).
    #[must_use]
    pub fn assemble(files: &[LoadedFile<'a>], budget: &BootstrapBudget) -> Self {
        Self::assemble_with_warning(files, budget, WarningMode::Once, &[])
    }

    /// Assemble with explicit warning dedup state (used by the runtime to
    /// thread the seen-signature history across sessions).
    #[must_use]
    pub fn assemble_with_warning(
        files: &[LoadedFile<'a>],
        budget: &BootstrapBudget,
        warning_mode: WarningMode,
        seen_signatures: &[String],
    ) -> Self {
        // Reorder into canonical injection order; keep at most one of each kind.
        let ordered: Vec<&LoadedFile<'a>> = BootstrapFile::ALL
            .into_iter()
            .filter_map(|kind| files.iter().find(|f| f.file == kind))
            .collect();

        // Build budget inputs (owned snapshot for the analysis).
        let inputs: Vec<BudgetInput> = ordered
            .iter()
            .map(|f| BudgetInput {
                name: f.file.filename().to_string(),
                path: f.path.clone(),
                content: f.content.map(str::to_string),
            })
            .collect();

        let (_injected_owned, analysis) = apply(budget, &inputs);

        // Build borrowed sections: when a file was NOT truncated, borrow the
        // original content directly (zero-copy); otherwise own the truncated
        // prefix computed here from the borrowed source.
        let mut sections: Vec<PromptSection<'a>> = Vec::new();
        for (loaded, analyzed) in ordered.iter().zip(analysis.files.iter()) {
            if analyzed.missing {
                continue;
            }
            let Some(src) = loaded.content else { continue };
            let trimmed = src.trim_end();
            let body: Cow<'a, str> = if analyzed.truncated {
                Cow::Owned(prefix_chars(trimmed, analyzed.injected_chars).to_string())
            } else {
                // Borrow the trimmed slice (a sub-slice of the cached bytes).
                Cow::Borrowed(trimmed)
            };
            sections.push(PromptSection {
                file: loaded.file,
                title: loaded.file.section_title(),
                body,
                truncated: analyzed.truncated,
            });
        }

        let truncation = TruncationReport::build(
            &analysis,
            warning_mode,
            seen_signatures,
            crate::budget::DEFAULT_WARNING_MAX_FILES,
        );

        Self {
            sections,
            truncation,
            budget: *budget,
        }
    }

    /// Render the view to a single deterministic system-prompt string.
    ///
    /// Format: each non-empty section as `## <title>\n\n<body>\n`, joined by a
    /// blank line, with the bootstrap truncation warning (when shown) appended
    /// as a trailing block — matching openclaw `appendBootstrapPromptWarning`.
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = String::new();
        for section in &self.sections {
            if section.body.trim().is_empty() {
                continue;
            }
            if !out.is_empty() {
                out.push_str("\n\n");
            }
            out.push_str("## ");
            out.push_str(section.title);
            out.push_str("\n\n");
            out.push_str(section.body.trim_end());
        }
        if let Some(report) = &self.truncation {
            if report.warning_shown && !report.lines.is_empty() {
                if !out.is_empty() {
                    out.push_str("\n\n");
                }
                out.push_str("[Bootstrap truncation warning]\n");
                out.push_str("Some workspace bootstrap files were truncated before injection.\n");
                out.push_str(
                    "Treat the context above as partial and read the relevant files directly if details seem missing.",
                );
                for line in &report.lines {
                    out.push_str("\n- ");
                    out.push_str(line.trim());
                }
            }
        }
        out
    }

    /// Whether any section was truncated.
    #[must_use]
    pub fn has_truncation(&self) -> bool {
        self.truncation.is_some()
    }
}

/// Borrow the first `n` scalar values of `s` as a sub-slice (no allocation).
fn prefix_chars(s: &str, n: usize) -> &str {
    match s.char_indices().nth(n) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loaded(file: BootstrapFile, content: &str) -> LoadedFile<'_> {
        LoadedFile {
            file,
            path: file.filename().to_string(),
            content: Some(content),
        }
    }

    #[test]
    fn sections_are_in_canonical_order_regardless_of_input() {
        let files = vec![
            loaded(BootstrapFile::User, "user body"),
            loaded(BootstrapFile::Agents, "agents body"),
            loaded(BootstrapFile::Soul, "soul body"),
        ];
        let view = SystemPromptView::assemble(&files, &BootstrapBudget::default());
        let order: Vec<BootstrapFile> = view.sections.iter().map(|s| s.file).collect();
        assert_eq!(order, vec![BootstrapFile::Agents, BootstrapFile::Soul, BootstrapFile::User]);
    }

    #[test]
    fn untruncated_sections_borrow_zero_copy() {
        let files = vec![loaded(BootstrapFile::Soul, "concise persona")];
        let view = SystemPromptView::assemble(&files, &BootstrapBudget::default());
        assert!(matches!(view.sections[0].body, Cow::Borrowed(_)));
        assert!(!view.sections[0].truncated);
    }

    #[test]
    fn truncated_section_owns_prefix() {
        let budget = BootstrapBudget {
            max_chars: 4,
            total_max_chars: 1000,
            near_limit_ratio: 0.85,
        };
        let files = vec![loaded(BootstrapFile::Soul, "0123456789")];
        let view = SystemPromptView::assemble(&files, &budget);
        assert!(matches!(view.sections[0].body, Cow::Owned(_)));
        assert_eq!(view.sections[0].body.as_ref(), "0123");
        assert!(view.sections[0].truncated);
        assert!(view.has_truncation());
    }

    #[test]
    fn render_is_deterministic() {
        let files = vec![
            loaded(BootstrapFile::Identity, "You are Aria."),
            loaded(BootstrapFile::Soul, "Precise and warm."),
        ];
        let a = SystemPromptView::assemble(&files, &BootstrapBudget::default()).render();
        let b = SystemPromptView::assemble(&files, &BootstrapBudget::default()).render();
        assert_eq!(a, b);
        // Soul precedes Identity in canonical order.
        let soul_pos = a.find("Soul (SOUL.md)").unwrap();
        let id_pos = a.find("Identity (IDENTITY.md)").unwrap();
        assert!(soul_pos < id_pos);
    }

    #[test]
    fn render_appends_truncation_warning() {
        let budget = BootstrapBudget {
            max_chars: 3,
            total_max_chars: 1000,
            near_limit_ratio: 0.85,
        };
        let files = vec![loaded(BootstrapFile::Soul, "abcdef")];
        let view = SystemPromptView::assemble(&files, &budget);
        let rendered = view.render();
        assert!(rendered.contains("[Bootstrap truncation warning]"));
    }

    #[test]
    fn missing_files_are_skipped_in_output() {
        let files = vec![
            LoadedFile {
                file: BootstrapFile::User,
                path: "USER.md".into(),
                content: None,
            },
            loaded(BootstrapFile::Soul, "persona"),
        ];
        let view = SystemPromptView::assemble(&files, &BootstrapBudget::default());
        assert_eq!(view.sections.len(), 1);
        assert_eq!(view.sections[0].file, BootstrapFile::Soul);
    }

    #[test]
    fn empty_sections_omitted_from_render() {
        let files = vec![loaded(BootstrapFile::Soul, "   \n  ")];
        let view = SystemPromptView::assemble(&files, &BootstrapBudget::default());
        assert_eq!(view.render(), "");
    }

    #[test]
    fn prefix_chars_handles_unicode() {
        assert_eq!(prefix_chars("aéb", 2), "aé");
        assert_eq!(prefix_chars("aéb", 99), "aéb");
        assert_eq!(prefix_chars("aéb", 0), "");
    }
}
