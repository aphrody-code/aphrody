// SPDX-License-Identifier: Apache-2.0
//! Ratatui `Frame` draw functions for the A2A coord channel TUI.
//!
//! Layout (terminal width >= 80 columns assumed):
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │  HEADER  aphrody [ONLINE 2m]          winclean [STALE 45m]              │
//! ├─────────────────────────────┬───────────────────────────────────────────┤
//! │  LIST (60% width)           │  DETAIL (40% width)                       │
//! │  id / ts / from→to / kind   │  pretty-printed JSON of selected body     │
//! │  (scrollable, j/k)          │                                           │
//! ├─────────────────────────────┴───────────────────────────────────────────┤
//! │  FOOTER  q quit  r refresh  / search  n/N match  Enter expand  Tab side │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```

use std::time::{Duration, SystemTime};

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::Envelope;

// ---------------------------------------------------------------------------
// M3 palette adapter
//
// When the `m3-tokens` optional dep is enabled we derive terminal colours
// from the canonical M3 ARGB roles.  Each ARGB u32 has the layout
// 0xAARRGGBB; we extract R/G/B and map to `Color::Rgb`.
//
// When `m3-tokens` is absent we fall back to a hand-curated ANSI-256 palette
// that mirrors the M3 intent.
// ---------------------------------------------------------------------------

/// Convert an M3 ARGB `u32` into a ratatui [`Color::Rgb`].
///
/// Only compiled when `m3-tokens` is active (enabled by the `native` feature).
#[cfg(feature = "m3-tokens")]
#[inline]
fn argb_to_rgb(argb: u32) -> Color {
    let r = ((argb >> 16) & 0xFF) as u8;
    let g = ((argb >> 8) & 0xFF) as u8;
    let b = (argb & 0xFF) as u8;
    Color::Rgb(r, g, b)
}

/// Terminal palette derived from the M3 baseline token set.
pub struct Palette {
    pub surface: Color,
    pub on_surface: Color,
    pub surface_variant: Color,
    pub on_surface_variant: Color,
    pub primary: Color,
    pub on_primary: Color,
    pub secondary: Color,
    pub tertiary: Color,
    pub error: Color,
}

