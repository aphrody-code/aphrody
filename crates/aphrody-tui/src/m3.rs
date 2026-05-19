// SPDX-License-Identifier: Apache-2.0
//! M3-themed production widgets for the aphrody TUI.
//!
//! Four widgets compose the canonical aphrody-TUI surface:
//!
//! - [`Block`] — bordered container with title + symmetric padding.
//! - [`Paragraph`] — unicode-aware word-wrap + vertical scroll.
//! - [`List`] — selectable rows with M3-themed highlight.
//! - [`Gauge`] — single-line progress bar.
//!
//! All four pull their colours from [`Palette`], which projects
//! [`m3_tokens::color::BASELINE_DARK`] (or `BASELINE`) into ratatui
//! [`Color::Rgb`] values. This keeps the TUI palette in lock-step with
//! the wgpu / web surfaces (no per-crate colour duplication — cf. memory
//! [[project_no_duplication_max_cohesion]]).
//!
//! Fusion sources (cf. CLAUDE.md §1):
//!   - `m3-tokens::color`                  — canonical M3 ARGB palette
//!   - `crate::ansi`                       — unicode handling pattern
//!   - `crate::layout`                     — Rect / Constraint dependency
//!   - `mui-rs::*`                         — sister M3 system on wgpu
//!   - `unicode-width` + `unicode-segmentation` — grapheme-aware wrap

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block as RtBlock, BorderType, Borders, List as RtList, ListItem,
        Paragraph as RtParagraph, Widget,
    },
};
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

// ---------------------------------------------------------------------------
// Palette bridge
// ---------------------------------------------------------------------------

/// Convert an `m3-tokens` ARGB `u32` (alpha in the high byte) into a
/// ratatui [`Color::Rgb`].  The terminal cannot render the alpha channel —
/// it is dropped intentionally, matching the M3 CSS export contract
/// (cf. [`m3_tokens::color::export_css`]).
#[must_use]
#[inline]
pub const fn argb_to_rgb(argb: u32) -> Color {
    let r = ((argb >> 16) & 0xFF) as u8;
    let g = ((argb >> 8) & 0xFF) as u8;
    let b = (argb & 0xFF) as u8;
    Color::Rgb(r, g, b)
}

/// Aphrody-flavoured M3 palette projected into ratatui [`Color`] / [`Style`]
/// values, sourced from [`m3_tokens::color::BASELINE_DARK`] (default) or
/// [`m3_tokens::color::BASELINE`].
///
/// Dark is the default surface for the LLM-first terminal (matches the
/// `crates/gui/src/index.html` baseline + memory
/// [[project_aphrody_providers_3only]]).  Use [`Palette::light`] to swap.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    /// Border / outline colour.
    pub outline: Color,
    /// Primary brand colour (active border, selected list row).
    pub primary: Color,
    /// On-primary contrast colour (text on a `primary` background).
    pub on_primary: Color,
    /// Surface colour (window/panel background).
    pub surface: Color,
    /// On-surface contrast colour (body text).
    pub on_surface: Color,
    /// Surface-variant colour (gauge troughs, list highlight bg).
    pub surface_variant: Color,
    /// On-surface-variant contrast colour (subtitles, dim labels).
    pub on_surface_variant: Color,
    /// Error colour (badges, failed states).
    pub error: Color,
}

impl Palette {
    /// Aphrody dark palette (default — pulled from `BASELINE_DARK`).
    #[must_use]
    pub const fn dark() -> Self {
        let t = m3_tokens::color::BASELINE_DARK;
        Self {
            outline: argb_to_rgb(t.outline),
            primary: argb_to_rgb(t.primary),
            on_primary: argb_to_rgb(t.on_primary),
            surface: argb_to_rgb(t.surface),
            on_surface: argb_to_rgb(t.on_surface),
            surface_variant: argb_to_rgb(t.surface_variant),
            on_surface_variant: argb_to_rgb(t.on_surface_variant),
            error: argb_to_rgb(t.error),
        }
    }

