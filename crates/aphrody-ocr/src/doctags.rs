// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//
// DocTags parsing.
//
// The document VLMs in aphrody's catalog (granite-docling, and the Docling
// family generally) do not emit markdown: they emit DocTags, an XML-ish
// serialisation where each block carries a semantic tag and a quantised
// bounding box:
//
//   <doctag>
//     <section_header_level_1><loc_17><loc_68><loc_308><loc_126>TITLE</section_header_level_1>
//     <text><loc_18><loc_185><loc_209><loc_252>body</text>
//     <picture><loc_51><loc_68><loc_228><loc_154><other></picture>
//     <page_footer><loc_40><loc_480><loc_52><loc_485>103</page_footer>
//   </doctag>
//
// This module turns that into markdown and, just as importantly, answers
// "does this page carry any text at all?". That second question is what keeps
// a caption-happy model from filling a database with descriptions of pictures:
// a page whose only blocks are `<picture>` has NO text, and must be reported
// as such rather than as prose.
//
// Pure string work: no filesystem, no process, builds for wasm32.

/// One decoded block.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Block {
    /// The `DocTags` tag name, e.g. `text`, `section_header_level_1`.
    pub tag: String,
    /// Text content with the `<loc_*>` coordinates stripped.
    pub text: String,
}

impl Block {
    /// Whether this block is a heading, and at which level.
    #[must_use]
    pub fn heading_level(&self) -> Option<usize> {
        let rest = self.tag.strip_prefix("section_header_level_")?;
        // DocTags levels are 1-based and shallow; anything unparseable is not
        // a heading rather than a level-0 one.
        rest.parse::<usize>().ok().filter(|level| (1..=6).contains(level))
    }

    /// Whether the block is a page ornament rather than content.
    ///
    /// Running headers and folios repeat on every page of a scan; carrying
    /// them into a transcription adds noise to every single record.
    #[must_use]
    pub fn is_furniture(&self) -> bool {
        matches!(self.tag.as_str(), "page_header" | "page_footer")
    }

    /// Whether the block carries no readable text.
    ///
    /// `<picture>` blocks are the obvious case; a block whose payload is only
    /// punctuation or repeated filler (`4# 4# 4#`) is the subtler one — that is
    /// what a document model emits when it is shown text it cannot read, and
    /// it must not be mistaken for a transcription.
    #[must_use]
    pub fn is_empty_content(&self) -> bool {
        if self.tag == "picture" {
            return true;
        }
        let trimmed = self.text.trim();
        if trimmed.is_empty() {
            return true;
        }
        // Require at least one alphanumeric character somewhere.
        if !trimmed.chars().any(char::is_alphanumeric) {
            return true;
        }
        looks_like_filler(trimmed)
    }
}

/// Whether a block's text is degenerate repetition rather than a transcription.
///
/// A document model shown glyphs it cannot read does not fail — it emits a
/// short motif over and over (`4# 4# 4# 4# 4#` is what granite-docling
/// produced for a page of Japanese speech bubbles). Recording that as the
/// transcription of a plate would be worse than recording nothing.
///
/// The test is deliberately narrow: only text whose alphanumeric content
/// collapses to one or two distinct characters AND repeats them. Real short
/// text — a folio, a Japanese interjection, an initial — has either more
/// variety or not enough length to qualify.
#[must_use]
pub fn looks_like_filler(text: &str) -> bool {
    let alphanumeric: Vec<char> =
        text.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect();
    if alphanumeric.len() < 5 {
        return false;
    }
    let distinct: std::collections::BTreeSet<char> = alphanumeric.iter().copied().collect();
    distinct.len() <= 2
}

/// A parsed `DocTags` document.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Document {
    /// Every decoded block, in emission order.
    pub blocks: Vec<Block>,
}