impl Palette {
    /// Build a [`Palette`] from M3 tokens when the `m3-tokens` feature is
    /// enabled, otherwise fall back to hardcoded ANSI-256 colours.
    pub fn resolve() -> Self {
        #[cfg(feature = "m3-tokens")]
        {
            use m3_tokens::color::BASELINE_DARK as T;
            Self {
                surface: argb_to_rgb(T.surface),
                on_surface: argb_to_rgb(T.on_surface),
                surface_variant: argb_to_rgb(T.surface_variant),
                on_surface_variant: argb_to_rgb(T.on_surface_variant),
                primary: argb_to_rgb(T.primary),
                on_primary: argb_to_rgb(T.on_primary),
                secondary: argb_to_rgb(T.secondary),
                tertiary: argb_to_rgb(T.tertiary),
                error: argb_to_rgb(T.error),
            }
        }
        #[cfg(not(feature = "m3-tokens"))]
        {
            // Fallback: ANSI-256 approximation of M3 dark-theme baseline.
            Self {
                surface: Color::Indexed(234),            // near #1C1B1F
                on_surface: Color::Indexed(253),         // near #E6E1E5
                surface_variant: Color::Indexed(237),    // near #49454F
                on_surface_variant: Color::Indexed(251), // near #CAC4D0
                primary: Color::Indexed(183),            // near #D0BCFF
                on_primary: Color::Indexed(55),          // near #381E72
                secondary: Color::Indexed(182),          // near #CCC2DC
                tertiary: Color::Indexed(218),           // near #EFB8C8
                error: Color::Indexed(217),              // near #F2B8B0
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Envelope kind colour
// ---------------------------------------------------------------------------

/// Return a colour for an envelope `kind` string.
fn kind_color(kind: &str, pal: &Palette) -> Color {
    match kind {
        "fact" => Color::Cyan,
        "ask" => Color::Yellow,
        "ack" => Color::Green,
        "error" => pal.error,
        "ping" => pal.secondary,
        _ => pal.on_surface_variant,
    }
}

// ---------------------------------------------------------------------------
// Peer status header
// ---------------------------------------------------------------------------

/// Age of the heartbeat file expressed as a human-friendly string.
fn age_label(mtime: Option<SystemTime>) -> (&'static str, Color) {
    let Some(mtime) = mtime else {
        return ("UNKNOWN", Color::DarkGray);
    };
    let age = SystemTime::now().duration_since(mtime).unwrap_or(Duration::MAX);

    if age < Duration::from_secs(5 * 60) {
        ("ONLINE", Color::Green)
    } else if age < Duration::from_secs(30 * 60) {
        ("STALE", Color::Yellow)
    } else {
        ("OFFLINE", Color::Red)
    }
}

/// Format seconds as `Xm Ys` or `Xh Ym`.
fn fmt_age(mtime: Option<SystemTime>) -> String {
    let Some(mtime) = mtime else {
        return "?".to_owned();
    };
    let secs = SystemTime::now().duration_since(mtime).unwrap_or(Duration::ZERO).as_secs();
    if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    }
}

// ---------------------------------------------------------------------------
// Public draw entry point
// ---------------------------------------------------------------------------

/// All mutable draw state passed to [`draw`].
pub struct DrawState<'a> {
    /// Envelopes for the currently visible side.
    pub envelopes: &'a [Envelope],
    /// Ratatui list state (tracks cursor + offset).
    pub list_state: &'a mut ListState,
    /// Index of the selected envelope for the detail pane.
    pub selected: Option<usize>,
    /// Side currently shown: `"aphrody"` or `"winclean"`.
    pub active_side: &'a str,
    /// Heartbeat mtime for the aphrody side.
    pub aphrody_hb: Option<SystemTime>,
    /// Heartbeat mtime for the winclean side.
    pub winclean_hb: Option<SystemTime>,
    /// Active search query, or `None` if search bar is closed.
    pub search: Option<&'a str>,
    /// Indices of envelopes that match the active search query.
    pub search_matches: &'a [usize],
    /// Resolved M3 palette.
    pub palette: &'a Palette,
}

/// Draw the full TUI into `frame`.
pub fn draw(frame: &mut Frame, state: &mut DrawState) {
    let area = frame.area();

    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(0),    // body (list + detail)
            Constraint::Length(3), // footer
        ])
        .split(area);

    draw_header(frame, vertical[0], state);
    draw_body(frame, vertical[1], state);
    draw_footer(frame, vertical[2], state);
}

// ---------------------------------------------------------------------------
// Header
// ---------------------------------------------------------------------------

