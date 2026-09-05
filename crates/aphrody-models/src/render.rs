// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 aphrody contributors
//
// Output rendering: one table model, five output formats.
//
// Every listing surface in the toolbox (installed models, the catalog,
// recommendations, and later OCR / transcription results) has the same shape:
// a title, some columns, some rows. Rendering that shape lives here so a
// caller picks a format once and every command honours it — a report can be
// piped into a document, an issue, or a static site without a second tool.
//
// Formats:
//
//   json      machine consumption; the caller supplies the value
//   markdown  GitHub-flavoured pipe table, escaped
//   html      standalone fragment or full document, escaped
//   text      aligned fixed-width columns for a terminal
//   csv       RFC 4180 quoting, for spreadsheets and `cut`
//
// Pure and allocation-only: no filesystem, no clock, builds for wasm32.

use core::fmt::Write as _;

/// An output format for a rendered report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum Format {
    /// Aligned fixed-width columns. The default for a terminal.
    #[default]
    Text,
    /// One JSON object. The caller renders its own value.
    Json,
    /// GitHub-flavoured Markdown pipe table.
    Markdown,
    /// HTML table, as a fragment or a standalone document.
    Html,
    /// RFC 4180 comma-separated values.
    Csv,
}

impl Format {
    /// Stable machine-friendly name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Json => "json",
            Self::Markdown => "markdown",
            Self::Html => "html",
            Self::Csv => "csv",
        }
    }

    /// Every format, in declaration order.
    #[must_use]
    pub const fn all() -> &'static [Self] {
        &[Self::Text, Self::Json, Self::Markdown, Self::Html, Self::Csv]
    }

    /// Parse a format name, accepting the common aliases people type:
    /// `md`, `txt`, `htm`, `tsv`.
    #[must_use]
    pub fn from_str_opt(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "text" | "txt" | "plain" | "table" => Some(Self::Text),
            "json" => Some(Self::Json),
            "markdown" | "md" => Some(Self::Markdown),
            "html" | "htm" => Some(Self::Html),
            "csv" => Some(Self::Csv),
            _ => None,
        }
    }

    /// Conventional file extension, for `--out report.<ext>`.
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Text => "txt",
            Self::Json => "json",
            Self::Markdown => "md",
            Self::Html => "html",
            Self::Csv => "csv",
        }
    }

    /// Guess a format from an output path's extension.
    #[must_use]
    pub fn from_extension(path: &str) -> Option<Self> {
        let ext = path.rsplit('.').next()?;
        if ext == path {
            // No dot at all: not an extension, just a name.
            return None;
        }
        Self::from_str_opt(ext)
    }
}

impl core::fmt::Display for Format {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A titled table: the common shape behind every rendered report.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Document / section heading.
    pub title: String,
    /// Optional lead paragraph rendered above the table.
    pub summary: Option<String>,
    /// Column headers.
    pub headers: Vec<String>,
    /// Rows, each expected to match `headers` in length. Short rows are padded
    /// and long rows truncated at render time, so a ragged caller degrades to
    /// a readable table instead of a panic.
    pub rows: Vec<Vec<String>>,
    /// Optional footer line, e.g. a total.
    pub footer: Option<String>,
}

impl Report {
    /// Start a report with a title and column headers.
    #[must_use]
    pub fn new(title: impl Into<String>, headers: &[&str]) -> Self {
        Self {
            title: title.into(),
            summary: None,
            headers: headers.iter().map(|h| (*h).to_owned()).collect(),
            rows: Vec::new(),
            footer: None,
        }
    }

    /// Attach a lead paragraph.
    #[must_use]
    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// Attach a footer line.
    #[must_use]
    pub fn with_footer(mut self, footer: impl Into<String>) -> Self {
        self.footer = Some(footer.into());
        self
    }

    /// Append a row.
    pub fn push(&mut self, row: Vec<String>) {
        self.rows.push(row);
    }

    /// Normalise a row to the header width.
    fn cell(row: &[String], index: usize) -> &str {
        row.get(index).map_or("", String::as_str)
    }

    /// Render in the requested format.
    ///
    /// [`Format::Json`] is not handled here: JSON output is the caller's own
    /// value, not a flattened table, so a caller that supports it serialises
    /// directly and never reaches this function. Asking for JSON anyway falls
    /// back to text rather than emitting something that only looks like JSON.
    #[must_use]
    pub fn render(&self, format: Format) -> String {
        match format {
            Format::Text | Format::Json => self.render_text(),
            Format::Markdown => self.render_markdown(),
            Format::Html => self.render_html(true),
            Format::Csv => self.render_csv(),
        }
    }