impl Document {
    /// Parse a model's raw stdout into blocks.
    ///
    /// Tolerant by design: the surrounding `<doctag>` wrapper is optional, and
    /// an unterminated final block is kept rather than dropped, because a
    /// generation cut short by a token limit still holds usable text.
    #[must_use]
    pub fn parse(raw: &str) -> Self {
        let blocks = parse_blocks(raw);
        if !blocks.is_empty() {
            return Self { blocks };
        }

        // Not every vision model speaks DocTags. dots.ocr answers in plain
        // markdown, or in HTML for a tabular page. Treating a tagless answer
        // as "no text" would silently discard a good transcription, so the
        // whole response becomes one block, with any markup flattened. That
        // also turns a bare `<doctag></doctag>` — a model that found nothing —
        // into the empty string it means.
        let plain = html_to_text(&strip_control_tokens(raw));
        if plain.is_empty() {
            return Self { blocks: Vec::new() };
        }
        let (trimmed, looped) = truncate_loop(&plain);
        if looped {
            tracing::debug!(kept = trimmed.len(), "truncated a looping generation");
        }
        if trimmed.is_empty() {
            return Self { blocks: Vec::new() };
        }
        Self { blocks: vec![Block { tag: "text".to_owned(), text: trimmed }] }
    }

    /// Parse only the `DocTags` blocks, without the plain-text fallback.
    #[must_use]
    pub fn parse_doctags(raw: &str) -> Self {
        Self { blocks: parse_blocks(raw) }
    }

    /// Whether the page carries any readable text.
    ///
    /// Page furniture alone does not count: a folio number is not a
    /// transcription, and a page whose only decoded block is `103` should be
    /// recorded as textless.
    #[must_use]
    pub fn has_text(&self) -> bool {
        self.blocks.iter().any(|block| !block.is_furniture() && !block.is_empty_content())
    }

    /// Render the content blocks as light markdown.
    ///
    /// Returns `None` for a page with no text, which is the signal a caller
    /// forwards as a null transcription rather than an empty string.
    #[must_use]
    pub fn to_markdown(&self) -> Option<String> {
        if !self.has_text() {
            return None;
        }

        let mut out = String::new();
        for block in &self.blocks {
            if block.is_furniture() || block.is_empty_content() {
                continue;
            }
            let text = block.text.trim();
            match block.tag.as_str() {
                _ if block.heading_level().is_some() => {
                    let level = block.heading_level().unwrap_or(1);
                    out.push_str(&"#".repeat(level));
                    out.push(' ');
                    out.push_str(text);
                    out.push_str("\n\n");
                }
                "list_item" => {
                    out.push_str("- ");
                    out.push_str(text);
                    out.push('\n');
                }
                "caption" => {
                    out.push('*');
                    out.push_str(text);
                    out.push_str("*\n\n");
                }
                _ => {
                    out.push_str(text);
                    out.push_str("\n\n");
                }
            }
        }
        let trimmed = out.trim_end().to_owned();
        // A document that passed `has_text` but rendered to nothing would be a
        // bug in the filters above; report textless rather than an empty
        // transcription, which the databooks API would reject anyway.
        (!trimmed.is_empty()).then_some(trimmed)
    }
}

/// The `DocTags` block scan.
fn parse_blocks(raw: &str) -> Vec<Block> {
    let mut blocks = Vec::new();
    let mut cursor = raw;

    while let Some(open_start) = cursor.find('<') {
        let after_open = &cursor[open_start + 1..];
        let Some(open_end) = after_open.find('>') else { break };
        let tag = &after_open[..open_end];
        let body_start = open_start + 1 + open_end + 1;

        // Skip the wrapper, closing tags and self-contained markers.
        if tag == "doctag" || tag.starts_with('/') || is_marker(tag) {
            cursor = &cursor[body_start..];
            continue;
        }

        let closing = format!("</{tag}>");
        let body = &cursor[body_start..];
        let (payload, next) = match body.find(&closing) {
            Some(end) => (&body[..end], &body[end + closing.len()..]),
            // Truncated generation: take what is left and stop.
            None => (body, ""),
        };

        blocks.push(Block { tag: tag.to_owned(), text: strip_markers(payload) });
        if next.is_empty() {
            break;
        }
        cursor = next;
    }

    blocks
}

