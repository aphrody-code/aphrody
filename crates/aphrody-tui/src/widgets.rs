// SPDX-License-Identifier: Apache-2.0
//! Aphrody-specific widgets — composed from stock `ratatui` primitives.
//!
//! Each widget implements [`ratatui::widgets::Widget`] (consume-by-value) so
//! it can be passed directly to [`ratatui::Frame::render_widget`].
//!
//! # Widget catalogue
//!
//! - [`SubAgentPane`] — list of running sub-agents with spinner + status.
//! - [`McpStatusBar`] — single-line bar of MCP server health dots.
//! - [`MarkdownView`] — render CommonMark via `aphrody-terminal-markdown`
//!   ([`aphrody_terminal_markdown::MarkdownRenderer::render_commonmark`]) and project the resulting
//!   ANSI-styled string into ratatui [`Line`]s.

use aphrody_terminal_markdown::MarkdownRenderer;
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Widget, Wrap},
};

use crate::ansi::ansi_to_lines;

// ---------------------------------------------------------------------------
// SubAgentPane
// ---------------------------------------------------------------------------

/// Lifecycle status of a sub-agent slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubAgentStatus {
    /// Agent is spinning up, executing, or otherwise mid-flight.
    Running,
    /// Agent finished normally.
    Done,
    /// Agent failed; the failure reason is shown alongside the name.
    Failed,
    /// Agent slot is allocated but not yet dispatched.
    Pending,
}

impl SubAgentStatus {
    /// Single-glyph indicator drawn next to the agent name.  ASCII only —
    /// the LLM-first terminal must never depend on emoji width tables.
    #[must_use]
    pub fn glyph(self, spinner_frame: u8) -> char {
        match self {
            // 4-frame ASCII spinner: |, /, -, \
            Self::Running => match spinner_frame % 4 {
                0 => '|',
                1 => '/',
                2 => '-',
                _ => '\\',
            },
            Self::Done => '+',
            Self::Failed => 'x',
            Self::Pending => '.',
        }
    }

    /// Foreground colour for the glyph.
    #[must_use]
    pub fn color(self) -> Color {
        match self {
            Self::Running => Color::Yellow,
            Self::Done => Color::Green,
            Self::Failed => Color::Red,
            Self::Pending => Color::Gray,
        }
    }
}

/// One row in the [`SubAgentPane`].
#[derive(Debug, Clone)]
pub struct SubAgentRow {
    /// Short human-readable name (e.g. `"yolo-prod-ready"`).
    pub name: String,
    /// Free-form one-line status (e.g. `"verifying cargo check"`).
    pub status_line: String,
    /// Lifecycle state — drives glyph + colour.
    pub status: SubAgentStatus,
}

/// Widget rendering a list of sub-agents with spinner + status.
#[derive(Debug, Clone)]
pub struct SubAgentPane {
    rows: Vec<SubAgentRow>,
    spinner_frame: u8,
    title: String,
}

impl SubAgentPane {
    /// Build a pane with the supplied rows.
    #[must_use]
    pub fn new(rows: Vec<SubAgentRow>) -> Self {
        Self { rows, spinner_frame: 0, title: "sub-agents".to_owned() }
    }

    /// Set the spinner phase (caller advances this each tick).
    #[must_use]
    pub fn spinner_frame(mut self, frame: u8) -> Self {
        self.spinner_frame = frame;
        self
    }

    /// Override the block title.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }
}

impl Widget for SubAgentPane {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let frame = self.spinner_frame;
        let items: Vec<ListItem<'_>> = self
            .rows
            .into_iter()
            .map(|row| {
                let glyph = row.status.glyph(frame);
                let color = row.status.color();
                let line = Line::from(vec![
                    Span::styled(format!("{glyph} "), Style::default().fg(color)),
                    Span::styled(row.name.clone(), Style::default().fg(Color::White)),
                    Span::raw("  "),
                    Span::styled(row.status_line, Style::default().fg(Color::Gray)),
                ]);
                ListItem::new(line)
            })
            .collect();

        let list = List::new(items).block(Block::default().borders(Borders::ALL).title(self.title));
        Widget::render(list, area, buf);
    }
}

// ---------------------------------------------------------------------------
// McpStatusBar
// ---------------------------------------------------------------------------

/// Health of a single MCP server slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpHealth {
    /// Connected and responsive.
    Up,
    /// Disconnected or unreachable.
    Down,
    /// Configured but not yet probed.
    Unknown,
}

impl McpHealth {
    fn color(self) -> Color {
        match self {
            Self::Up => Color::Green,
            Self::Down => Color::Red,
            Self::Unknown => Color::Gray,
        }
    }
}

/// Single MCP server slot rendered in the status bar.
#[derive(Debug, Clone)]
pub struct McpServerSlot {
    /// Short server name (e.g. `"context7"`, `"bxc"`).
    pub name: String,
    /// Current health.
    pub health: McpHealth,
}

/// One-line bar with a coloured dot + name per MCP server.
#[derive(Debug, Clone)]
pub struct McpStatusBar {
    slots: Vec<McpServerSlot>,
}

impl McpStatusBar {
    /// Build a bar from the supplied slots.
    #[must_use]
    pub fn new(slots: Vec<McpServerSlot>) -> Self {
        Self { slots }
    }
}

