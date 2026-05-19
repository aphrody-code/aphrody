// SPDX-License-Identifier: Apache-2.0
//! Spec-compliant smoke tests for PLAN.md §0.5 T-10 — short canonical names
//! (`dsl_render`, `layout_engine`, `input_event`) required by the YOLO grind
//! verify command:
//!
//! ```text
//! cargo nextest run -p aphrody-tui --offline \
//!     -E 'test(/dsl_render|layout_engine|input_event/)'
//! ```
//!
//! These tests are a fusion of three existing, production-grade surfaces:
//!
//! 1. [`aphrody_tui::widgets`] — `SubAgentPane`, `McpStatusBar` (concrete
//!    DSL widget builders rendered into a `ratatui` `Buffer`).
//! 2. [`aphrody_tui::layout`] — `Layout::split`, `vsplit`, `hsplit` (the
//!    constraint solver — `Length` / `Percentage` / `Min` / `Ratio`).
//! 3. [`aphrody_tui::TuiEvent`] / [`crossterm::event::KeyEvent`] — input
//!    taxonomy dispatched to `Update::update`.
//!
//! No new logic lives here; the tests exercise the same public APIs that the
//! `integration`, `layout_smoke`, and `widgets_smoke` suites cover, but under
//! the canonical short test names so the spec verify command matches exactly.

use aphrody_tui::{
    App, Render, TuiEvent, Update,
    layout::{Layout, hsplit, vsplit},
    widgets::{McpHealth, McpServerSlot, McpStatusBar, SubAgentPane, SubAgentRow, SubAgentStatus},
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::{
    Frame, Terminal,
    backend::TestBackend,
    layout::{Constraint, Direction, Rect},
};

// ---------------------------------------------------------------------------
// 1. dsl_render — build a `panel`-style widget (SubAgentPane) and assert the
//    title + child text + a border glyph land in the buffer.
// ---------------------------------------------------------------------------

#[test]
fn dsl_render() {
    let backend = TestBackend::new(40, 5);
    let mut terminal = Terminal::new(backend).expect("terminal");

    terminal
        .draw(|frame: &mut Frame<'_>| {
            let pane = SubAgentPane::new(vec![
                SubAgentRow {
                    name: "world".to_owned(),
                    status_line: "ready".to_owned(),
                    status: SubAgentStatus::Done,
                },
            ])
            .title("Hello");
            frame.render_widget(pane, frame.area());

            // Render a second widget on the same buffer to exercise the
            // status-bar DSL surface in the same draw call.
            let bar = McpStatusBar::new(vec![
                McpServerSlot { name: "bxc".to_owned(), health: McpHealth::Up },
            ]);
            let bar_area = Rect::new(0, 4, 40, 1);
            frame.render_widget(bar, bar_area);
        })
        .expect("draw");

    let buffer = terminal.backend().buffer();
    let dump: String = buffer.content().iter().map(ratatui::buffer::Cell::symbol).collect();

    assert!(dump.contains("world"), "child text missing in buffer:\n{dump}");
    assert!(dump.contains("Hello"), "panel title missing in buffer:\n{dump}");
    // Stock ratatui Block draws horizontal borders with the U+2500 box-drawing
    // glyph by default — proves the DSL render hit the buffer.
    assert!(
        dump.contains('\u{2500}'),
        "expected top-border box-drawing char (U+2500) in buffer:\n{dump}"
    );
    assert!(dump.contains("bxc"), "status-bar slot missing in buffer:\n{dump}");
}

// ---------------------------------------------------------------------------
// 2. layout_engine — split(Rect{0,0,100,10}, Horizontal, &[Length(20),
//    Percentage(30), Min(0)]) → 3 rects (20 + 30 + remainder=50). The spec
//    asked for `Fill(1)` semantics; `Constraint::Min(0)` is the equivalent
//    "absorb remainder" primitive in ratatui 0.30's solver and is what the
//    existing crate is already wired to use.
// ---------------------------------------------------------------------------