/// Flatten markup to text, replacing each tag with a space.
///
/// A tag becomes a separator rather than nothing, so `<td>A</td><td>B</td>`
/// reads as `A B` and not `AB`. Runs of whitespace collapse afterwards, which
/// also normalises the ragged spacing a model produces around markup.
#[must_use]
pub fn html_to_text(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(start) = rest.find('<') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        if let Some(end) = after.find('>') {
            out.push(' ');
            rest = &after[end + 1..];
        } else {
            // A dangling `<` is literal text (`5 < 10`), not a tag.
            out.push('<');
            rest = after;
        }
    }
    out.push_str(rest);

    // Collapse whitespace but keep paragraph breaks: a double newline is
    // structure a transcription should preserve.
    let mut normalised = String::with_capacity(out.len());
    let mut blank_run = 0_usize;
    for line in out.lines() {
        let trimmed = line.split_whitespace().collect::<Vec<_>>().join(" ");
        if trimmed.is_empty() {
            blank_run += 1;
            continue;
        }
        if !normalised.is_empty() {
            normalised.push('\n');
            if blank_run > 0 {
                normalised.push('\n');
            }
        }
        blank_run = 0;
        normalised.push_str(&trimmed);
    }
    normalised.trim().to_owned()
}

/// How many consecutive repeats of the same token run count as a stuck loop.
///
/// Four is above anything a real transcription produces — a table column of
/// identical values is separated by other tokens — and below the dozens a
/// degenerate model emits.
const LOOP_THRESHOLD: usize = 4;

/// Cut a generation at the point where the model started looping.
///
/// A vision model that runs out of readable content does not stop: it repeats
/// a short token run until the budget is spent. Observed on a Dragon Ball
/// databook plate, where dots.ocr read a whole technical sheet correctly and
/// then emitted `ふるさ` forty times.
///
/// Discarding the whole answer would throw away a good transcription over a
/// bad tail, so the loop is truncated and everything before it is kept.
/// Returns the trimmed text and whether a loop was found.
#[must_use]
pub fn truncate_loop(text: &str) -> (String, bool) {
    let tokens: Vec<&str> = text.split_whitespace().collect();
    if tokens.len() < LOOP_THRESHOLD * 2 {
        return (text.trim().to_owned(), false);
    }

    // Try short motifs first: `a a a a` should cut at the first `a`, not be
    // missed because `a a` also repeats.
    for window in 1..=4_usize {
        let mut index = 0;
        while index + window * (LOOP_THRESHOLD + 1) <= tokens.len() {
            let motif = &tokens[index..index + window];
            let mut repeats = 1;
            let mut probe = index + window;
            while probe + window <= tokens.len() && &tokens[probe..probe + window] == motif {
                repeats += 1;
                probe += window;
            }
            if repeats > LOOP_THRESHOLD {
                // Keep everything before the run, plus one instance of the
                // motif: a legitimate final word is often the loop's seed.
                let keep = tokens[..index + window].join(" ");
                return (keep.trim().to_owned(), true);
            }
            index += 1;
        }
    }

    (text.trim().to_owned(), false)
}

/// Strip chat control tokens and surrounding whitespace from a raw answer.
///
/// llama.cpp echoes the model's end-of-turn token (`<|endofassistant|>` for
/// dots.ocr) into stdout; leaving it in would put it in the database.
#[must_use]
pub fn strip_control_tokens(raw: &str) -> String {
    let mut out = raw.to_owned();
    for token in ["<|endofassistant|>", "<|im_end|>", "<|end|>", "<|eot_id|>", "</s>"] {
        out = out.replace(token, " ");
    }
    out.trim().to_owned()
}

/// Whether a tag is a self-contained marker rather than a block opener.
///
/// `<loc_123>` coordinates and `<other>` classifiers appear inline and never
/// have a closing tag.
fn is_marker(tag: &str) -> bool {
    // `<|endofassistant|>` and friends are chat control tokens, not blocks.
    // Without this they parse as a tag named `|endofassistant|`, which makes a
    // tagless answer look like a DocTags document and defeats the plain-text
    // fallback entirely.
    tag.starts_with('|')
        || tag.starts_with("loc_")
        || tag == "other"
        || tag.starts_with("page_break")
}