    /// Aligned fixed-width columns.
    #[must_use]
    pub fn render_text(&self) -> String {
        let widths = self.column_widths();
        let mut out = String::new();

        if !self.title.is_empty() {
            let _ = writeln!(out, "{}", self.title);
            let _ = writeln!(out, "{}", "=".repeat(self.title.chars().count()));
        }
        if let Some(summary) = &self.summary {
            let _ = writeln!(out, "{summary}\n");
        }

        let mut header_line = String::new();
        for (index, header) in self.headers.iter().enumerate() {
            let _ = write!(header_line, "{:<width$}  ", header, width = widths[index]);
        }
        let _ = writeln!(out, "{}", header_line.trim_end());
        let _ = writeln!(out, "{}", "-".repeat(header_line.trim_end().chars().count().max(1)));

        for row in &self.rows {
            let mut line = String::new();
            for index in 0..self.headers.len() {
                let _ =
                    write!(line, "{:<width$}  ", Self::cell(row, index), width = widths[index]);
            }
            let _ = writeln!(out, "{}", line.trim_end());
        }

        if let Some(footer) = &self.footer {
            let _ = writeln!(out, "\n{footer}");
        }
        out
    }

    /// GitHub-flavoured Markdown.
    #[must_use]
    pub fn render_markdown(&self) -> String {
        let mut out = String::new();
        if !self.title.is_empty() {
            let _ = writeln!(out, "# {}\n", self.title);
        }
        if let Some(summary) = &self.summary {
            let _ = writeln!(out, "{summary}\n");
        }

        let _ = writeln!(
            out,
            "| {} |",
            self.headers.iter().map(|h| escape_markdown(h)).collect::<Vec<_>>().join(" | ")
        );
        let _ = writeln!(
            out,
            "|{}|",
            self.headers.iter().map(|_| "---").collect::<Vec<_>>().join("|")
        );
        for row in &self.rows {
            let cells: Vec<String> = (0..self.headers.len())
                .map(|index| escape_markdown(Self::cell(row, index)))
                .collect();
            let _ = writeln!(out, "| {} |", cells.join(" | "));
        }

        if let Some(footer) = &self.footer {
            let _ = writeln!(out, "\n{footer}");
        }
        out
    }

    /// HTML table. `standalone` wraps it in a minimal document with styling;
    /// otherwise only the `<section>` fragment is emitted, for embedding.
    #[must_use]
    pub fn render_html(&self, standalone: bool) -> String {
        let mut body = String::new();
        let _ = writeln!(body, "<section class=\"aphrody-report\">");
        if !self.title.is_empty() {
            let _ = writeln!(body, "  <h1>{}</h1>", escape_html(&self.title));
        }
        if let Some(summary) = &self.summary {
            let _ = writeln!(body, "  <p>{}</p>", escape_html(summary));
        }
        let _ = writeln!(body, "  <table>");
        let _ = writeln!(body, "    <thead><tr>");
        for header in &self.headers {
            let _ = writeln!(body, "      <th>{}</th>", escape_html(header));
        }
        let _ = writeln!(body, "    </tr></thead>");
        let _ = writeln!(body, "    <tbody>");
        for row in &self.rows {
            let _ = writeln!(body, "      <tr>");
            for index in 0..self.headers.len() {
                let _ = writeln!(body, "        <td>{}</td>", escape_html(Self::cell(row, index)));
            }
            let _ = writeln!(body, "      </tr>");
        }
        let _ = writeln!(body, "    </tbody>");
        let _ = writeln!(body, "  </table>");
        if let Some(footer) = &self.footer {
            let _ = writeln!(body, "  <p class=\"footer\">{}</p>", escape_html(footer));
        }
        let _ = writeln!(body, "</section>");

        if !standalone {
            return body;
        }

        // Self-contained: no external stylesheet, no font fetch, and a colour
        // scheme that follows the reader rather than forcing one.
        format!(
            "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
             <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
             <title>{}</title>\n<style>\n{}\n</style>\n</head>\n<body>\n{body}</body>\n</html>\n",
            escape_html(&self.title),
            HTML_STYLE
        )
    }

    /// RFC 4180 CSV.
    #[must_use]
    pub fn render_csv(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "{}",
            self.headers.iter().map(|h| escape_csv(h)).collect::<Vec<_>>().join(",")
        );
        for row in &self.rows {
            let cells: Vec<String> =
                (0..self.headers.len()).map(|i| escape_csv(Self::cell(row, i))).collect();
            let _ = writeln!(out, "{}", cells.join(","));
        }
        out
    }

    /// Widest cell per column, headers included, floored at 1.
    fn column_widths(&self) -> Vec<usize> {
        self.headers
            .iter()
            .enumerate()
            .map(|(index, header)| {
                let widest_cell = self
                    .rows
                    .iter()
                    .map(|row| Self::cell(row, index).chars().count())
                    .max()
                    .unwrap_or(0);
                widest_cell.max(header.chars().count()).max(1)
            })
            .collect()
    }
}

