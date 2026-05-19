// SPDX-License-Identifier: Apache-2.0
//! `aphrody-tui` — pure-Rust ratatui-style DSL for the aphrody LLM-first terminal.
//!
//! This crate is the *canonical long-term* native TUI surface for aphrody.
//! It sits next to the WASM renderer (`aphrody-terminal-wasm`) and reuses the
//! shared primitives stack (`aphrody-terminal-vt`, `aphrody-terminal-llm`,
//! `aphrody-terminal-markdown`).
//!
//! # Why ratatui (not bespoke)
//!
//! `ratatui` already encodes a Buffer/Renderer/Widget contract that matches
//! the aphrody design (cf. `docs/design/aphrody-terminal-spec.md`).  Building
//! on top of it lets us focus on:
//!
//! - **DSL ergonomics** — concise layout helpers ([`layout`]) and a small set of aphrody-specific
//!   widgets ([`widgets`]).
//! - **Async event loop** — a 60 fps tokio loop ([`run`]) that interleaves crossterm input with
//!   periodic ticks, restoring the terminal on panic.
//! - **Composable state** — generic [`App`] driven by the [`Render`] + [`Update`] traits, so
//!   callers ship a single type.
//!
//! # Quickstart
//!
//! ```no_run
//! use aphrody_tui::{App, Render, TuiEvent, Update, run};
//! use ratatui::Frame;
//!
//! #[derive(Default)]
//! struct Counter {
//!     n: u32,
//! }
//!
//! impl Render for Counter {
//!     fn render(&self, frame: &mut Frame<'_>) {
//!         frame.render_widget(format!("count = {}", self.n), frame.area());
//!     }
//! }
//!
//! impl Update for Counter {
//!     type Event = TuiEvent;
//!     fn update(&mut self, event: TuiEvent) {
//!         if matches!(event, TuiEvent::Tick) {
//!             self.n += 1;
//!         }
//!     }
//! }
//!
//! # async fn _demo() -> Result<(), aphrody_tui::TuiError> {
//! run(App::new(Counter::default())).await
//! # }
//! ```

#![forbid(unsafe_code)]

pub mod ansi;
pub mod error;
pub mod layout;
pub mod m3;
pub mod widgets;

use std::{
    io::{Stdout, stdout},
    time::Duration,
};

use crossterm::{
    event::{Event as CtEvent, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseEvent},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Frame, Terminal, backend::CrosstermBackend};
use tokio::time::{Interval, MissedTickBehavior, interval};

pub use crate::error::TuiError;

// ---------------------------------------------------------------------------
// Render loop constants
// ---------------------------------------------------------------------------

/// Target frame interval in milliseconds.  16 ms ~= 62.5 fps, just above the
/// 60 fps contract from PLAN.md tick T-10.
pub const TICK_MS: u64 = 16;

/// Maximum time spent blocking on a single crossterm poll before yielding to
/// the tokio scheduler.  Keeping this short keeps the loop responsive even
/// when no input arrives between ticks.
pub const POLL_MS: u64 = 4;

/// Compile-time sanity: 1000 / TICK_MS must round to at least 60.
const _: () = {
    assert!(1000 / TICK_MS >= 60, "TICK_MS too large for 60fps target");
};

// ---------------------------------------------------------------------------
// Public traits
// ---------------------------------------------------------------------------

/// Render a single frame of UI given the current application state.
///
/// Implementations must be deterministic and side-effect free — the loop may
/// re-render at any time without warning.
pub trait Render {
    /// Draw onto `frame`.  Use [`Frame::area`] for the root rect.
    fn render(&self, frame: &mut Frame<'_>);
}

/// Mutate application state in response to an event.
///
/// `Event` is generic so non-terminal sources (e.g. background channels) can
/// drive the state alongside [`TuiEvent`]s.
pub trait Update {
    /// Event payload — typically [`TuiEvent`] for terminal-only apps.
    type Event;
    /// Apply `event` to the application state.
    fn update(&mut self, event: Self::Event);
}

// ---------------------------------------------------------------------------
// Event taxonomy
// ---------------------------------------------------------------------------

/// Events the [`run`] loop dispatches to [`Update::update`].
///
/// `Tick` fires every [`TICK_MS`] milliseconds regardless of input activity,
/// giving the app a chance to advance animations or poll external state.
#[derive(Debug, Clone)]
pub enum TuiEvent {
    /// A key press or release from the terminal.  Filtered to
    /// [`KeyEventKind::Press`] only on Windows where crossterm emits both
    /// edges.
    Key(KeyEvent),
    /// A mouse event (movement, button, scroll).  Only emitted when the
    /// terminal has mouse capture enabled.
    Mouse(MouseEvent),
    /// Terminal resize: `(cols, rows)`.
    Resize(u16, u16),
    /// Periodic tick — fires at [`TICK_MS`] cadence.
    Tick,
}

// ---------------------------------------------------------------------------
// App wrapper
// ---------------------------------------------------------------------------

/// Stateful wrapper around any [`Render`] + [`Update`] implementor.
///
/// The wrapper exists so [`run`] can take ownership of the inner state and
/// drive it for the lifetime of the loop.  Most callers can use
/// [`App::new`] directly.
#[derive(Debug, Default)]
pub struct App<S> {
    state: S,
    /// Tracks whether the loop has been asked to terminate via Ctrl-C or an
    /// explicit `Quit` event.  Exposed for tests via [`App::quit`].
    quit: bool,
}

impl<S> App<S> {
    /// Wrap `state` in a fresh, non-quitting [`App`].
    #[must_use]
    pub fn new(state: S) -> Self {
        Self { state, quit: false }
    }

