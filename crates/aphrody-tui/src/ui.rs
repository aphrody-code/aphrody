// SPDX-License-Identifier: Apache-2.0
//! Ratatui rendering of [`AppState`].
//!
//! The draw path is a pure function of the state: it never mutates `AppState`
//! and performs no I/O, so it can be exercised against a `TestBackend`.

use ratatui::Frame;
use ratatui::layout::Constraint;
use ratatui::layout::Layout;
use ratatui::style::Color;
use ratatui::style::Modifier;
use ratatui::style::Style;
use ratatui::text::Line;
use ratatui::text::Span;
use ratatui::text::Text;
use ratatui::widgets::Block;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Wrap;

use crate::state::AppState;
use crate::state::PendingApproval;
use crate::state::TranscriptCell;

/// The animation frames for the in-progress turn spinner (ASCII only).
const SPINNER: [&str; 4] = ["-", "\\", "|", "/"];

/// Renders the whole agent surface into `frame`.
///
/// Layout (top to bottom): a scrollable transcript fills the available space,
/// a one-line status bar shows model / tokens / turn state, and a bordered
/// input box (or an approval prompt) sits at the bottom.
///
/// `tick` advances the spinner; callers pass a monotonically increasing counter
/// (e.g. a redraw tick) so the spinner animates without state mutation.
pub fn draw(frame: &mut Frame, state: &AppState, tick: u64) {
    let layout = Layout::vertical([
        Constraint::Fill(1),    // transcript
        Constraint::Length(1),  // status bar
        Constraint::Length(3),  // input box
    ]);
    let [transcript_area, status_area, input_area] = frame.area().layout(&layout);

    draw_transcript(frame, state, transcript_area);
    draw_status(frame, state, status_area, tick);
    draw_input(frame, state, input_area);
}

/// Builds the transcript paragraph (wrapped, scrollable) and renders it.
fn draw_transcript(frame: &mut Frame, state: &AppState, area: ratatui::layout::Rect) {
    let mut lines: Vec<Line> = Vec::new();
    for cell in state.transcript() {
        push_cell_lines(&mut lines, cell);
    }
    // Stream the in-flight agent message below the finalized transcript so the
    // user sees text as it arrives.
    if !state.agent_buffer().is_empty() {
        lines.push(Line::from(vec![
            Span::styled("agent ", Style::default().fg(Color::Cyan)),
            Span::raw(state.agent_buffer().to_string()),
        ]));
    }

    let total = u16::try_from(lines.len()).unwrap_or(u16::MAX);
    let viewport = area.height.saturating_sub(2); // account for the block border
    let max_top = total.saturating_sub(viewport);
    // scroll_offset counts lines up from the bottom; translate to a top offset.
    let top = max_top.saturating_sub(state.scroll_offset());

    let block = Block::bordered().title(Line::from("Conversation").left_aligned());
    let para = Paragraph::new(Text::from(lines))
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((top, 0));
    frame.render_widget(para, area);
}

/// Appends the rendered lines for a single transcript cell.
fn push_cell_lines(lines: &mut Vec<Line<'static>>, cell: &TranscriptCell) {
    match cell {
        TranscriptCell::UserMessage(text) => {
            lines.push(labeled("you", Color::Green, text));
        }
        TranscriptCell::AgentMessage(text) => {
            lines.push(labeled("agent", Color::Cyan, text));
        }
        TranscriptCell::Reasoning(text) => {
            lines.push(labeled("think", Color::Magenta, text));
        }
        TranscriptCell::ToolCall {
            name,
            done,
            call_id: _,
        } => {
            let status = match done {
                None => "running",
                Some(true) => "ok",
                Some(false) => "failed",
            };
            lines.push(Line::from(vec![
                Span::styled("tool  ", Style::default().fg(Color::Yellow)),
                Span::raw(format!("{name} [{status}]")),
            ]));
        }
        TranscriptCell::Exec {
            command,
            output,
            exit_code,
            call_id: _,
        } => {
            let header = match exit_code {
                None => format!("$ {} (running)", command.join(" ")),
                Some(code) => format!("$ {} (exit {code})", command.join(" ")),
            };
            lines.push(Line::from(vec![
                Span::styled("exec  ", Style::default().fg(Color::Blue)),
                Span::styled(header, Style::default().add_modifier(Modifier::DIM)),
            ]));
            for out_line in output.lines() {
                lines.push(Line::from(Span::styled(
                    format!("      {out_line}"),
                    Style::default().add_modifier(Modifier::DIM),
                )));
            }
        }
        TranscriptCell::ErrorCell(text) => {
            lines.push(labeled("error", Color::Red, text));
        }
        TranscriptCell::Notice(text) => {
            lines.push(Line::from(Span::styled(
                text.clone(),
                Style::default().add_modifier(Modifier::DIM),
            )));
        }
    }
}