impl Widget for McpStatusBar {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let mut spans: Vec<Span<'_>> = Vec::with_capacity(self.slots.len() * 3);
        for (i, slot) in self.slots.into_iter().enumerate() {
            if i > 0 {
                spans.push(Span::raw("  "));
            }
            spans.push(Span::styled("*", Style::default().fg(slot.health.color())));
            spans.push(Span::raw(" "));
            spans.push(Span::styled(slot.name, Style::default().fg(Color::White)));
        }
        let line = Line::from(spans);
        let para = Paragraph::new(line);
        Widget::render(para, area, buf);
    }
}

// ---------------------------------------------------------------------------
// MarkdownView
// ---------------------------------------------------------------------------

/// Render a CommonMark document via
/// [`aphrody_terminal_markdown::MarkdownRenderer`] and project the resulting
/// ANSI-styled string into a styled ratatui [`Paragraph`].
///
/// The renderer is cached on the view so repeated draws (e.g. on every
/// 60 fps frame) don't reparse syntect themes from disk.  Constructing
/// the view does NOT pay the parse cost — that is deferred to
/// [`Widget::render`], which runs CommonMark -> ANSI -> [`Line`]s on each
/// draw call (a 1-3 ms hit for typical chat bubbles, dominated by syntect
/// fenced-code highlighting; the rest is allocation-cheap).
#[derive(Debug, Clone)]
pub struct MarkdownView {
    body: String,
    block_title: Option<String>,
    renderer: MarkdownRenderer,
}

impl MarkdownView {
    /// Build a view from a markdown source string.
    #[must_use]
    pub fn new(body: impl Into<String>) -> Self {
        Self { body: body.into(), block_title: None, renderer: MarkdownRenderer::new() }
    }

    /// Wrap the view in a bordered block with the supplied title.
    #[must_use]
    pub fn block_title(mut self, title: impl Into<String>) -> Self {
        self.block_title = Some(title.into());
        self
    }

    /// Override the syntect theme used for fenced code blocks
    /// (default: `aphrody_terminal_markdown::DEFAULT_THEME`).  Unknown
    /// theme names fall back silently to the default at render time.
    #[must_use]
    pub fn theme(mut self, theme: &str) -> Self {
        self.renderer = MarkdownRenderer::with_theme(theme);
        self
    }
}

impl Widget for MarkdownView {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let ansi = self.renderer.render_commonmark(&self.body);
        let lines = ansi_to_lines(&ansi);
        let mut para = Paragraph::new(lines).wrap(Wrap { trim: false });
        if let Some(title) = self.block_title {
            para = para.block(Block::default().borders(Borders::ALL).title(title));
        }
        Widget::render(para, area, buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spinner_cycles_four_frames() {
        let s = SubAgentStatus::Running;
        assert_eq!(s.glyph(0), '|');
        assert_eq!(s.glyph(1), '/');
        assert_eq!(s.glyph(2), '-');
        assert_eq!(s.glyph(3), '\\');
        assert_eq!(s.glyph(4), '|');
    }

    #[test]
    fn status_glyphs_distinct() {
        assert_eq!(SubAgentStatus::Done.glyph(0), '+');
        assert_eq!(SubAgentStatus::Failed.glyph(0), 'x');
        assert_eq!(SubAgentStatus::Pending.glyph(0), '.');
    }

    #[test]
    fn mcp_health_colors_are_stable() {
        assert_eq!(McpHealth::Up.color(), Color::Green);
        assert_eq!(McpHealth::Down.color(), Color::Red);
        assert_eq!(McpHealth::Unknown.color(), Color::Gray);
    }

    #[test]
    fn markdown_view_renders_via_terminal_markdown() {
        use ratatui::{Terminal, backend::TestBackend, style::Modifier};

        // 40x6 grid: ample room for "Hello" heading + "bold" inline run.
        let backend = TestBackend::new(40, 6);
        let mut terminal = Terminal::new(backend).expect("test backend");

        terminal
            .draw(|frame| {
                let view = MarkdownView::new("# Hello\n**bold** plain");
                frame.render_widget(view, frame.area());
            })
            .expect("draw ok");

        let buffer = terminal.backend().buffer().clone();

        // 1. "Hello" heading must appear somewhere in the buffer.
        let mut joined = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                joined.push_str(buffer[(x, y)].symbol());
            }
            joined.push('\n');
        }
        assert!(joined.contains("Hello"), "rendered buffer missing 'Hello' heading: {joined:?}");
        assert!(joined.contains("bold"), "rendered buffer missing 'bold' run: {joined:?}");
        assert!(
            joined.contains("plain"),
            "rendered buffer missing 'plain' trailing text: {joined:?}"
        );

        // 2. Locate the 'b' of "bold" and assert its cell carries the BOLD modifier — this is the
        //    load-bearing proof that render_commonmark + ansi_to_lines actually wired SGR 1 into
        //    ratatui Styles (i.e. we are NOT in the old passthrough path, which would have left
        //    modifiers empty).
        let mut bold_cell_found = false;
        'outer: for y in 0..buffer.area.height {
            for x in 0..buffer.area.width.saturating_sub(3) {
                if buffer[(x, y)].symbol() == "b"
                    && buffer[(x + 1, y)].symbol() == "o"
                    && buffer[(x + 2, y)].symbol() == "l"
                    && buffer[(x + 3, y)].symbol() == "d"
                {
                    let style = buffer[(x, y)].style();
                    assert!(
                        style.add_modifier.contains(Modifier::BOLD),
                        "'bold' run was not rendered as BOLD: {style:?}"
                    );
                    bold_cell_found = true;
                    break 'outer;
                }
            }
        }
        assert!(bold_cell_found, "could not locate 'bold' run in buffer");
    }
}