/// Minimal stylesheet for standalone HTML output.
const HTML_STYLE: &str = "\
:root { color-scheme: light dark; --fg: #16181d; --bg: #ffffff; --line: #d8dde5; --muted: #5b6472; }
@media (prefers-color-scheme: dark) {
  :root { --fg: #e6e9ef; --bg: #14161a; --line: #2b3038; --muted: #9aa4b2; }
}
body { margin: 0; padding: 2rem 1.25rem; background: var(--bg); color: var(--fg);
  font: 15px/1.55 ui-sans-serif, system-ui, -apple-system, Segoe UI, Roboto, sans-serif; }
.aphrody-report { max-width: 68rem; margin: 0 auto; }
h1 { font-size: 1.4rem; margin: 0 0 .5rem; }
p { color: var(--muted); margin: 0 0 1.25rem; }
table { width: 100%; border-collapse: collapse; font-variant-numeric: tabular-nums; }
th, td { text-align: left; padding: .5rem .75rem; border-bottom: 1px solid var(--line); }
th { font-weight: 600; white-space: nowrap; }
td { vertical-align: top; }
tbody tr:last-child td { border-bottom: none; }
.footer { margin-top: 1.25rem; }";

/// Escape the characters that would break a Markdown pipe table.
///
/// A literal `|` ends a cell and a newline ends a row, so both are neutralised;
/// everything else Markdown does to text inside a table cell is cosmetic.
#[must_use]
pub fn escape_markdown(raw: &str) -> String {
    raw.replace('\\', "\\\\").replace('|', "\\|").replace(['\n', '\r'], " ")
}

/// Escape text for an HTML text node or a quoted attribute.
#[must_use]
pub fn escape_html(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            other => out.push(other),
        }
    }
    out
}