#[test]
fn layout_engine() {
    // --- Horizontal split: fixed + percentage + remainder ------------------
    let area = Rect::new(0, 0, 100, 10);
    let cols = hsplit(area, &[Constraint::Length(20), Constraint::Percentage(30), Constraint::Min(0)]);
    assert_eq!(cols.len(), 3);
    assert_eq!(cols[0].width, 20, "first col must honour fixed Length(20)");
    assert_eq!(cols[1].width, 30, "second col must be 30% of 100 = 30");
    assert_eq!(cols[2].width, 50, "third col absorbs remainder: 100 - 20 - 30 = 50");

    // Children cover the parent without overlap.
    assert_eq!(cols[0].x, 0);
    assert_eq!(cols[1].x, cols[0].x + cols[0].width);
    assert_eq!(cols[2].x, cols[1].x + cols[1].width);
    let total: u16 = cols.iter().map(|c| c.width).sum();
    assert_eq!(total, area.width);

    // --- Vertical split via typed Layout facade ----------------------------
    let v_area = Rect::new(0, 0, 80, 24);
    let rows = Layout::new(Direction::Vertical)
        .split(v_area, &[Constraint::Length(3), Constraint::Min(0), Constraint::Length(1)]);
    assert_eq!(rows.len(), 3);
    assert_eq!(rows[0].height, 3);
    assert_eq!(rows[2].height, 1);
    let v_total: u16 = rows.iter().map(|r| r.height).sum();
    assert_eq!(v_total, v_area.height, "vertical split must cover full parent height");

    // --- vsplit free function matches the typed Layout facade --------------
    let free = vsplit(v_area, &[Constraint::Length(3), Constraint::Min(0), Constraint::Length(1)]);
    assert_eq!(free, rows);
}

// ---------------------------------------------------------------------------
// 3. input_event — build a `KeyEvent` (the input taxonomy), dispatch it to
//    `Update::update` via `App`, assert it lands. Also covers Char + arrow-key
//    style inputs (Up) and Ctrl modifier propagation.
// ---------------------------------------------------------------------------

#[derive(Default)]
struct Probe {
    chars: Vec<char>,
    arrow_up: u32,
    ctrl_chars: u32,
    ticks: u32,
}

impl Render for Probe {
    fn render(&self, _frame: &mut Frame<'_>) {}
}

impl Update for Probe {
    type Event = TuiEvent;
    fn update(&mut self, event: TuiEvent) {
        match event {
            TuiEvent::Key(KeyEvent { code: KeyCode::Char(c), modifiers, .. }) => {
                if modifiers.contains(KeyModifiers::CONTROL) {
                    self.ctrl_chars += 1;
                } else {
                    self.chars.push(c);
                }
            }
            TuiEvent::Key(KeyEvent { code: KeyCode::Up, .. }) => self.arrow_up += 1,
            TuiEvent::Tick => self.ticks += 1,
            _ => {}
        }
    }
}

fn key(code: KeyCode, modifiers: KeyModifiers) -> TuiEvent {
    TuiEvent::Key(KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    })
}

#[test]
fn input_event() {
    let mut app = App::new(Probe::default());

    // Plain `Char('a')` — parses to the ASCII letter, no modifiers.
    app.state_mut().update(key(KeyCode::Char('a'), KeyModifiers::NONE));
    // Arrow Up — corresponds to CSI `\x1b[A` on the wire.
    app.state_mut().update(key(KeyCode::Up, KeyModifiers::NONE));
    // Ctrl-C style: Char('C') with CONTROL modifier — corresponds to CSI
    // `\x1b[1;5C` on the wire for arrow-right-with-ctrl; we use a Char here
    // to keep the probe simple while still asserting the modifier survives.
    app.state_mut().update(key(KeyCode::Char('C'), KeyModifiers::CONTROL));
    // A tick must also dispatch — proves the broader event taxonomy is wired.
    app.state_mut().update(TuiEvent::Tick);

    assert_eq!(app.state().chars, vec!['a'], "expected single Char('a') dispatch");
    assert_eq!(app.state().arrow_up, 1, "expected single Up arrow dispatch");
    assert_eq!(app.state().ctrl_chars, 1, "expected single Ctrl+char dispatch");
    assert_eq!(app.state().ticks, 1, "expected single Tick dispatch");
    assert!(!app.quit(), "no interrupt requested => app must not be quitting");
}
