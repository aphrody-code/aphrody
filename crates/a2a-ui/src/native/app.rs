// SPDX-License-Identifier: Apache-2.0
//! Application state and async event loop for the A2A coord channel TUI.
//!
//! `run` is the single entry point: it initialises the terminal in raw mode,
//! drives the draw/event loop, and restores the terminal on exit (including
//! on panic via the `restore_terminal` guard).

use std::path::Path;
use std::time::{Duration, SystemTime};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend, widgets::ListState};
use tokio::time::interval;

use super::{
    key::{self, Action},
    mailbox::{mailbox_pair_envelopes, MailboxPair},
    ui::{DrawState, Palette, draw},
    NativeTuiArgs,
};

use crate::Envelope;

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

/// All mutable state for the TUI event loop.
struct App {
    /// Coord directory.
    coord_dir: std::path::PathBuf,
    /// Both peer mailboxes.
    mailboxes: MailboxPair,
    /// Which side is active: `"aphrody"` or `"winclean"`.
    active_side: String,
    /// Ratatui list cursor state.
    list_state: ListState,
    /// Index of the selected envelope (detail pane).
    selected: Option<usize>,
    /// Active search query (None = search bar closed).
    search: Option<String>,
    /// Indices matching the search query.
    search_matches: Vec<usize>,
    /// Current match cursor inside `search_matches`.
    search_match_cursor: usize,
    /// Whether the search bar is in input mode.
    search_active: bool,
    /// Resolved M3 palette.
    palette: Palette,
    /// Should the loop exit?
    quit: bool,
    /// Last heartbeat mtime for aphrody.
    aphrody_hb: Option<SystemTime>,
    /// Last heartbeat mtime for winclean.
    winclean_hb: Option<SystemTime>,
}

impl App {
    fn new(coord_dir: std::path::PathBuf, args: &NativeTuiArgs) -> Result<Self> {
        let mut mailboxes = MailboxPair::new(&coord_dir);
        mailboxes.refresh()?;

        let aphrody_hb = MailboxPair::heartbeat_mtime(&coord_dir, "aphrody");
        let winclean_hb = MailboxPair::heartbeat_mtime(&coord_dir, "winclean");

        let mut list_state = ListState::default();
        list_state.select(Some(0));

        Ok(Self {
            coord_dir,
            mailboxes,
            active_side: args.initial_side.clone(),
            list_state,
            selected: None,
            search: None,
            search_matches: Vec::new(),
            search_match_cursor: 0,
            search_active: false,
            palette: Palette::resolve(),
            quit: false,
            aphrody_hb,
            winclean_hb,
        })
    }

    /// Reference to the currently visible envelope slice.
    fn envelopes(&self) -> &[Envelope] {
        mailbox_pair_envelopes(&self.mailboxes, &self.active_side)
    }

    /// Current cursor position in the list (0-based).
    fn cursor(&self) -> usize {
        self.list_state.selected().unwrap_or(0)
    }

    fn clamp_cursor(&mut self) {
        let len = self.envelopes().len();
        if len == 0 {
            self.list_state.select(None);
        } else {
            let c = self.cursor().min(len - 1);
            self.list_state.select(Some(c));
        }
    }

    fn handle_action(&mut self, action: Action) {
        match action {
            Action::Quit => self.quit = true,

            Action::CursorUp => {
                let c = self.cursor().saturating_sub(1);
                self.list_state.select(Some(c));
            }
            Action::CursorDown => {
                let len = self.envelopes().len();
                if len > 0 {
                    let c = (self.cursor() + 1).min(len - 1);
                    self.list_state.select(Some(c));
                }
            }
            Action::CursorFirst => {
                if !self.envelopes().is_empty() {
                    self.list_state.select(Some(0));
                }
            }
            Action::CursorLast => {
                let len = self.envelopes().len();
                if len > 0 {
                    self.list_state.select(Some(len - 1));
                }
            }

            Action::DetailExpand => {
                self.selected = self.list_state.selected();
            }

            Action::SwitchSide => {
                self.active_side = if self.active_side == "aphrody" {
                    "winclean".to_owned()
                } else {
                    "aphrody".to_owned()
                };
                self.list_state.select(Some(0));
                self.selected = None;
                self.search = None;
                self.search_matches.clear();
                self.search_match_cursor = 0;
                self.search_active = false;
            }

            Action::Refresh => {
                let _ = self.mailboxes.refresh();
                self.aphrody_hb = MailboxPair::heartbeat_mtime(&self.coord_dir, "aphrody");
                self.winclean_hb = MailboxPair::heartbeat_mtime(&self.coord_dir, "winclean");
                self.clamp_cursor();
                self.rebuild_matches();
            }

            Action::SearchOpen => {
                self.search = Some(String::new());
                self.search_active = true;
            }
            Action::SearchConfirm => {
                self.search_active = false;
                if self.search.as_deref() == Some("") {
                    self.search = None;
                    self.search_matches.clear();
                }
            }
            Action::SearchBackspace => {
                if let Some(ref mut q) = self.search {
                    q.pop();
                    self.rebuild_matches();
                }
            }
            Action::SearchAppend(c) => {
                if let Some(ref mut q) = self.search {
                    q.push(c);
                    self.rebuild_matches();
                }
            }
            Action::SearchNext => {
                if !self.search_matches.is_empty() {
                    self.search_match_cursor =
                        (self.search_match_cursor + 1) % self.search_matches.len();
                    let idx = self.search_matches[self.search_match_cursor];
                    self.list_state.select(Some(idx));
                }
            }
            Action::SearchPrev => {
                if !self.search_matches.is_empty() {
                    self.search_match_cursor = self
                        .search_match_cursor
                        .checked_sub(1)
                        .unwrap_or(self.search_matches.len() - 1);
                    let idx = self.search_matches[self.search_match_cursor];
                    self.list_state.select(Some(idx));
                }
            }

            Action::Noop => {}
        }
    }

