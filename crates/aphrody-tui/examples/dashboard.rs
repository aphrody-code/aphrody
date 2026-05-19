// SPDX-License-Identifier: Apache-2.0
//! 2x2 dashboard composed of the four production widgets:
//!
//!     +-----------------+-----------------+
//!     |  Block + Para   |     List        |
//!     +-----------------+-----------------+
//!     |      Gauge      |   MarkdownView  |
//!     +-----------------+-----------------+
//!
//! Designed to run headless (no real terminal needed): renders one frame
//! into ratatui's [`TestBackend`] sized 80x24, then dumps the buffer to
//! stderr so CI / smoke harnesses can capture the output. This sidesteps
//! `enable_raw_mode` so the example runs the same way inside `cargo run`,
//! `cargo test`, and Windows headless boxes.

#![forbid(unsafe_code)]

use aphrody_tui::{
    TestBackend,
    layout::{Layout, RatatuiConstraint},
    m3::{Block, BorderStyle, Gauge, List, Padding, Palette, Paragraph, WrapMode},
    widgets::MarkdownView,
};
use ratatui::{Terminal, layout::Direction};

const COLS: u16 = 80;
const ROWS: u16 = 24;

fn main() -> std::io::Result<()> {
    let backend = TestBackend::new(COLS, ROWS);
    let mut terminal = Terminal::new(backend).expect("test backend");

    terminal
        .draw(|frame| {
            let area = frame.area();
            let palette = Palette::dark();

            // 2x2 grid.
            let rows = Layout::new(Direction::Vertical).split(
                area,
                &[RatatuiConstraint::Percentage(50), RatatuiConstraint::Percentage(50)],
            );
            let top = Layout::new(Direction::Horizontal).split(
                rows[0],
                &[RatatuiConstraint::Percentage(50), RatatuiConstraint::Percentage(50)],
            );
            let bot = Layout::new(Direction::Horizontal).split(
                rows[1],
                &[RatatuiConstraint::Percentage(50), RatatuiConstraint::Percentage(50)],
            );

            // ── Top-left: bordered Block + Paragraph inside ─────────────
            let block_tl = Block::new()
                .title("aphrody-tui")
                .border_style(BorderStyle::Rounded)
                .padding(Padding::uniform(1))
                .palette(palette)
                .focused(true);
            let inner_tl = block_tl.inner(top[0]);
            frame.render_widget(block_tl, top[0]);
            let body = "Pure-Rust ratatui-style DSL. The four production widgets — Block, \
                        Paragraph, List, Gauge — all pull their colours from m3-tokens via \
                        Palette::dark(), so every aphrody surface looks like one product.";
            frame.render_widget(
                Paragraph::new(body).wrap(WrapMode::Word).palette(palette),
                inner_tl,
            );

            // ── Top-right: selectable List inside a Block ──────────────
            let block_tr =
                Block::new().title("sub-agents").border_style(BorderStyle::Plain).palette(palette);
            let inner_tr = block_tr.inner(top[0]).clamp(top[1]);
            frame.render_widget(block_tr, top[1]);
            let items: Vec<String> = vec![
                "yolo-prod-ready".into(),
                "rust-engineer".into(),
                "cargo-auditor".into(),
                "ffi-architect".into(),
                "skill-vetter".into(),
            ];
            let list = List::new(items).selected(Some(1)).palette(palette);
            frame.render_widget(list, inner_tr);

            // ── Bottom-left: Gauge inside a Block ──────────────────────
            let block_bl = Block::new()
                .title("build progress")
                .border_style(BorderStyle::Double)
                .palette(palette);
            let inner_bl = block_bl.inner(top[0]).clamp(bot[0]);
            frame.render_widget(block_bl, bot[0]);
            frame.render_widget(Gauge::new(0.62).label("62% (T-2 VT)").palette(palette), inner_bl);

            // ── Bottom-right: MarkdownView (existing widget, reused) ──
            let md = "# verify\n\nDashboard composed via `aphrody_tui::layout::Layout::split` \
                      + `m3::{Block,Paragraph,List,Gauge}`.\n\n```text\nrendered: 80x24\n```";
            let block_br =
                Block::new().title("markdown").border_style(BorderStyle::Thick).palette(palette);
            let inner_br = block_br.inner(top[0]).clamp(bot[1]);
            frame.render_widget(block_br, bot[1]);
            frame.render_widget(MarkdownView::new(md), inner_br);
        })
        .expect("draw");

    // Dump the rendered buffer to stderr — observable, scriptable, no TTY required.
    let buf = terminal.backend().buffer();
    eprintln!("=== aphrody-tui dashboard render ({COLS}x{ROWS}) ===");
    for y in 0..buf.area.height {
        let mut line = String::with_capacity(buf.area.width as usize);
        for x in 0..buf.area.width {
            line.push_str(buf[(x, y)].symbol());
        }
        eprintln!("{line}");
    }
    eprintln!("=== /dashboard ===");
    println!("ok: rendered {COLS}x{ROWS} dashboard with 4 widgets via Layout::split 2x2 grid");
    Ok(())
}