/// Remove `<loc_*>` / `<other>` markers from a block payload.
///
/// Shares [`html_to_text`] so a block that turns out to hold markup — an HTML
/// table nested inside what looked like a `DocTags` block — is flattened with
/// cell separators intact rather than run together.
fn strip_markers(payload: &str) -> String {
    html_to_text(payload)
}


#[cfg(test)]
mod tests {
    use super::*;

    const PAGE_WITH_TEXT: &str = "<doctag><section_header_level_1><loc_17><loc_68><loc_308><loc_126>APHRODY LOCAL INFERENCE</section_header_level_1>\n<text><loc_18><loc_185><loc_209><loc_252>Invoice 2026-08-21</text>\n<text><loc_18><loc_302><loc_213><loc_366>Total: 1337.42 EUR</text>\n</doctag>";

    const PICTURE_ONLY: &str = "<doctag><page_header><loc_42><loc_17><loc_213><loc_26>ORIGINAL COLOR WORKS part1</page_header>\n<picture><loc_51><loc_68><loc_228><loc_154><other></picture>\n<picture><loc_52><loc_159><loc_170><loc_261><other></picture>\n<page_footer><loc_40><loc_480><loc_52><loc_485>103</page_footer>\n<text><loc_328><loc_330><loc_397><loc_343>4# 4# 4# 4# 4#</text>\n</doctag>";

    #[test]
    fn a_real_page_parses_into_blocks_without_coordinates() {
        let doc = Document::parse(PAGE_WITH_TEXT);
        assert_eq!(doc.blocks.len(), 3);
        assert_eq!(doc.blocks[0].tag, "section_header_level_1");
        assert_eq!(doc.blocks[0].text, "APHRODY LOCAL INFERENCE");
        assert_eq!(doc.blocks[1].text, "Invoice 2026-08-21");
        assert_eq!(doc.blocks[2].text, "Total: 1337.42 EUR");
        // No coordinate marker may survive into the text.
        assert!(!doc.blocks[0].text.contains("loc_"));
    }

    #[test]
    fn markdown_uses_the_heading_level_from_the_tag() {
        let markdown = Document::parse(PAGE_WITH_TEXT).to_markdown().unwrap();
        assert!(markdown.starts_with("# APHRODY LOCAL INFERENCE"), "{markdown}");
        assert!(markdown.contains("Invoice 2026-08-21"));
        assert!(markdown.contains("Total: 1337.42 EUR"));
    }

    #[test]
    fn deeper_heading_levels_render_deeper() {
        let doc = Document::parse("<section_header_level_3><loc_1>Sub</section_header_level_3>");
        assert_eq!(doc.blocks[0].heading_level(), Some(3));
        assert!(doc.to_markdown().unwrap().starts_with("### Sub"));
    }

    #[test]
    fn a_picture_only_page_reports_no_text() {
        // This is the exact shape granite-docling returns for a manga plate it
        // cannot read: furniture, pictures, and one filler block.
        let doc = Document::parse(PICTURE_ONLY);
        assert!(!doc.has_text(), "{:#?}", doc.blocks);
        assert_eq!(doc.to_markdown(), None);
    }

    #[test]
    fn punctuation_only_blocks_carry_no_text() {
        for text in ["...", "— ‥ !!", "   ", "|||"] {
            let block = Block { tag: "text".into(), text: text.into() };
            assert!(block.is_empty_content(), "{text:?}");
        }
    }

    #[test]
    fn degenerate_repetition_is_rejected_as_filler() {
        // Exactly what granite-docling emitted for a page of Japanese speech
        // bubbles it could not read.
        assert!(looks_like_filler("4# 4# 4# 4# 4#"));
        assert!(looks_like_filler("aaaaaaa"));
        assert!(looks_like_filler("ab ab ab ab"));
        assert!(looks_like_filler("1 1 1 1 1 1"));
    }

    #[test]
    fn real_short_text_is_not_filler() {
        // A folio, an interjection, an initial, a price: all legitimate.
        assert!(!looks_like_filler("103"));
        assert!(!looks_like_filler("うんこたれ"));
        assert!(!looks_like_filler("Total: 1337.42 EUR"));
        assert!(!looks_like_filler("A."));
        assert!(!looks_like_filler("ORIGINAL COLOR WORKS part1"));
        // Too short to judge: two identical letters prove nothing.
        assert!(!looks_like_filler("aa"));
    }

