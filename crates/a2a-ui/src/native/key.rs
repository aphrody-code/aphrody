// SPDX-License-Identifier: Apache-2.0
//! Keymap and action dispatch for the A2A TUI.
//!
//! All key bindings are centralised here so that [`app`] and [`ui`] never
//! hard-code `KeyCode` constants directly.

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// High-level actions that the application state machine handles.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Move the list cursor one item up.
    CursorUp,
    /// Move the list cursor one item down.
    CursorDown,
    /// Jump to the first item.
    CursorFirst,
    /// Jump to the last item.
    CursorLast,
    /// Expand the selected envelope into the detail pane.
    DetailExpand,
    /// Switch between the aphrody / winclean peer sides.
    SwitchSide,
    /// Force an immediate mailbox refresh (bypasses mtime cache).
    Refresh,
    /// Open the search bar.
    SearchOpen,
    /// Move to the next search match.
    SearchNext,
    /// Move to the previous search match.
    SearchPrev,
    /// Append a character to the active search query.
    SearchAppend(char),
    /// Delete the last character from the active search query.
    SearchBackspace,
    /// Confirm / close the search bar, keeping the highlight.
    SearchConfirm,
    /// Quit the application.
    Quit,
    /// Unrecognised key — ignored.
    Noop,
}

/// Map a raw crossterm [`KeyEvent`] to an [`Action`].
///
/// `search_active` controls whether characters are routed to the search bar
/// or to list navigation.
pub fn map(event: KeyEvent, search_active: bool) -> Action {
    // Ctrl+C always quits regardless of mode.
    if event.modifiers == KeyModifiers::CONTROL && event.code == KeyCode::Char('c') {
        return Action::Quit;
    }

    if search_active {
        return map_search(event);
    }

    map_normal(event)
}

fn map_normal(event: KeyEvent) -> Action {
    match event.code {
        KeyCode::Char('q') | KeyCode::Char('Q') => Action::Quit,
        KeyCode::Char('j') | KeyCode::Down => Action::CursorDown,
        KeyCode::Char('k') | KeyCode::Up => Action::CursorUp,
        KeyCode::Char('g') | KeyCode::Home => Action::CursorFirst,
        KeyCode::Char('G') | KeyCode::End => Action::CursorLast,
        KeyCode::Enter => Action::DetailExpand,
        KeyCode::Tab => Action::SwitchSide,
        KeyCode::Char('r') | KeyCode::Char('R') => Action::Refresh,
        KeyCode::Char('/') => Action::SearchOpen,
        KeyCode::Char('n') => Action::SearchNext,
        KeyCode::Char('N') => Action::SearchPrev,
        _ => Action::Noop,
    }
}

fn map_search(event: KeyEvent) -> Action {
    match event.code {
        KeyCode::Esc | KeyCode::Enter => Action::SearchConfirm,
        KeyCode::Backspace => Action::SearchBackspace,
        KeyCode::Char(c) => Action::SearchAppend(c),
        _ => Action::Noop,
    }
}
