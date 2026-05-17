// SPDX-License-Identifier: Apache-2.0
//! Demo app showcasing aphrody-tui widgets at 60fps.
//!
//! Lays out three panes in a single column:
//!
//! - top: [`MarkdownView`] rendered via `aphrody-terminal-markdown`
//!   (CommonMark -> ANSI -> ratatui `Line`s), with a frame counter
//!   interpolated into the body so the live render path is visible;
//! - middle: [`SubAgentPane`] with one row per lifecycle state
//!   (`Running` / `Done` / `Failed` / `Pending`) — the running row's
//!   ASCII spinner advances every 8 ticks (~128 ms);
//! - bottom: [`McpStatusBar`] with three MCP slots whose health rotates
//!   (`Up` -> `Down` -> `Unknown` -> `Up` ...) every 60 ticks (~1 s)
//!   so the colour wiring is observable without external state.
//!
//! Runs the canonical 60 fps loop ([`run`]) and quits on `q`, `Esc`, or
//! `Ctrl-C`. The demo is intentionally self-contained: no MCP probes,
//! no real sub-agents — every datum is synthesised on tick so the demo
//! compiles and runs identically on Linux, Windows, and inside CI.

#![forbid(unsafe_code)]

use std::error::Error;

use aphrody_tui::layout::vsplit;
use aphrody_tui::widgets::{
    McpHealth, McpServerSlot, McpStatusBar, MarkdownView, SubAgentPane, SubAgentRow,
    SubAgentStatus,
};
use aphrody_tui::{App, CrosstermKeyCode, Render, TuiEvent, Update, run};
use crossterm::execute;
use crossterm::terminal::{LeaveAlternateScreen, disable_raw_mode};
use ratatui::Frame;
use ratatui::layout::Constraint;

/// Tick cadence (in [`aphrody_tui::TICK_MS`] units) between spinner phase
/// advances. 8 ticks at 16 ms = ~128 ms per spinner frame, slow enough to
/// read on a 60 Hz terminal yet fast enough to feel alive.
const SPINNER_PERIOD_TICKS: u64 = 8;

/// Tick cadence between MCP health rotations. 60 ticks = ~1 s.
const MCP_ROTATE_PERIOD_TICKS: u64 = 60;

/// Application state.
struct DemoApp {
    /// Monotonic tick counter. Wraps at `u64::MAX` (i.e. never, in
    /// practice — the loop would have to run for ~9.7 trillion years).
    frame: u64,
}

impl DemoApp {
    fn new() -> Self {
        Self { frame: 0 }
    }

    /// Build the live [`SubAgentPane`] for the current frame.
    fn sub_agent_pane(&self) -> SubAgentPane {
        let spinner = (self.frame / SPINNER_PERIOD_TICKS) as u8;
        let rows = vec![
            SubAgentRow {
                name: "yolo-prod-ready".to_owned(),
                status_line: format!("rendering demo (frame {})", self.frame),
                status: SubAgentStatus::Running,
            },
            SubAgentRow {
                name: "cargo-auditor".to_owned(),
                status_line: "clippy -D warnings: clean".to_owned(),
                status: SubAgentStatus::Done,
            },
            SubAgentRow {
                name: "ievr-verify".to_owned(),
                status_line: "gate 3/5 — WebGPU adapter pending".to_owned(),
                status: SubAgentStatus::Failed,
            },
            SubAgentRow {
                name: "skill-sync".to_owned(),
                status_line: "queued: vercel-labs/skills".to_owned(),
                status: SubAgentStatus::Pending,
            },
        ];
        SubAgentPane::new(rows)
            .spinner_frame(spinner)
            .title("sub-agents")
    }

    /// Build the live [`McpStatusBar`] for the current frame.
    ///
    /// Rotates one slot's health per period so the colour wiring is
    /// observable without any external MCP probe.
    fn mcp_status_bar(&self) -> McpStatusBar {
        let phase = (self.frame / MCP_ROTATE_PERIOD_TICKS) % 3;
        let cycle = [McpHealth::Up, McpHealth::Down, McpHealth::Unknown];
        let slots = vec![
            McpServerSlot {
                name: "context7".to_owned(),
                health: cycle[phase as usize],
            },
            McpServerSlot {
                name: "bxc".to_owned(),
                health: cycle[((phase + 1) % 3) as usize],
            },
            McpServerSlot {
                name: "google-mcp".to_owned(),
                health: cycle[((phase + 2) % 3) as usize],
            },
        ];
        McpStatusBar::new(slots)
    }

    /// Build the live [`MarkdownView`] for the current frame.
    fn markdown_view(&self) -> MarkdownView {
        let body = format!(
            "# aphrody-tui demo\n\
             \n\
             Live **CommonMark** rendered via `aphrody-terminal-markdown` -> ANSI -> ratatui.\n\
             \n\
             - frame: `{frame}`\n\
             - tick: 16 ms (~60 fps)\n\
             - press `q`, `Esc`, or `Ctrl-C` to quit\n\
             \n\
             ```rust\n\
             fn main() {{\n    println!(\"hello, aphrody\");\n}}\n\
             ```\n",
            frame = self.frame,
        );
        MarkdownView::new(body).block_title("markdown")
    }
}

impl Render for DemoApp {
    fn render(&self, frame: &mut Frame<'_>) {
        // 3 stacked panes:
        //   - markdown: flexible top (Min(7) — at least heading + body + fence)
        //   - sub-agents: fixed 6 rows (4 rows + 2 border lines)
        //   - mcp bar: fixed 1 row (no border, single line)
        let area = frame.area();
        let chunks = vsplit(
            area,
            &[Constraint::Min(7), Constraint::Length(6), Constraint::Length(1)],
        );

        frame.render_widget(self.markdown_view(), chunks[0]);
        frame.render_widget(self.sub_agent_pane(), chunks[1]);
        frame.render_widget(self.mcp_status_bar(), chunks[2]);
    }
}

impl Update for DemoApp {
    type Event = TuiEvent;

    fn update(&mut self, event: TuiEvent) {
        match event {
            TuiEvent::Tick => {
                self.frame = self.frame.wrapping_add(1);
            }
            TuiEvent::Key(key) => {
                if matches!(
                    key.code,
                    CrosstermKeyCode::Char('q') | CrosstermKeyCode::Esc
                ) {
                    // The `App::request_quit` setter lives on the outer
                    // wrapper, which `Update::update` doesn't own. Restore
                    // the terminal manually (mirrors `TerminalGuard::drop`
                    // inside `aphrody_tui::run`) and exit cleanly so the
                    // shell isn't left in raw mode + alt screen.
                    let _ = disable_raw_mode();
                    let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
                    std::process::exit(0);
                }
            }
            TuiEvent::Mouse(_) | TuiEvent::Resize(_, _) => {
                // No-op: layout is fully constraint-driven, mouse
                // routing is out of scope for the demo.
            }
        }
    }
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    run(App::new(DemoApp::new())).await?;
    Ok(())
}