/// Builds a `label: text` line with a colored label.
fn labeled(label: &str, color: Color, text: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{label} "),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
        Span::raw(text.to_string()),
    ])
}

/// Renders the one-line status bar.
fn draw_status(frame: &mut Frame, state: &AppState, area: ratatui::layout::Rect, tick: u64) {
    let mut spans: Vec<Span> = Vec::new();
    if state.turn_active() {
        let frame_ch = SPINNER[(tick as usize) % SPINNER.len()];
        spans.push(Span::styled(
            format!("{frame_ch} working "),
            Style::default().fg(Color::Yellow),
        ));
    } else {
        spans.push(Span::styled("idle ", Style::default().fg(Color::Green)));
    }
    spans.push(Span::raw("| model: aphrody-agent "));
    if let Some((input, output, total)) = state.token_count() {
        spans.push(Span::raw(format!(
            "| tokens in {input} out {output} total {total} "
        )));
    }
    spans.push(Span::styled(
        "| Enter send  Ctrl-C interrupt  Esc quit",
        Style::default().add_modifier(Modifier::DIM),
    ));
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// Renders the bottom input box, or the approval prompt if one is pending.
fn draw_input(frame: &mut Frame, state: &AppState, area: ratatui::layout::Rect) {
    if let Some(pending) = state.pending_approval() {
        let (title, body) = match pending {
            PendingApproval::Exec {
                command, reason, ..
            } => {
                let reason = reason.as_deref().unwrap_or("approval required");
                (
                    "Approve command? (y)es (n)o (a)bort",
                    format!("$ {} -- {reason}", command.join(" ")),
                )
            }
            PendingApproval::Patch { files, .. } => (
                "Approve patch? (y)es (n)o (a)bort",
                format!("files: {}", files.join(", ")),
            ),
        };
        let para = Paragraph::new(body)
            .wrap(Wrap { trim: false })
            .block(
                Block::bordered()
                    .title(Line::from(title).left_aligned())
                    .border_style(Style::default().fg(Color::Yellow)),
            );
        frame.render_widget(para, area);
        return;
    }

    let para = Paragraph::new(state.input())
        .block(Block::bordered().title(Line::from("Message").left_aligned()));
    frame.render_widget(para, area);

    // Place the terminal cursor inside the input box.
    let cursor_x = area.x + 1 + u16::try_from(state.cursor()).unwrap_or(u16::MAX);
    let cursor_y = area.y + 1;
    frame.set_cursor_position((cursor_x, cursor_y));
}

#[cfg(test)]
mod tests {
    use super::*;
    use aphrody_agent_proto::EventMsg;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    /// Flattens a `TestBackend` buffer into a single searchable string.
    fn buffer_text(terminal: &Terminal<TestBackend>) -> String {
        terminal
            .backend()
            .buffer()
            .content()
            .iter()
            .map(ratatui::buffer::Cell::symbol)
            .collect()
    }

    #[test]
    fn draw_renders_without_panic_and_shows_content() {
        let mut state = AppState::new();
        state.apply_event(EventMsg::TurnStarted);
        state.apply_event(EventMsg::AgentMessage {
            text: "answer".into(),
        });
        state.apply_event(EventMsg::TokenCount {
            input: 1,
            output: 2,
            total: 3,
        });
        state.apply_event(EventMsg::TurnComplete {
            last_agent_message: Some("answer".into()),
        });

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| draw(frame, &state, 0))
            .expect("draw");

        let text = buffer_text(&terminal);
        assert!(text.contains("answer"), "transcript text rendered: {text}");
        assert!(text.contains("Conversation"), "transcript block title");
        assert!(text.contains("Message"), "input block title");
        assert!(text.contains("tokens"), "status bar tokens");
    }

    #[test]
    fn draw_renders_approval_prompt() {
        let mut state = AppState::new();
        state.apply_event(EventMsg::ExecApprovalRequest {
            id: "r".into(),
            command: vec!["rm".into(), "file".into()],
            reason: Some("danger".into()),
        });
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| draw(frame, &state, 0))
            .expect("draw");
        let text = buffer_text(&terminal);
        assert!(text.contains("Approve command"), "approval prompt: {text}");
    }

    #[test]
    fn draw_handles_tiny_area() {
        // A 1x1 terminal must not panic the renderer.
        let state = AppState::new();
        let backend = TestBackend::new(1, 1);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| draw(frame, &state, 7))
            .expect("draw tiny");
    }
}