    /// Aphrody light palette (pulled from `BASELINE`).
    #[must_use]
    pub const fn light() -> Self {
        let t = m3_tokens::color::BASELINE;
        Self {
            outline: argb_to_rgb(t.outline),
            primary: argb_to_rgb(t.primary),
            on_primary: argb_to_rgb(t.on_primary),
            surface: argb_to_rgb(t.surface),
            on_surface: argb_to_rgb(t.on_surface),
            surface_variant: argb_to_rgb(t.surface_variant),
            on_surface_variant: argb_to_rgb(t.on_surface_variant),
            error: argb_to_rgb(t.error),
        }
    }
}

impl Default for Palette {
    fn default() -> Self {
        Self::dark()
    }
}

// ---------------------------------------------------------------------------
// Block
// ---------------------------------------------------------------------------

/// Visual style for a [`Block`] border.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BorderStyle {
    /// Single line, sharp corners.
    Plain,
    /// Single line, rounded corners.
    Rounded,
    /// Double line — used for "active focus" panels.
    Double,
    /// Heavy line — used for error / destructive panels.
    Thick,
}

impl BorderStyle {
    fn to_ratatui(self) -> BorderType {
        match self {
            Self::Plain => BorderType::Plain,
            Self::Rounded => BorderType::Rounded,
            Self::Double => BorderType::Double,
            Self::Thick => BorderType::Thick,
        }
    }
}

/// Symmetric inner padding for a [`Block`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Padding {
    /// Horizontal padding (cells on each side).
    pub horizontal: u16,
    /// Vertical padding (cells on top and bottom).
    pub vertical: u16,
}

impl Padding {
    /// Uniform padding (`horizontal = vertical = n`).
    #[must_use]
    pub const fn uniform(n: u16) -> Self {
        Self { horizontal: n, vertical: n }
    }

    /// Compute the inner area of `area` after applying padding + border.
    #[must_use]
    pub fn inner(self, area: Rect, border_cells: u16) -> Rect {
        let dx = border_cells.saturating_add(self.horizontal);
        let dy = border_cells.saturating_add(self.vertical);
        let w = area.width.saturating_sub(dx.saturating_mul(2));
        let h = area.height.saturating_sub(dy.saturating_mul(2));
        Rect::new(area.x.saturating_add(dx), area.y.saturating_add(dy), w, h)
    }
}

/// Bordered container with optional title — the canonical "panel" widget,
/// styled with the M3 palette so every aphrody surface looks like one
/// product.
#[derive(Debug, Clone)]
pub struct Block {
    title: Option<String>,
    borders: Borders,
    border_style: BorderStyle,
    padding: Padding,
    palette: Palette,
    focused: bool,
}

impl Block {
    /// Empty block — no title, all four borders, rounded corners, no padding.
    #[must_use]
    pub fn new() -> Self {
        Self {
            title: None,
            borders: Borders::ALL,
            border_style: BorderStyle::Rounded,
            padding: Padding::default(),
            palette: Palette::dark(),
            focused: false,
        }
    }

    /// Set the block title shown on the top border.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Override which sides receive a border (default: [`Borders::ALL`]).
    #[must_use]
    pub fn borders(mut self, b: Borders) -> Self {
        self.borders = b;
        self
    }

    /// Override the border style.
    #[must_use]
    pub fn border_style(mut self, s: BorderStyle) -> Self {
        self.border_style = s;
        self
    }

    /// Override the inner padding.
    #[must_use]
    pub fn padding(mut self, p: Padding) -> Self {
        self.padding = p;
        self
    }

    /// Override the M3 palette (default: [`Palette::dark`]).
    #[must_use]
    pub fn palette(mut self, p: Palette) -> Self {
        self.palette = p;
        self
    }

    /// Mark the block as focused — switches border to `Palette::primary`.
    #[must_use]
    pub fn focused(mut self, yes: bool) -> Self {
        self.focused = yes;
        self
    }