    #[test]
    fn page_furniture_alone_is_not_text() {
        let doc = Document::parse(
            "<page_header><loc_1>Chapter 4</page_header><page_footer><loc_2>253</page_footer>",
        );
        assert_eq!(doc.blocks.len(), 2);
        assert!(doc.blocks.iter().all(Block::is_furniture));
        assert!(!doc.has_text());
        assert_eq!(doc.to_markdown(), None);
    }

    #[test]
    fn furniture_is_dropped_from_a_page_that_does_have_text() {
        let doc = Document::parse(
            "<page_header><loc_1>Running head</page_header><text><loc_2>Real body text</text><page_footer><loc_3>12</page_footer>",
        );
        let markdown = doc.to_markdown().unwrap();
        assert_eq!(markdown, "Real body text");
        assert!(!markdown.contains("Running head"));
        assert!(!markdown.contains("12"));
    }

    #[test]
    fn list_items_and_captions_get_their_markdown_form() {
        let doc = Document::parse(
            "<list_item><loc_1>First</list_item><list_item><loc_2>Second</list_item><caption><loc_3>Figure 1</caption>",
        );
        let markdown = doc.to_markdown().unwrap();
        assert!(markdown.contains("- First\n- Second"), "{markdown}");
        assert!(markdown.contains("*Figure 1*"), "{markdown}");
    }

    #[test]
    fn a_truncated_generation_keeps_what_it_produced() {
        // Hitting the token limit mid-block is normal on a dense page.
        let doc = Document::parse("<doctag><text><loc_1>Half a sentence that never clo");
        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(doc.blocks[0].text, "Half a sentence that never clo");
        assert!(doc.has_text());
    }

    #[test]
    fn output_without_the_doctag_wrapper_still_parses() {
        let doc = Document::parse("<text><loc_1>bare</text>");
        assert_eq!(doc.blocks.len(), 1);
        assert_eq!(doc.blocks[0].text, "bare");
    }

    #[test]
    fn empty_input_yields_no_text_rather_than_panicking() {
        for raw in ["", "   ", "<doctag></doctag>", "<|endofassistant|>"] {
            let doc = Document::parse(raw);
            assert!(!doc.has_text(), "{raw:?} produced {:#?}", doc.blocks);
            assert_eq!(doc.to_markdown(), None, "{raw:?}");
        }
    }

    #[test]
    fn html_output_is_flattened_to_readable_text() {
        // Exactly what dots.ocr returns for a tabular page.
        let raw = "<html><body><table><tr><td>APHRODY LOCAL INFERENCE</td></tr><tr><td>Total: 1337.42 EUR</td></tr></table></body></html><|endofassistant|>";
        let doc = Document::parse(raw);
        let markdown = doc.to_markdown().unwrap();
        assert!(markdown.contains("APHRODY LOCAL INFERENCE"), "{markdown}");
        assert!(markdown.contains("Total: 1337.42 EUR"), "{markdown}");
        // Cells must not run together, and no markup may survive.
        assert!(!markdown.contains("INFERENCETotal"), "{markdown}");
        assert!(!markdown.contains('<'), "{markdown}");
    }

    #[test]
    fn flattening_keeps_paragraph_structure() {
        let text = html_to_text("# Titre\n\n## Sous-titre\nligne");
        assert_eq!(text, "# Titre\n\n## Sous-titre\nligne");
    }

    #[test]
    fn flattening_keeps_a_literal_less_than() {
        assert_eq!(html_to_text("5 < 10 always"), "5 < 10 always");
    }

    #[test]
    fn prose_without_tags_is_text_not_garbage() {
        // The plain-text fallback: a model that answers in markdown must not
        // be read as an empty page.
        let doc = Document::parse("no tags here at all");
        assert!(doc.has_text());
        assert_eq!(doc.to_markdown().as_deref(), Some("no tags here at all"));
    }