    /// Rebuild `search_matches` from the current query and active side.
    fn rebuild_matches(&mut self) {
        self.search_matches.clear();
        self.search_match_cursor = 0;

        let query = match self.search.as_deref() {
            Some(q) if !q.is_empty() => q.to_lowercase(),
            _ => return,
        };

        let matches: Vec<usize> = self
            .envelopes()
            .iter()
            .enumerate()
            .filter(|(_, env)| {
                env.topic.to_lowercase().contains(&query)
                    || env.kind.to_lowercase().contains(&query)
                    || env.from.to_lowercase().contains(&query)
                    || env.body.to_lowercase().contains(&query)
            })
            .map(|(i, _)| i)
            .collect();

        self.search_matches = matches;
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Async event loop.  Blocks until the user quits.
pub async fn run(coord_dir: &Path, args: NativeTuiArgs) -> Result<()> {
    let mut app = App::new(coord_dir.to_path_buf(), &args)?;

    // --- terminal setup -----------------------------------------------------
    enable_raw_mode().map_err(|e| anyhow::anyhow!("enable_raw_mode: {e}"))?;
    let mut stdout = std::io::stdout();
    execute!(stdout, EnterAlternateScreen)
        .map_err(|e| anyhow::anyhow!("EnterAlternateScreen: {e}"))?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal =
        Terminal::new(backend).map_err(|e| anyhow::anyhow!("Terminal::new: {e}"))?;

    // Ensure we restore the terminal even on panic.
    let restore = TerminalGuard;

    let refresh = Duration::from_secs(args.refresh_secs);
    let mut tick = interval(refresh);

    let result = event_loop(&mut terminal, &mut app, &mut tick).await;

    // Explicit restore before the guard drop so we can capture errors.
    drop(restore);

    result
}

/// The actual draw + event loop.
async fn event_loop(
    terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>,
    app: &mut App,
    tick: &mut tokio::time::Interval,
) -> Result<()> {
    loop {
        // Draw the current frame.
        {
            let envelopes = mailbox_pair_envelopes(&app.mailboxes, &app.active_side);
            let matches_snapshot = app.search_matches.clone();
            let active_side_str = app.active_side.clone();
            let search_str = app.search.clone();
            let selected = app.selected;
            let aphrody_hb = app.aphrody_hb;
            let winclean_hb = app.winclean_hb;
            let palette = &app.palette;
            let list_state = &mut app.list_state;

            terminal.draw(|frame| {
                let mut state = DrawState {
                    envelopes,
                    list_state,
                    selected,
                    active_side: &active_side_str,
                    aphrody_hb,
                    winclean_hb,
                    search: search_str.as_deref(),
                    search_matches: &matches_snapshot,
                    palette,
                };
                draw(frame, &mut state);
            })?;
        }

        if app.quit {
            break;
        }

        // Wait for either a key event or the periodic tick — whichever comes
        // first.  crossterm event reading is synchronous (blocking), so we
        // run it on a blocking thread via `spawn_blocking`.
        let timeout_ms = 100u64;
        let key_ready = tokio::task::spawn_blocking(move || {
            event::poll(Duration::from_millis(timeout_ms)).unwrap_or(false)
        })
        .await
        .unwrap_or(false);

        if key_ready {
            if let Ok(Event::Key(key)) = event::read() {
                if key.kind == KeyEventKind::Press {
                    let action = key::map(key, app.search_active);
                    app.handle_action(action);
                }
            }
        }

        // Non-blocking tick check: if the interval fires immediately, refresh.
        // We use `try_recv`-style semantics by wrapping in `tokio::select!`
        // with a zero-duration sleep on the else branch.
        let tick_fired = tokio::select! {
            _ = tick.tick() => true,
            _ = tokio::time::sleep(Duration::ZERO) => false,
        };
        if tick_fired {
            let changed = app.mailboxes.refresh().unwrap_or(false);
            app.aphrody_hb = MailboxPair::heartbeat_mtime(&app.coord_dir, "aphrody");
            app.winclean_hb = MailboxPair::heartbeat_mtime(&app.coord_dir, "winclean");
            if changed {
                app.clamp_cursor();
                app.rebuild_matches();
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// RAII terminal restore guard
// ---------------------------------------------------------------------------

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
    }
}