/// Quote a CSV field per RFC 4180 when it contains a delimiter, a quote or a
/// line break; a doubled quote escapes a literal one.
#[must_use]
pub fn escape_csv(raw: &str) -> String {
    if raw.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", raw.replace('"', "\"\""))
    } else {
        raw.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Report {
        let mut report = Report::new("Installed models", &["SIZE", "FORMAT", "REFERENCE"])
            .with_summary("2 models under ~/.aphrody/models")
            .with_footer("total 288 MiB");
        report.push(vec!["141.11 MiB".into(), "ggml".into(), "hf:a/b/base.bin".into()]);
        report.push(vec!["147.00 MiB".into(), "gguf".into(), "hf:c/d/m.gguf".into()]);
        report
    }

    #[test]
    fn format_names_and_aliases_parse() {
        assert_eq!(Format::from_str_opt("md"), Some(Format::Markdown));
        assert_eq!(Format::from_str_opt("MARKDOWN"), Some(Format::Markdown));
        assert_eq!(Format::from_str_opt("txt"), Some(Format::Text));
        assert_eq!(Format::from_str_opt(" html "), Some(Format::Html));
        assert_eq!(Format::from_str_opt("htm"), Some(Format::Html));
        assert_eq!(Format::from_str_opt("csv"), Some(Format::Csv));
        assert_eq!(Format::from_str_opt("pdf"), None);
        for format in Format::all() {
            assert_eq!(Format::from_str_opt(format.as_str()), Some(*format));
        }
    }

    #[test]
    fn format_is_inferred_from_an_output_path() {
        assert_eq!(Format::from_extension("report.md"), Some(Format::Markdown));
        assert_eq!(Format::from_extension("/tmp/out.html"), Some(Format::Html));
        assert_eq!(Format::from_extension("out.txt"), Some(Format::Text));
        assert_eq!(Format::from_extension("data.csv"), Some(Format::Csv));
        assert_eq!(Format::from_extension("report.pdf"), None);
        // A bare name is not an extension.
        assert_eq!(Format::from_extension("report"), None);
        assert_eq!(Format::Markdown.extension(), "md");
        assert_eq!(Format::Text.extension(), "txt");
    }

    #[test]
    fn markdown_is_a_valid_pipe_table() {
        let out = sample().render_markdown();
        assert!(out.starts_with("# Installed models\n"), "{out}");
        assert!(out.contains("| SIZE | FORMAT | REFERENCE |"), "{out}");
        assert!(out.contains("|---|---|---|"), "{out}");
        assert!(out.contains("| 141.11 MiB | ggml | hf:a/b/base.bin |"), "{out}");
        assert!(out.trim_end().ends_with("total 288 MiB"), "{out}");
        // Header row, delimiter row, and one row per record.
        let table_rows = out.lines().filter(|l| l.starts_with('|')).count();
        assert_eq!(table_rows, 4);
    }

    #[test]
    fn markdown_escapes_pipes_and_newlines_so_the_table_survives() {
        let mut report = Report::new("t", &["A"]);
        report.push(vec!["a|b\nc".into()]);
        let out = report.render_markdown();
        assert!(out.contains(r"a\|b c"), "{out}");
        // Exactly three table lines: header, delimiter, one row.
        assert_eq!(out.lines().filter(|l| l.starts_with('|')).count(), 3);
    }

    #[test]
    fn html_standalone_is_self_contained_and_theme_aware() {
        let out = sample().render_html(true);
        assert!(out.starts_with("<!doctype html>"), "{out}");
        assert!(out.contains("<title>Installed models</title>"));
        assert!(out.contains("color-scheme: light dark"));
        assert!(out.contains("prefers-color-scheme: dark"));
        // No external resource may be referenced.
        assert!(!out.contains("http://") && !out.contains("https://"), "{out}");
        assert!(out.contains("<td>141.11 MiB</td>"));
    }

    #[test]
    fn html_fragment_omits_the_document_wrapper() {
        let out = sample().render_html(false);
        assert!(!out.contains("<!doctype"));
        assert!(out.starts_with("<section class=\"aphrody-report\">"));
        assert!(out.trim_end().ends_with("</section>"));
    }

    #[test]
    fn html_escapes_markup_in_every_position() {
        let mut report = Report::new("<title> & \"quotes\"", &["<th>"])
            .with_summary("<script>alert(1)</script>")
            .with_footer("a > b");
        report.push(vec!["<img src=x onerror=alert(1)>".into()]);
        let out = report.render_html(true);
        // What matters is that no attacker-supplied text ever opens a tag:
        // the payload text may survive verbatim inside a text node, it just
        // must not be parsed as markup.
        assert!(!out.contains("<script>"), "{out}");
        assert!(!out.contains("<img"), "{out}");
        assert!(out.contains("&lt;script&gt;"), "{out}");
        assert!(out.contains("&lt;img src=x onerror=alert(1)&gt;"), "{out}");
        assert!(out.contains("<title>&lt;title&gt; &amp; &quot;quotes&quot;</title>"), "{out}");
    }

    #[test]
    fn text_columns_align_to_the_widest_cell() {
        let out = sample().render_text();
        let lines: Vec<&str> = out.lines().collect();
        // Title, underline, summary, blank, header, rule, two rows, blank, footer.
        assert_eq!(lines[0], "Installed models");
        assert_eq!(lines[1], "================");
        let header = lines.iter().find(|l| l.starts_with("SIZE")).unwrap();
        // `147.00 MiB` is 10 chars, so the column is 10 wide plus the gutter.
        assert!(header.starts_with("SIZE        FORMAT"), "{header:?}");
        assert!(out.contains("total 288 MiB"));
    }

    #[test]
    fn csv_quotes_only_what_rfc4180_requires() {
        let mut report = Report::new("t", &["A", "B"]);
        report.push(vec!["plain".into(), "has,comma".into()]);
        report.push(vec!["say \"hi\"".into(), "line\nbreak".into()]);
        let out = report.render_csv();
        assert!(out.starts_with("A,B\n"), "{out}");
        assert!(out.contains("plain,\"has,comma\""), "{out}");
        assert!(out.contains("\"say \"\"hi\"\"\",\"line\nbreak\""), "{out}");
    }

    #[test]
    fn ragged_rows_degrade_instead_of_panicking() {
        let mut report = Report::new("t", &["A", "B", "C"]);
        report.push(vec!["only-one".into()]);
        report.push(vec!["a".into(), "b".into(), "c".into(), "extra".into()]);
        for format in Format::all() {
            let out = report.render(*format);
            assert!(out.contains("only-one"), "{format}: {out}");
            // The surplus cell is dropped, never rendered into a fourth column.
            assert!(!out.contains("extra"), "{format}: {out}");
        }
    }

    #[test]
    fn an_empty_report_still_renders_a_header_in_every_format() {
        let report = Report::new("Nothing here", &["A", "B"]);
        for format in Format::all() {
            let out = report.render(*format);
            assert!(!out.is_empty(), "{format} rendered nothing");
            assert!(out.contains('A'), "{format}: {out}");
        }
    }

    #[test]
    fn json_falls_back_to_text_rather_than_faking_json() {
        // Callers serialise their own value for JSON; reaching `render` with
        // it must not produce something that merely resembles JSON.
        let out = sample().render(Format::Json);
        assert_eq!(out, sample().render_text());
        assert!(!out.trim_start().starts_with('{'));
    }
}
