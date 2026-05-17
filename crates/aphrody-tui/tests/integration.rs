// SPDX-License-Identifier: Apache-2.0
//! Integration tests for the four PLAN.md T-10 contracts:
//!   1. `dsl_render_renders_widget_to_buffer`
//!   2. `layout_engine_splits_correctly`
//!   3. `input_event_dispatched_to_update`
//!   4. `render_loop_60fps_tick_interval`

use aphrody_tui::layout::{hsplit, vsplit};
use aphrody_tui::widgets::{
    McpHealth, McpServerSlot, McpStatusBar, SubAgentPane, SubAgentRow, SubAgentStatus,
};
use aphrody_tui::{App, Render, TICK_MS, TestBackend, TuiEvent, Update, build_tick_interval};

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::Frame;
use ratatui::Terminal;
use ratatui::layout::{Constraint, Rect};

// ---------------------------------------------------------------------------
// 1. dsl_render_renders_widget_to_buffer
// ---------------------------------------------------------------------------

#[test]
fn dsl_render_renders_widget_to_buffer() {
    let backend = TestBackend::new(40, 6);
    let mut terminal = Terminal::new(backend).expect("terminal");

    terminal
        .draw(|frame: &mut Frame<'_>| {
            let pane = SubAgentPane::new(vec![
                SubAgentRow {
                    name: "yolo-prod-ready".to_owned(),
                    status_line: "verifying".to_owned(),
                    status: SubAgentStatus::Running,
                },
                SubAgentRow {
                    name: "release-please".to_owned(),
                    status_line: "ready".to_owned(),
                    status: SubAgentStatus::Done,
                },
            ])
            .title("agents");
            frame.render_widget(pane, frame.area());
        })
        .expect("draw");

    let buffer = terminal.backend().buffer();
    // Render the buffer to a single string and assert key tokens are present.
    let dump: String = buffer
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect();

    assert!(dump.contains("yolo-prod-ready"), "missing agent name in: {dump:?}");
    assert!(dump.contains("release-please"), "missing second agent in: {dump:?}");
    assert!(dump.contains("agents"), "missing block title in: {dump:?}");
}

// ---------------------------------------------------------------------------
// 2. layout_engine_splits_correctly
// ---------------------------------------------------------------------------

#[test]
fn layout_engine_splits_correctly() {
    let area = Rect::new(0, 0, 80, 24);
    let rows = vsplit(
        area,
        &[
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ],
    );

    assert_eq!(rows.len(), 3, "vsplit must yield three rects");
    // Geometry checks.
    assert_eq!(rows[0].y, 0);
    assert_eq!(rows[0].height, 3);
    assert_eq!(rows[2].y, 23);
    assert_eq!(rows[2].height, 1);
    // Sum invariant: stacked heights cover the entire area.
    let total: u16 = rows.iter().map(|r| r.height).sum();
    assert_eq!(total, area.height);
    // Widths are preserved by a vertical split.
    for r in &rows {
        assert_eq!(r.width, area.width);
    }

    // Horizontal split sanity.
    let cols = hsplit(
        area,
        &[Constraint::Percentage(25), Constraint::Percentage(75)],
    );
    assert_eq!(cols.len(), 2);
    assert_eq!(cols[0].width + cols[1].width, area.width);
}

// ---------------------------------------------------------------------------
// 3. input_event_dispatched_to_update
// ---------------------------------------------------------------------------

/// Minimal app that counts how many [`TuiEvent::Key`] events it has seen.
#[derive(Default)]
struct Counter {
    key_events: u32,
    tick_events: u32,
    last_resize: Option<(u16, u16)>,
}

impl Render for Counter {
    fn render(&self, _frame: &mut Frame<'_>) {}
}

impl Update for Counter {
    type Event = TuiEvent;
    fn update(&mut self, event: TuiEvent) {
        match event {
            TuiEvent::Key(_) => self.key_events += 1,
            TuiEvent::Tick => self.tick_events += 1,
            TuiEvent::Resize(w, h) => self.last_resize = Some((w, h)),
            TuiEvent::Mouse(_) => {}
        }
    }
}

#[test]
fn input_event_dispatched_to_update() {
    let mut app = App::new(Counter::default());
    let press = |c: char| {
        TuiEvent::Key(KeyEvent {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        })
    };

    app.state_mut().update(press('a'));
    app.state_mut().update(press('b'));
    app.state_mut().update(press('c'));
    app.state_mut().update(TuiEvent::Tick);
    app.state_mut().update(TuiEvent::Resize(120, 40));

    assert_eq!(app.state().key_events, 3, "expected three key events");
    assert_eq!(app.state().tick_events, 1, "tick must dispatch as well");
    assert_eq!(app.state().last_resize, Some((120, 40)));
    assert!(!app.quit(), "no Ctrl-C => no quit");
}

// ---------------------------------------------------------------------------
// 4. render_loop_60fps_tick_interval
// ---------------------------------------------------------------------------

#[test]
fn render_loop_60fps_tick_interval() {
    // Arithmetic contract: 16 ms cadence => >= 60 ticks per second.
    assert_eq!(TICK_MS, 16, "60fps target requires 16ms ticks");
    let ticks_per_second = 1000 / TICK_MS;
    assert!(
        ticks_per_second >= 60,
        "{ticks_per_second} ticks/s does not meet the 60fps contract"
    );

    // Constructor sanity — interval must build without panicking inside a
    // tokio runtime context.  `tokio::test` provides one automatically.
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .expect("runtime");
    rt.block_on(async {
        let interval = build_tick_interval();
        assert_eq!(interval.period().as_millis(), u128::from(TICK_MS));
    });
}

// ---------------------------------------------------------------------------
// Bonus: McpStatusBar smoke test — keeps coverage on the widget pathway.
// ---------------------------------------------------------------------------

#[test]
fn mcp_status_bar_renders_dots_and_names() {
    let backend = TestBackend::new(40, 1);
    let mut terminal = Terminal::new(backend).expect("terminal");

    terminal
        .draw(|frame: &mut Frame<'_>| {
            let bar = McpStatusBar::new(vec![
                McpServerSlot {
                    name: "context7".to_owned(),
                    health: McpHealth::Up,
                },
                McpServerSlot {
                    name: "bxc".to_owned(),
                    health: McpHealth::Down,
                },
            ]);
            frame.render_widget(bar, frame.area());
        })
        .expect("draw");

    let dump: String = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(ratatui::buffer::Cell::symbol)
        .collect();
    assert!(dump.contains("context7"));
    assert!(dump.contains("bxc"));
}