    #[test]
    fn a_dangling_angle_bracket_is_treated_as_literal_text() {
        let doc = Document::parse("<text><loc_1>5 < 10 always</text>");
        assert!(doc.blocks[0].text.contains("10 always"), "{:?}", doc.blocks[0].text);
    }

    #[test]
    fn a_tagless_answer_is_kept_as_plain_text() {
        // dots.ocr answers in markdown, not DocTags. Treating that as "no
        // text" would discard a good transcription.
        let doc = Document::parse("# 【オートバイ】バイク\n\n## BIKE SELECTION<|endofassistant|>");
        assert_eq!(doc.blocks.len(), 1);
        assert!(doc.has_text());
        let markdown = doc.to_markdown().unwrap();
        assert!(markdown.contains("BIKE SELECTION"), "{markdown}");
        // The chat control token must never reach the database.
        assert!(!markdown.contains("endofassistant"), "{markdown}");
    }

    #[test]
    fn control_tokens_are_stripped() {
        assert_eq!(strip_control_tokens("hello<|endofassistant|>"), "hello");
        assert_eq!(strip_control_tokens("  a <|im_end|> "), "a");
        assert_eq!(strip_control_tokens("<|endofassistant|>"), "");
    }

    #[test]
    fn a_looping_generation_is_cut_but_its_good_prefix_is_kept() {
        // The real shape observed on a databook plate: a correct technical
        // sheet, then the same token forty times.
        let raw = "1 カプセル ナンバー No.9 バイク ふるさ ふるさ ふるさ ふるさ ふるさ ふるさ ふるさ ふるさ";
        let (kept, looped) = truncate_loop(raw);
        assert!(looped);
        assert!(kept.starts_with("1 カプセル ナンバー No.9 バイク"), "{kept}");
        // One instance of the motif may survive; dozens may not.
        assert!(kept.matches("ふるさ").count() <= 1, "{kept}");
    }

    #[test]
    fn multi_token_loops_are_caught_too() {
        let raw = "real content here ab cd ab cd ab cd ab cd ab cd ab cd";
        let (kept, looped) = truncate_loop(raw);
        assert!(looped);
        assert!(kept.starts_with("real content here"), "{kept}");
    }

    #[test]
    fn legitimate_repetition_is_not_truncated() {
        // A table with a repeated value, separated by other tokens.
        let raw = "Nom Goku Taille 175 cm Poids 62 kg Nom Vegeta Taille 164 cm Poids 56 kg";
        let (kept, looped) = truncate_loop(raw);
        assert!(!looped, "{kept}");
        assert_eq!(kept, raw);
    }

    #[test]
    fn short_text_is_never_considered_a_loop() {
        for raw in ["", "one", "a a a", "titre de section"] {
            let (kept, looped) = truncate_loop(raw);
            assert!(!looped, "{raw:?} -> {kept:?}");
        }
    }

    #[test]
    fn a_looping_answer_reaches_the_document_as_its_good_prefix() {
        let doc = Document::parse(
            "BIKE SELECTION ふるさ ふるさ ふるさ ふるさ ふるさ ふるさ ふるさ<|endofassistant|>",
        );
        let markdown = doc.to_markdown().unwrap();
        assert!(markdown.starts_with("BIKE SELECTION"), "{markdown}");
        assert!(markdown.matches("ふるさ").count() <= 1, "{markdown}");
    }

    #[test]
    fn parse_doctags_does_not_fall_back_to_plain_text() {
        // The strict parser is what a DocTags-only caller wants.
        assert!(Document::parse_doctags("just prose").blocks.is_empty());
        assert!(!Document::parse("just prose").blocks.is_empty());
    }

    #[test]
    fn japanese_text_survives_intact() {
        // The corpus this was built for is largely Japanese; a byte-oriented
        // strip would corrupt it.
        let doc = Document::parse("<text><loc_1>うんこたれじゃないーっ!!!!</text>");
        assert_eq!(doc.blocks[0].text, "うんこたれじゃないーっ!!!!");
        assert!(doc.has_text());
        assert_eq!(doc.to_markdown().as_deref(), Some("うんこたれじゃないーっ!!!!"));
    }
}