    /// Inner content area after subtracting borders + padding.
    #[must_use]
    pub fn inner(&self, area: Rect) -> Rect {
        let border_cells = u16::from(!self.borders.is_empty());
        self.padding.inner(area, border_cells)
    }
}

impl Default for Block {
    fn default() -> Self {
        Self::new()
    }
}

impl Widget for Block {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let border_color = if self.focused { self.palette.primary } else { self.palette.outline };
        let mut b = RtBlock::default()
            .borders(self.borders)
            .border_type(self.border_style.to_ratatui())
            .border_style(Style::default().fg(border_color));
        if let Some(t) = self.title {
            let title_style =
                Style::default().fg(self.palette.on_surface).add_modifier(Modifier::BOLD);
            b = b.title(Line::from(Span::styled(t, title_style)));
        }
        Widget::render(b, area, buf);
    }
}

// ---------------------------------------------------------------------------
// Paragraph
// ---------------------------------------------------------------------------

/// Wrapping policy for a [`Paragraph`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WrapMode {
    /// No wrapping — long lines are clipped at the right edge.
    None,
    /// Word-wrap at the nearest grapheme boundary that fits the width.
    Word,
}

/// Multi-line text widget with optional word-wrap + scroll offset.
///
/// Built on top of [`UnicodeWidthStr`] so CJK / combining marks count
/// correctly, and [`UnicodeSegmentation`] for grapheme-aware breaking.
#[derive(Debug, Clone)]
pub struct Paragraph {
    text: String,
    wrap: WrapMode,
    scroll: u16,
    palette: Palette,
    style: Style,
}

impl Paragraph {
    /// Build a paragraph from the supplied text.
    #[must_use]
    pub fn new(text: impl Into<String>) -> Self {
        let palette = Palette::dark();
        Self {
            text: text.into(),
            wrap: WrapMode::Word,
            scroll: 0,
            palette,
            style: Style::default().fg(palette.on_surface),
        }
    }

    /// Override the wrap mode (default: [`WrapMode::Word`]).
    #[must_use]
    pub fn wrap(mut self, w: WrapMode) -> Self {
        self.wrap = w;
        self
    }

    /// Skip the first `n` rendered rows (post-wrap).
    #[must_use]
    pub fn scroll(mut self, n: u16) -> Self {
        self.scroll = n;
        self
    }

    /// Override the foreground style.
    #[must_use]
    pub fn style(mut self, s: Style) -> Self {
        self.style = s;
        self
    }

    /// Override the M3 palette.
    #[must_use]
    pub fn palette(mut self, p: Palette) -> Self {
        self.palette = p;
        self.style = self.style.fg(p.on_surface);
        self
    }

    /// Wrap `text` into rows of width ≤ `width` cells, grapheme-aware.
    /// Exposed for unit tests + callers that want to pre-measure.
    #[must_use]
    pub fn wrap_to_rows(text: &str, width: u16, mode: WrapMode) -> Vec<String> {
        if width == 0 {
            return Vec::new();
        }
        let limit = width as usize;
        let mut rows: Vec<String> = Vec::new();
        for src_line in text.split('\n') {
            if mode == WrapMode::None {
                rows.push(src_line.to_owned());
                continue;
            }

            let mut current = String::new();
            let mut current_w: usize = 0;

            for tok in src_line.split_whitespace() {
                let tok_w = tok.width();
                if tok_w > limit {
                    if !current.is_empty() {
                        rows.push(std::mem::take(&mut current));
                        current_w = 0;
                    }
                    let mut chunk = String::new();
                    let mut chunk_w: usize = 0;
                    for g in tok.graphemes(true) {
                        let gw = g.width();
                        if chunk_w + gw > limit && !chunk.is_empty() {
                            rows.push(std::mem::take(&mut chunk));
                            chunk_w = 0;
                        }
                        chunk.push_str(g);
                        chunk_w += gw;
                    }
                    if !chunk.is_empty() {
                        current = chunk;
                        current_w = chunk_w;
                    }
                    continue;
                }
                let needed = if current.is_empty() {
                    tok_w
                } else {
                    current_w.saturating_add(1 + tok_w)
                };
                if needed > limit && !current.is_empty() {
                    rows.push(std::mem::take(&mut current));
                    current_w = 0;
                }
                if !current.is_empty() {
                    current.push(' ');
                    current_w += 1;
                }
                current.push_str(tok);
                current_w += tok_w;
            }
            rows.push(current);
        }
        rows
    }
}

