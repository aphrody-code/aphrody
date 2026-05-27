// SPDX-License-Identifier: Apache-2.0
//! The async event loop wiring the protocol channels to the terminal.
//!
//! This is the only module that owns a real terminal. It is deliberately thin:
//! all decision-making lives in [`AppState`], which is pure and tested without
//! a terminal. The loop multiplexes three sources with `tokio::select!`:
//!
//! 1. incoming [`Event`]s from the engine,
//! 2. terminal key events (via crossterm's `EventStream`),
//! 3. a periodic redraw tick that animates the working spinner.

use std::time::Duration;

use aphrody_agent_proto::Event;
use aphrody_agent_proto::Submission;
use crossterm::event::Event as CtEvent;
use crossterm::event::EventStream;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use ratatui::DefaultTerminal;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use tokio_stream::StreamExt;

use crate::state::Action;
use crate::state::AppState;
use crate::state::KeyPress;
use crate::ui;

/// Redraw frequency for spinner animation. The transcript and input redraw on
/// demand too, so this only governs the spinner cadence.
const REDRAW_HZ: f32 = 12.0;

/// A terminal restore guard: [`ratatui::restore`] runs on drop, so the terminal
/// is brought back to a sane state even if the loop returns early or a `?`
/// propagates an error.
struct RestoreGuard;

impl Drop for RestoreGuard {
    fn drop(&mut self) {
        ratatui::restore();
    }
}

/// Runs the full-screen agent TUI until the user quits, the engine closes the
/// event channel, or a fatal I/O error occurs.
///
/// `events` is the stream of [`Event`]s the engine emits; `submissions` is the
/// sink the TUI writes [`Submission`]s to. The TUI never blocks the engine: it
/// drains events as fast as it can and forwards user actions immediately.
///
/// # Errors
/// Returns the first terminal draw / I/O error encountered. The terminal is
/// always restored (raw mode off, alternate screen left) before returning,
/// including on the error path, via a drop guard.
pub async fn run(
    mut events: UnboundedReceiver<Event>,
    submissions: UnboundedSender<Submission>,
) -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    let _guard = RestoreGuard;
    let result = event_loop(&mut terminal, &mut events, &submissions).await;
    // `_guard` restores the terminal on drop, before `result` is returned.
    result
}

/// The inner loop, split out so the [`RestoreGuard`] in [`run`] wraps every
/// early return path.
async fn event_loop(
    terminal: &mut DefaultTerminal,
    events: &mut UnboundedReceiver<Event>,
    submissions: &UnboundedSender<Submission>,
) -> std::io::Result<()> {
    let mut state = AppState::new();
    let mut key_stream = EventStream::new();
    let period = Duration::from_secs_f32(1.0 / REDRAW_HZ);
    let mut ticker = tokio::time::interval(period);
    let mut tick: u64 = 0;

    terminal.draw(|frame| ui::draw(frame, &state, tick))?;

    loop {
        tokio::select! {
            // Engine events. `None` means the engine hung up: exit cleanly.
            maybe_event = events.recv() => {
                match maybe_event {
                    Some(event) => state.apply_event(event.msg),
                    None => return Ok(()),
                }
                terminal.draw(|frame| ui::draw(frame, &state, tick))?;
            }

            // Terminal input.
            maybe_term = key_stream.next() => {
                match maybe_term {
                    Some(Ok(CtEvent::Key(key))) => {
                        if let Some(press) = translate_key(&key) {
                            match state.on_key(press) {
                                Action::Quit => {
                                    // Best-effort: ask the engine to shut down,
                                    // ignore the error if it already hung up.
                                    let _ = submissions.send(
                                        Submission::new(aphrody_agent_proto::Op::Shutdown),
                                    );
                                    return Ok(());
                                }
                                Action::Submit(sub) => {
                                    if submissions.send(sub).is_err() {
                                        // Engine is gone; nothing more to do.
                                        return Ok(());
                                    }
                                }
                                Action::None => {}
                            }
                        }
                        terminal.draw(|frame| ui::draw(frame, &state, tick))?;
                    }
                    // Resize / focus / paste / mouse: redraw to reflow.
                    Some(Ok(_)) => {
                        terminal.draw(|frame| ui::draw(frame, &state, tick))?;
                    }
                    // A read error on the terminal is fatal for the UI.
                    Some(Err(err)) => return Err(err),
                    // The terminal input stream ended.
                    None => return Ok(()),
                }
            }

            // Spinner / periodic redraw.
            _ = ticker.tick() => {
                tick = tick.wrapping_add(1);
                if state.turn_active() {
                    terminal.draw(|frame| ui::draw(frame, &state, tick))?;
                }
            }
        }
    }
}

/// Maps a crossterm [`KeyEvent`] into the backend-agnostic [`KeyPress`] used by
/// the pure state machine. Returns `None` for keys the UI does not act on (and
/// for non-press kinds, so a key is not handled twice on press+release).
fn translate_key(key: &KeyEvent) -> Option<KeyPress> {
    // Ignore key releases / repeats on backends that report them, except we do
    // want repeats for editing keys. crossterm reports `Press` on terminals
    // without the kitty protocol, so treat `Repeat` like `Press` and drop
    // `Release` only.
    if key.kind == KeyEventKind::Release {
        return None;
    }

    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    match key.code {
        KeyCode::Char('c') if ctrl => Some(KeyPress::CtrlC),
        // Ctrl + other letters are not bound; swallow them so they do not get
        // inserted as literal text.
        KeyCode::Char(_) if ctrl => None,
        KeyCode::Char(c) => Some(KeyPress::Char(c)),
        KeyCode::Enter => Some(KeyPress::Enter),
        KeyCode::Backspace => Some(KeyPress::Backspace),
        KeyCode::Esc => Some(KeyPress::Esc),
        KeyCode::Left => Some(KeyPress::Left),
        KeyCode::Right => Some(KeyPress::Right),
        KeyCode::Home => Some(KeyPress::Home),
        KeyCode::End => Some(KeyPress::End),
        KeyCode::PageUp => Some(KeyPress::PageUp),
        KeyCode::PageDown => Some(KeyPress::PageDown),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, mods)
    }

    #[test]
    fn translate_maps_printable_and_control() {
        assert_eq!(
            translate_key(&key(KeyCode::Char('a'), KeyModifiers::NONE)),
            Some(KeyPress::Char('a'))
        );
        assert_eq!(
            translate_key(&key(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(KeyPress::CtrlC)
        );
        // Ctrl+other is swallowed, not inserted.
        assert_eq!(
            translate_key(&key(KeyCode::Char('x'), KeyModifiers::CONTROL)),
            None
        );
    }

    #[test]
    fn translate_maps_editing_keys() {
        assert_eq!(
            translate_key(&key(KeyCode::Enter, KeyModifiers::NONE)),
            Some(KeyPress::Enter)
        );
        assert_eq!(
            translate_key(&key(KeyCode::Backspace, KeyModifiers::NONE)),
            Some(KeyPress::Backspace)
        );
        assert_eq!(
            translate_key(&key(KeyCode::Esc, KeyModifiers::NONE)),
            Some(KeyPress::Esc)
        );
    }

    #[test]
    fn translate_ignores_key_release() {
        let mut ev = key(KeyCode::Char('a'), KeyModifiers::NONE);
        ev.kind = KeyEventKind::Release;
        assert_eq!(translate_key(&ev), None);
    }

    #[test]
    fn translate_ignores_unbound_keys() {
        assert_eq!(translate_key(&key(KeyCode::Tab, KeyModifiers::NONE)), None);
    }
}