fn draw_header(frame: &mut Frame, area: Rect, state: &DrawState) {
    let pal = state.palette;

    let (a_label, a_color) = age_label(state.aphrody_hb);
    let (w_label, w_color) = age_label(state.winclean_hb);

    let a_age = fmt_age(state.aphrody_hb);
    let w_age = fmt_age(state.winclean_hb);

    let line = Line::from(vec![
        Span::styled(
            "  A2A coord channel  ",
            Style::default().fg(pal.on_primary).bg(pal.primary).add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled("aphrody", Style::default().fg(pal.primary).add_modifier(Modifier::BOLD)),
        Span::raw(" ["),
        Span::styled(a_label, Style::default().fg(a_color)),
        Span::styled(format!(" {a_age}"), Style::default().fg(pal.on_surface_variant)),
        Span::raw("]"),
        Span::raw("    "),
        Span::styled("winclean", Style::default().fg(pal.secondary).add_modifier(Modifier::BOLD)),
        Span::raw(" ["),
        Span::styled(w_label, Style::default().fg(w_color)),
        Span::styled(format!(" {w_age}"), Style::default().fg(pal.on_surface_variant)),
        Span::raw("]"),
    ]);

    let block = Block::default().borders(Borders::ALL).style(Style::default().bg(pal.surface));

    let paragraph = Paragraph::new(Text::from(line)).block(block);
    frame.render_widget(paragraph, area);
}

// ---------------------------------------------------------------------------
// Body (list + detail)
// ---------------------------------------------------------------------------

fn draw_body(frame: &mut Frame, area: Rect, state: &mut DrawState) {
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    draw_list(frame, horizontal[0], state);
    draw_detail(frame, horizontal[1], state);
}

fn draw_list(frame: &mut Frame, area: Rect, state: &mut DrawState) {
    let pal = state.palette;

    let title = format!(" inbox-from-{} ({}) ", state.active_side, state.envelopes.len());

    let match_set: std::collections::HashSet<usize> =
        state.search_matches.iter().copied().collect();

    let items: Vec<ListItem> = state
        .envelopes
        .iter()
        .enumerate()
        .map(|(i, env)| {
            let kc = kind_color(&env.kind, pal);
            let is_match = match_set.contains(&i);

            let from_to = if let Some(ref to) = env.to {
                format!("{}->{}", env.from, to)
            } else {
                env.from.clone()
            };

            // Truncate topic to 30 chars to keep the line readable.
            let topic: String = env.topic.chars().take(30).collect();

            // Short timestamp: take only the time portion (after 'T').
            let ts_short: &str = env.ts.find('T').map(|p| &env.ts[p + 1..]).unwrap_or(&env.ts);

            let line = Line::from(vec![
                Span::styled(format!("{:<8}", env.kind), Style::default().fg(kc)),
                Span::raw(" "),
                Span::styled(
                    format!("{:<20}", from_to),
                    Style::default().fg(pal.on_surface_variant),
                ),
                Span::raw(" "),
                Span::styled(format!("{:<10}", ts_short), Style::default().fg(pal.surface_variant)),
                Span::raw(" "),
                Span::styled(
                    topic,
                    if is_match {
                        Style::default()
                            .fg(Color::Black)
                            .bg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(pal.on_surface)
                    },
                ),
            ]);
            ListItem::new(line)
        })
        .collect();

    let block =
        Block::default().title(title).borders(Borders::ALL).style(Style::default().bg(pal.surface));

    let list = List::new(items)
        .block(block)
        .highlight_style(
            Style::default()
                .bg(pal.surface_variant)
                .fg(pal.on_surface)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("> ");

    frame.render_stateful_widget(list, area, state.list_state);
}

fn draw_detail(frame: &mut Frame, area: Rect, state: &DrawState) {
    let pal = state.palette;

    let content: Text = if let Some(idx) = state.selected {
        if let Some(env) = state.envelopes.get(idx) {
            build_detail_text(env, pal)
        } else {
            Text::raw("(no selection)")
        }
    } else {
        Text::raw("(press Enter to expand)")
    };

    let block = Block::default()
        .title(" detail ")
        .borders(Borders::ALL)
        .style(Style::default().bg(pal.surface));

    let paragraph = Paragraph::new(content).block(block).wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

/// Build a syntax-coloured detail view for `env`.
fn build_detail_text<'a>(env: &Envelope, pal: &Palette) -> Text<'a> {
    let kc = kind_color(&env.kind, pal);

    let mut lines: Vec<Line> = Vec::new();

    // --- header fields -------------------------------------------------------
    lines.push(Line::from(vec![
        Span::styled("kind:    ", Style::default().fg(pal.on_surface_variant)),
        Span::styled(env.kind.clone(), Style::default().fg(kc).add_modifier(Modifier::BOLD)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("from:    ", Style::default().fg(pal.on_surface_variant)),
        Span::styled(env.from.clone(), Style::default().fg(pal.primary)),
    ]));
    if let Some(ref to) = env.to {
        lines.push(Line::from(vec![
            Span::styled("to:      ", Style::default().fg(pal.on_surface_variant)),
            Span::styled(to.clone(), Style::default().fg(pal.secondary)),
        ]));
    }
    lines.push(Line::from(vec![
        Span::styled("ts:      ", Style::default().fg(pal.on_surface_variant)),
        Span::styled(env.ts.clone(), Style::default().fg(pal.on_surface)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("topic:   ", Style::default().fg(pal.on_surface_variant)),
        Span::styled(env.topic.clone(), Style::default().fg(pal.on_surface)),
    ]));
    if let Some(v) = env.v {
        lines.push(Line::from(vec![
            Span::styled("v:       ", Style::default().fg(pal.on_surface_variant)),
            Span::styled(v.to_string(), Style::default().fg(pal.on_surface_variant)),
        ]));
    }

    lines.push(Line::raw(""));
    lines.push(Line::from(Span::styled("body:", Style::default().fg(pal.on_surface_variant))));
    lines.push(Line::raw(""));

    // Try to pretty-print the body if it is JSON; fall back to raw text.
    let body_text = if let Ok(val) = serde_json::from_str::<serde_json::Value>(&env.body) {
        serde_json::to_string_pretty(&val).unwrap_or_else(|_| env.body.clone())
    } else {
        env.body.clone()
    };

    for raw_line in body_text.lines() {
        lines.push(colour_json_line(raw_line, pal));
    }

    Text::from(lines)
}

/// Very lightweight JSON syntax colouring: keys yellow, string values green,
/// numbers/booleans/null cyan, braces/brackets default.
fn colour_json_line<'a>(line: &str, pal: &Palette) -> Line<'a> {
    let trimmed = line.trim_start();
    let indent = &line[..line.len() - trimmed.len()];

    // Key detection: `"something":` pattern.
    if let Some(colon_pos) = trimmed.find("\": ").or_else(|| trimmed.find("\":")) {
        if trimmed.starts_with('"') {
            let key_with_colon = &trimmed[..colon_pos + 2];
            let rest = &trimmed[colon_pos + 2..];

            let value_color = if rest.trim_start().starts_with('"') {
                Color::Green
            } else if rest.trim_start().starts_with(['{', '[']) {
                pal.on_surface
            } else {
                Color::Cyan
            };

            return Line::from(vec![
                Span::raw(indent.to_owned()),
                Span::styled(key_with_colon.to_owned(), Style::default().fg(Color::Yellow)),
                Span::styled(rest.to_owned(), Style::default().fg(value_color)),
            ]);
        }
    }

    Line::from(Span::styled(line.to_owned(), Style::default().fg(pal.on_surface)))
}

// ---------------------------------------------------------------------------
// Footer
// ---------------------------------------------------------------------------

fn draw_footer(frame: &mut Frame, area: Rect, state: &DrawState) {
    let pal = state.palette;

    let search_hint = if let Some(q) = state.search {
        format!("  /{q}_  |  n/N match  |  Esc close  ")
    } else {
        "  / search  |  n/N match  ".to_owned()
    };

    let tab_label =
        if state.active_side == "aphrody" { "Tab -> winclean" } else { "Tab -> aphrody" };

    let line = Line::from(vec![
        Span::styled(" q ", Style::default().fg(pal.error).add_modifier(Modifier::BOLD)),
        Span::raw("quit  "),
        Span::styled(" r ", Style::default().fg(pal.primary).add_modifier(Modifier::BOLD)),
        Span::raw("refresh  "),
        Span::raw(search_hint),
        Span::styled(" Enter ", Style::default().fg(pal.tertiary).add_modifier(Modifier::BOLD)),
        Span::raw("expand  "),
        Span::styled(
            format!(" {tab_label} "),
            Style::default().fg(pal.on_surface_variant).add_modifier(Modifier::BOLD),
        ),
    ]);

    let block = Block::default().borders(Borders::ALL).style(Style::default().bg(pal.surface));

    let paragraph = Paragraph::new(Text::from(line)).block(block);
    frame.render_widget(paragraph, area);
}