impl Widget for Paragraph {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let rows = Self::wrap_to_rows(&self.text, area.width, self.wrap);
        let visible: Vec<Line<'static>> = rows
            .into_iter()
            .skip(self.scroll as usize)
            .take(area.height as usize)
            .map(|s| Line::from(Span::styled(s, self.style)))
            .collect();
        let para = RtParagraph::new(visible);
        Widget::render(para, area, buf);
    }
}

// ---------------------------------------------------------------------------
// List
// ---------------------------------------------------------------------------

/// Selectable list. `selected = None` means no row is highlighted.
#[derive(Debug, Clone)]
pub struct List {
    items: Vec<String>,
    selected: Option<usize>,
    highlight_style: Style,
    palette: Palette,
    highlight_symbol: String,
}

impl List {
    /// Build a list from owned items.
    #[must_use]
    pub fn new(items: Vec<String>) -> Self {
        let palette = Palette::dark();
        Self {
            items,
            selected: None,
            highlight_style: Style::default()
                .bg(palette.primary)
                .fg(palette.on_primary)
                .add_modifier(Modifier::BOLD),
            palette,
            highlight_symbol: "> ".to_owned(),
        }
    }

    /// Set the selected row index (clamped at render time).
    #[must_use]
    pub fn selected(mut self, idx: Option<usize>) -> Self {
        self.selected = idx;
        self
    }

    /// Override the highlight style.
    #[must_use]
    pub fn highlight_style(mut self, s: Style) -> Self {
        self.highlight_style = s;
        self
    }

    /// Override the highlight marker (default: `"> "`).
    #[must_use]
    pub fn highlight_symbol(mut self, s: impl Into<String>) -> Self {
        self.highlight_symbol = s.into();
        self
    }

    /// Swap palette — also resets highlight style to the new primary colour.
    #[must_use]
    pub fn palette(mut self, p: Palette) -> Self {
        self.palette = p;
        self.highlight_style =
            Style::default().bg(p.primary).fg(p.on_primary).add_modifier(Modifier::BOLD);
        self
    }
}

impl Widget for List {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let max_w = area.width as usize;
        let sel = self.selected.filter(|i| *i < self.items.len());
        let body_style = Style::default().fg(self.palette.on_surface);

        let items: Vec<ListItem<'_>> = self
            .items
            .iter()
            .enumerate()
            .map(|(idx, item)| {
                let is_sel = Some(idx) == sel;
                let prefix = if is_sel { self.highlight_symbol.as_str() } else { "  " };
                let cell_style = if is_sel { self.highlight_style } else { body_style };

                let prefix_w = prefix.width();
                let budget = max_w.saturating_sub(prefix_w);
                let mut truncated = String::with_capacity(item.len());
                let mut w: usize = 0;
                for g in item.graphemes(true) {
                    let gw = g.width();
                    if w + gw > budget {
                        break;
                    }
                    truncated.push_str(g);
                    w += gw;
                }
                let line = Line::from(vec![
                    Span::styled(prefix.to_owned(), cell_style),
                    Span::styled(truncated, cell_style),
                ]);
                ListItem::new(line)
            })
            .collect();

        let list = RtList::new(items);
        Widget::render(list, area, buf);
    }
}

// ---------------------------------------------------------------------------
// Gauge
// ---------------------------------------------------------------------------

/// Single-line progress bar in `[0.0, 1.0]`. Renders into the first row of
/// `area` regardless of height (so it composes inside a vsplit cell).
#[derive(Debug, Clone)]
pub struct Gauge {
    ratio: f32,
    label: Option<String>,
    palette: Palette,
    style: Option<Style>,
}

