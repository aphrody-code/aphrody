// SPDX-License-Identifier: Apache-2.0
//! Alternate screen buffer (DECSET 1049 / DECRST 1049).
//!
//! When alt-screen mode is entered the terminal saves the primary screen,
//! cursor, and SGR state, then clears and switches to a fresh alternate
//! buffer. Exiting restores the saved state verbatim.

use crate::{Attr, Cell, Color, Cursor};

/// A complete snapshot of the visible screen plus pen/cursor state.
///
/// Used as the storage for the primary buffer while the terminal is in
/// alternate-screen mode, and vice-versa.
#[derive(Clone)]
pub(crate) struct ScreenSnapshot {
    pub cols: u16,
    pub rows: u16,
    pub cells: Vec<Cell>,
    pub cursor: Cursor,
    pub fg: Color,
    pub bg: Color,
    pub attr: Attr,
}
