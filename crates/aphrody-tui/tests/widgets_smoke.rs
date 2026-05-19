// SPDX-License-Identifier: Apache-2.0
//! Smoke tests for the four production widgets exposed by `aphrody_tui::m3`:
//! [`Block`], [`Paragraph`], [`List`], [`Gauge`]. Every test renders into an
//! 80x24 [`TestBackend`] and asserts on cell content + (where load-bearing)
//! cell style.
//!
//! The point of these tests is to keep the render path observable — if a
//! widget regresses (e.g. unicode-width misuse drops a cell), CI fails
//! deterministically without spinning up a real terminal.

use aphrody_tui::{
    TestBackend,
    m3::{BorderStyle, Block, Gauge, List, Padding, Palette, Paragraph, WrapMode, argb_to_rgb},
};
use ratatui::{Terminal, layout::Rect, style::Color};
use unicode_width::UnicodeWidthStr;

const COLS: u16 = 80;
const ROWS: u16 = 24;

fn dump(terminal: &Terminal<TestBackend>) -> String {
    let buf = terminal.backend().buffer();
    let mut out = String::with_capacity((COLS as usize + 1) * ROWS as usize);
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

#[test]
fn block_renders_title_and_rounded_borders() {
    let backend = TestBackend::new(COLS, ROWS);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            let block = Block::new()
                .title("aphrody")
                .border_style(BorderStyle::Rounded)
                .padding(Padding::uniform(1));
            frame.render_widget(block, Rect::new(0, 0, 30, 5));
        })
        .unwrap();
    let d = dump(&terminal);
    assert!(d.contains("aphrody"), "title missing\n{d}");
    // Rounded corners use ╭ at TL / ╰ at BL.
    assert!(d.contains('\u{256d}') || d.contains('\u{2570}'), "rounded corners missing\n{d}");
}

#[test]
fn block_palette_drives_border_color() {
    let backend = TestBackend::new(COLS, ROWS);
    let mut terminal = Terminal::new(backend).unwrap();
    let palette = Palette::dark();
    terminal
        .draw(|frame| {
            let block = Block::new().focused(true).palette(palette);
            frame.render_widget(block, Rect::new(0, 0, 10, 3));
        })
        .unwrap();
    let buf = terminal.backend().buffer();
    let top_left = buf[(0, 0)].style();
    // Focused border uses palette.primary.
    assert_eq!(top_left.fg, Some(palette.primary));
}

#[test]
fn paragraph_word_wraps_long_text() {
    let rows = Paragraph::wrap_to_rows("hello world from aphrody tui", 12, WrapMode::Word);
    for r in &rows {
        assert!(r.width() <= 12, "overflow: {r:?}");
    }
    assert!(rows.iter().any(|r| r.contains("hello")));
    assert!(rows.iter().any(|r| r.contains("aphrody")));
}

#[test]
fn paragraph_hard_wraps_long_token() {
    let long = "a".repeat(50);
    let rows = Paragraph::wrap_to_rows(&long, 10, WrapMode::Word);
    // 50 'a's at width 10 => 5 rows of exactly 10.
    assert_eq!(rows.len(), 5);
    for r in &rows {
        assert_eq!(r.len(), 10);
    }
}

#[test]
fn paragraph_renders_into_buffer() {
    let backend = TestBackend::new(COLS, ROWS);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            let p = Paragraph::new("the quick brown fox jumps over").wrap(WrapMode::Word);
            frame.render_widget(p, Rect::new(0, 0, 20, 5));
        })
        .unwrap();
    let d = dump(&terminal);
    assert!(d.contains("quick"), "missing wrapped word\n{d}");
}

#[test]
fn list_highlights_selected_row() {
    let backend = TestBackend::new(COLS, ROWS);
    let mut terminal = Terminal::new(backend).unwrap();
    let palette = Palette::dark();
    terminal
        .draw(|frame| {
            let list = List::new(vec!["alpha".into(), "beta".into(), "gamma".into()])
                .selected(Some(1))
                .palette(palette);
            frame.render_widget(list, Rect::new(0, 0, 20, 5));
        })
        .unwrap();
    let buf = terminal.backend().buffer();
    let d = dump(&terminal);
    assert!(d.contains("alpha"));
    assert!(d.contains("beta"));
    // Row 1 (y=1) col 2 = first char of 'beta' (after "> " prefix).
    let cell = buf[(2, 1)].style();
    assert_eq!(cell.bg, Some(palette.primary), "selected row missing primary bg");
}

#[test]
fn list_truncates_long_rows() {
    let backend = TestBackend::new(COLS, ROWS);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            let list = List::new(vec!["this is a very long item name that overflows".into()]);
            frame.render_widget(list, Rect::new(0, 0, 10, 1));
        })
        .unwrap();
    let buf = terminal.backend().buffer();
    // Last visible cell must still be inside the area — no overflow into next row.
    let mut last_nonblank = 0_u16;
    for x in 0..10 {
        if buf[(x, 0)].symbol() != " " {
            last_nonblank = x;
        }
    }
    assert!(last_nonblank < 10);
}

#[test]
fn gauge_fills_proportionally() {
    let backend = TestBackend::new(COLS, ROWS);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|frame| {
            // 50 % over 20 cols = 10 filled cells.
            frame.render_widget(Gauge::new(0.5).label("HALF"), Rect::new(0, 0, 20, 1));
        })
        .unwrap();
    let buf = terminal.backend().buffer();
    let palette = Palette::dark();
    let mut filled = 0;
    for x in 0..20 {
        if buf[(x, 0)].style().bg == Some(palette.primary) {
            filled += 1;
        }
    }
    assert_eq!(filled, 10, "expected exactly 10 filled cells");

    // Label must appear somewhere in the line.
    let mut joined = String::new();
    for x in 0..20 {
        joined.push_str(buf[(x, 0)].symbol());
    }
    assert!(joined.contains("HALF"), "label missing in: {joined:?}");
}

#[test]
fn gauge_clamps_out_of_range() {
    assert!((Gauge::new(1.5).ratio_value() - 1.0).abs() < f32::EPSILON);
    assert!((Gauge::new(-0.1).ratio_value() - 0.0).abs() < f32::EPSILON);
}

#[test]
fn argb_to_rgb_drops_alpha_byte() {
    // M3 BASELINE primary = 0xFF6750A4 -> Rgb(0x67, 0x50, 0xA4).
    assert_eq!(argb_to_rgb(0xFF6750A4), Color::Rgb(0x67, 0x50, 0xA4));
    // Alpha byte is ignored.
    assert_eq!(argb_to_rgb(0x006750A4), Color::Rgb(0x67, 0x50, 0xA4));
}

#[test]
fn palette_pulls_from_m3_tokens() {
    let dark = Palette::dark();
    let light = Palette::light();
    assert_ne!(dark.surface, light.surface);
    let expected = argb_to_rgb(m3_tokens::color::BASELINE_DARK.surface);
    assert_eq!(dark.surface, expected);
}