    /// Borrow the inner state.
    pub fn state(&self) -> &S {
        &self.state
    }

    /// Mutably borrow the inner state.
    pub fn state_mut(&mut self) -> &mut S {
        &mut self.state
    }

    /// Has the loop been asked to terminate?
    #[must_use]
    pub fn quit(&self) -> bool {
        self.quit
    }

    /// Request loop termination at the end of the current frame.
    pub fn request_quit(&mut self) {
        self.quit = true;
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// True if `event` is the canonical "interrupt" gesture — Ctrl-C or the
/// platform-specific cancel keys we treat as quit.
#[must_use]
pub fn is_interrupt(event: &KeyEvent) -> bool {
    matches!(
        event,
        KeyEvent {
            code: KeyCode::Char('c'),
            modifiers,
            ..
        } if modifiers.contains(KeyModifiers::CONTROL)
    )
}

/// Build the tokio `Interval` used by [`run`].  Exposed so integration tests
/// can sanity-check the tick cadence without spinning up a real terminal.
#[must_use]
pub fn build_tick_interval() -> Interval {
    let mut i = interval(Duration::from_millis(TICK_MS));
    // Burst on lag rather than spamming catch-up ticks — keeps the frame
    // budget honest if the previous tick overran.
    i.set_missed_tick_behavior(MissedTickBehavior::Skip);
    i
}

// ---------------------------------------------------------------------------
// Terminal RAII
// ---------------------------------------------------------------------------

/// Drop guard that restores the terminal to cooked mode even on panic.
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen);
    }
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>, TuiError> {
    enable_raw_mode().map_err(TuiError::Io)?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen).map_err(TuiError::Io)?;
    let backend = CrosstermBackend::new(out);
    Terminal::new(backend).map_err(TuiError::Io)
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Run `app` until either Ctrl-C is pressed or
/// [`App::request_quit`] is called from inside [`Update::update`].
///
/// The loop:
/// 1. Polls crossterm for input on a blocking task (4 ms timeout).
/// 2. Awaits a tokio interval ticking at [`TICK_MS`] ms.
/// 3. Dispatches the event (or `Tick`) into the user's [`Update`] impl.
/// 4. Calls [`Render::render`] and pushes the frame to the terminal.
///
/// # Errors
///
/// Returns [`TuiError::Io`] if the terminal cannot be initialised or if a
/// fatal crossterm error occurs mid-loop.  The terminal is always restored
/// before this function returns (panic or not), via [`TerminalGuard`].
pub async fn run<A>(mut app: App<A>) -> Result<(), TuiError>
where
    A: Render + Update<Event = TuiEvent>,
{
    let mut terminal = setup_terminal()?;
    let _guard = TerminalGuard;

    let mut ticker = build_tick_interval();

    loop {
        // ---- draw ----------------------------------------------------------
        let state_ref = &app.state;
        terminal.draw(|frame| state_ref.render(frame)).map_err(TuiError::Io)?;

        if app.quit {
            break;
        }

        // ---- event multiplex ----------------------------------------------
        let poll_handle = tokio::task::spawn_blocking(|| {
            crossterm::event::poll(Duration::from_millis(POLL_MS)).unwrap_or(false)
        });

        tokio::select! {
            biased;
            poll_result = poll_handle => {
                let has_input = poll_result.unwrap_or(false);
                if has_input {
                    drain_input(&mut app)?;
                }
            }
            _ = ticker.tick() => {
                app.state.update(TuiEvent::Tick);
            }
        }
    }

    Ok(())
}

/// Drain every queued crossterm event into the app.  Called whenever the
/// non-blocking poll says input is available.
fn drain_input<A>(app: &mut App<A>) -> Result<(), TuiError>
where
    A: Render + Update<Event = TuiEvent>,
{
    while crossterm::event::poll(Duration::from_millis(0)).map_err(TuiError::Io)? {
        let ev = crossterm::event::read().map_err(TuiError::Io)?;
        match ev {
            CtEvent::Key(k) => {
                if k.kind != KeyEventKind::Press {
                    continue;
                }
                if is_interrupt(&k) {
                    app.request_quit();
                    return Ok(());
                }
                app.state.update(TuiEvent::Key(k));
            },
            CtEvent::Mouse(m) => app.state.update(TuiEvent::Mouse(m)),
            CtEvent::Resize(w, h) => app.state.update(TuiEvent::Resize(w, h)),
            // FocusGained/Lost/Paste — ignored for now; widgets can opt in
            // via a future TuiEvent variant if needed.
            _ => {},
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Re-exports for downstream convenience
// ---------------------------------------------------------------------------

pub use crossterm::event::{KeyCode as CrosstermKeyCode, KeyModifiers as CrosstermKeyModifiers};
pub use ratatui::backend::TestBackend;