impl Gauge {
    /// Build a gauge clamped to `[0.0, 1.0]`.
    #[must_use]
    pub fn new(ratio: f32) -> Self {
        Self {
            ratio: ratio.clamp(0.0, 1.0),
            label: None,
            palette: Palette::dark(),
            style: None,
        }
    }

    /// Set the centred label (default: `"NN%"` derived from ratio).
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Override the M3 palette.
    #[must_use]
    pub fn palette(mut self, p: Palette) -> Self {
        self.palette = p;
        self
    }

    /// Override the fill style explicitly (overrides palette).
    #[must_use]
    pub fn style(mut self, s: Style) -> Self {
        self.style = Some(s);
        self
    }

    /// Current ratio (post-clamp).
    #[must_use]
    pub const fn ratio_value(&self) -> f32 {
        self.ratio
    }
}

impl Widget for Gauge {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let y = area.y;
        let width = area.width as usize;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let filled = ((self.ratio * width as f32).round() as usize).min(width);

        let fill_style = self
            .style
            .unwrap_or_else(|| Style::default().bg(self.palette.primary).fg(self.palette.on_primary));
        let trough_style =
            Style::default().bg(self.palette.surface_variant).fg(self.palette.on_surface_variant);

        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let pct = (self.ratio * 100.0).round() as i32;
        let label = self.label.clone().unwrap_or_else(|| format!("{pct}%"));
        let label_w = label.width();
        let label_x = area.x as usize + width.saturating_sub(label_w) / 2;
        let label_end = label_x + label_w;
        let label_graphemes: Vec<&str> = label.graphemes(true).collect();
        let mut label_iter = label_graphemes.into_iter();

        for col in 0..width {
            let abs_x = area.x as usize + col;
            let in_fill = col < filled;
            let style = if in_fill { fill_style } else { trough_style };
            let symbol: &str = if abs_x >= label_x && abs_x < label_end {
                label_iter.next().unwrap_or(" ")
            } else if in_fill {
                "\u{2588}"
            } else {
                " "
            };
            let cell = &mut buf[(abs_x as u16, y)];
            cell.set_symbol(symbol);
            cell.set_style(style);
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argb_to_rgb_drops_alpha() {
        assert_eq!(argb_to_rgb(0xFF6750A4), Color::Rgb(0x67, 0x50, 0xA4));
        assert_eq!(argb_to_rgb(0x006750A4), Color::Rgb(0x67, 0x50, 0xA4));
    }

    #[test]
    fn palette_dark_differs_from_light() {
        let d = Palette::dark();
        let l = Palette::light();
        assert_ne!(d.surface, l.surface);
    }

    #[test]
    fn padding_inner_subtracts_border_and_padding() {
        let p = Padding::uniform(2);
        let r = p.inner(Rect::new(0, 0, 20, 10), 1);
        // border=1, pad=2 => shrink by 3 on each side: 20-6=14, 10-6=4.
        assert_eq!(r.width, 14);
        assert_eq!(r.height, 4);
        assert_eq!(r.x, 3);
        assert_eq!(r.y, 3);
    }

    #[test]
    fn paragraph_wrap_respects_width() {
        let rows = Paragraph::wrap_to_rows("alpha beta gamma delta", 10, WrapMode::Word);
        for r in &rows {
            assert!(r.width() <= 10, "row overflow: {r:?}");
        }
    }

    #[test]
    fn paragraph_wrap_none_keeps_full_lines() {
        let rows = Paragraph::wrap_to_rows("very long single line", 5, WrapMode::None);
        assert_eq!(rows, vec!["very long single line".to_owned()]);
    }

    #[test]
    fn gauge_clamps_out_of_range() {
        assert!((Gauge::new(1.5).ratio_value() - 1.0).abs() < f32::EPSILON);
        assert!((Gauge::new(-0.1).ratio_value() - 0.0).abs() < f32::EPSILON);
    }
}
