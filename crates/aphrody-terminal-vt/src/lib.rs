// SPDX-License-Identifier: Apache-2.0
//! Pure Rust VT/ANSI parser and screen buffer.
//!
//! Implements a subset of the VT100/VT220/xterm protocol sufficient for a
//! real terminal emulator: printable characters, C0 controls, and the most
//! common CSI sequences (cursor movement, erase, SGR colour/attribute).
//!
//! The crate is `no_std`-compatible when the `std` feature of `vte` is
//! disabled; the only allocations are the cell grid and the dirty-row vector,
//! both of which use `alloc::vec::Vec`.

#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

use bitflags::bitflags;
use vte::{Params, Perform};

// ── Colour ───────────────────────────────────────────────────────────────────

/// An RGB colour value stored as three bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Color {
    /// Construct a colour from raw RGB bytes.
    #[must_use]
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }
}

// Standard 16-colour palette (xterm / VT220 defaults).
pub const BLACK: Color = Color::new(0, 0, 0);
pub const RED: Color = Color::new(170, 0, 0);
pub const GREEN: Color = Color::new(0, 170, 0);
pub const YELLOW: Color = Color::new(170, 85, 0);
pub const BLUE: Color = Color::new(0, 0, 170);
pub const MAGENTA: Color = Color::new(170, 0, 170);
pub const CYAN: Color = Color::new(0, 170, 170);
pub const WHITE: Color = Color::new(170, 170, 170);

pub const BRIGHT_BLACK: Color = Color::new(85, 85, 85);
pub const BRIGHT_RED: Color = Color::new(255, 85, 85);
pub const BRIGHT_GREEN: Color = Color::new(85, 255, 85);
pub const BRIGHT_YELLOW: Color = Color::new(255, 255, 85);
pub const BRIGHT_BLUE: Color = Color::new(85, 85, 255);
pub const BRIGHT_MAGENTA: Color = Color::new(255, 85, 255);
pub const BRIGHT_CYAN: Color = Color::new(85, 255, 255);
pub const BRIGHT_WHITE: Color = Color::new(255, 255, 255);

/// The default terminal foreground colour (light grey).
pub const DEFAULT_FG: Color = WHITE;
/// The default terminal background colour (black).
pub const DEFAULT_BG: Color = BLACK;

/// Map an ANSI colour index (0-15) to an [`Color`].
fn ansi_to_color(index: u16) -> Color {
    match index {
        0 => BLACK,
        1 => RED,
        2 => GREEN,
        3 => YELLOW,
        4 => BLUE,
        5 => MAGENTA,
        6 => CYAN,
        7 => WHITE,
        8 => BRIGHT_BLACK,
        9 => BRIGHT_RED,
        10 => BRIGHT_GREEN,
        11 => BRIGHT_YELLOW,
        12 => BRIGHT_BLUE,
        13 => BRIGHT_MAGENTA,
        14 => BRIGHT_CYAN,
        15 => BRIGHT_WHITE,
        _ => WHITE,
    }
}

// ── Text attributes ───────────────────────────────────────────────────────────

bitflags! {
    /// Text rendering attributes carried by a [`Cell`].
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
    pub struct Attr: u8 {
        /// Bold / increased intensity.
        const BOLD      = 0b0000_0001;
        /// Italic.
        const ITALIC    = 0b0000_0010;
        /// Single underline.
        const UNDERLINE = 0b0000_0100;
        /// Reversed video (fg and bg swapped).
        const INVERSE   = 0b0000_1000;
        /// Blinking text.
        const BLINK     = 0b0001_0000;
    }
}

// ── Screen cell ──────────────────────────────────────────────────────────────

/// A single character cell on the terminal grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    /// The Unicode scalar value displayed in this cell.
    pub ch: char,
    /// Foreground (text) colour.
    pub fg: Color,
    /// Background colour.
    pub bg: Color,
    /// Combined text attributes.
    pub attr: Attr,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            fg: DEFAULT_FG,
            bg: DEFAULT_BG,
            attr: Attr::empty(),
        }
    }
}

// ── Cursor ───────────────────────────────────────────────────────────────────

/// Cursor position and visibility state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cursor {
    /// Zero-based row (0 = top).
    pub row: u16,
    /// Zero-based column (0 = left).
    pub col: u16,
    /// Whether the cursor is currently visible.
    pub visible: bool,
}

impl Default for Cursor {
    fn default() -> Self {
        Self {
            row: 0,
            col: 0,
            visible: true,
        }
    }
}

// ── Terminal state ────────────────────────────────────────────────────────────

/// Complete VT parser + screen buffer.
///
/// Feed raw bytes via [`Self::feed`]; inspect the grid via [`Self::cell`] or
/// [`Self::cells`]; query which rows changed since last check via
/// [`Self::dirty_rows_drain`].
pub struct TerminalState {
    cols: u16,
    rows: u16,
    /// Flat cell grid, row-major: index = row * cols + col.
    cells: Vec<Cell>,
    cursor: Cursor,
    /// Current SGR foreground colour for new characters.
    current_fg: Color,
    /// Current SGR background colour for new characters.
    current_bg: Color,
    /// Current SGR attribute flags for new characters.
    current_attr: Attr,
    /// vte state machine.
    parser: vte::Parser,
    /// One dirty flag per row; set whenever a cell in that row changes.
    dirty: Vec<bool>,
}

impl TerminalState {
    /// Allocate a new terminal with the given dimensions.
    ///
    /// All cells are initialised to [`Cell::default`] (space, default colours).
    #[must_use]
    pub fn new(cols: u16, rows: u16) -> Self {
        let total = usize::from(cols) * usize::from(rows);
        Self {
            cols,
            rows,
            cells: vec![Cell::default(); total],
            cursor: Cursor::default(),
            current_fg: DEFAULT_FG,
            current_bg: DEFAULT_BG,
            current_attr: Attr::empty(),
            parser: vte::Parser::new(),
            dirty: vec![false; usize::from(rows)],
        }
    }

    /// Feed raw bytes from the pty/shell into the parser.
    ///
    /// This drives the vte state machine which calls back into the [`Perform`]
    /// implementation on `self` for every complete escape sequence or printable
    /// character.
    pub fn feed(&mut self, bytes: &[u8]) {
        // vte::Parser::advance takes a mutable performer reference.
        // We cannot pass `self` directly because `parser` is a field of `self`.
        // Work around: swap the parser out, advance, then put it back.
        let mut parser = core::mem::replace(&mut self.parser, vte::Parser::new());
        parser.advance(self, bytes);
        self.parser = parser;
    }

    /// Resize the terminal to `cols` × `rows`.
    ///
    /// Content that fits in the new grid is preserved; additional cells are
    /// filled with [`Cell::default`].  The cursor is clamped to the new bounds.
    pub fn resize(&mut self, cols: u16, rows: u16) {
        let new_total = usize::from(cols) * usize::from(rows);
        let mut new_cells = vec![Cell::default(); new_total];

        // Copy the overlapping rectangle from the old grid.
        let copy_cols = self.cols.min(cols) as usize;
        let copy_rows = self.rows.min(rows) as usize;
        for r in 0..copy_rows {
            for c in 0..copy_cols {
                let old_idx = r * usize::from(self.cols) + c;
                let new_idx = r * usize::from(cols) + c;
                new_cells[new_idx] = self.cells[old_idx];
            }
        }

        self.cols = cols;
        self.rows = rows;
        self.cells = new_cells;
        self.dirty = vec![true; usize::from(rows)]; // All rows are "new".

        // Clamp cursor to valid range.
        self.cursor.row = self.cursor.row.min(rows.saturating_sub(1));
        self.cursor.col = self.cursor.col.min(cols.saturating_sub(1));
    }

    /// Return the cell at the given position.
    ///
    /// Returns [`Cell::default`] for out-of-range coordinates.
    #[must_use]
    pub fn cell(&self, row: u16, col: u16) -> Cell {
        if row >= self.rows || col >= self.cols {
            return Cell::default();
        }
        let idx = usize::from(row) * usize::from(self.cols) + usize::from(col);
        self.cells[idx]
    }

    /// Return a read-only slice over the entire cell grid (row-major).
    #[must_use]
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    /// Return the current cursor state.
    #[must_use]
    pub fn cursor(&self) -> Cursor {
        self.cursor
    }

    /// Return the grid width in columns.
    #[must_use]
    pub fn cols(&self) -> u16 {
        self.cols
    }

    /// Return the grid height in rows.
    #[must_use]
    pub fn rows(&self) -> u16 {
        self.rows
    }

    /// Drain the dirty-row flags, returning the list of row indices that
    /// changed since the last call, and resetting all flags to `false`.
    pub fn dirty_rows_drain(&mut self) -> Vec<u16> {
        self.dirty
            .iter_mut()
            .enumerate()
            .filter_map(|(i, flag)| {
                if *flag {
                    *flag = false;
                    // invariant: i < rows <= u16::MAX, cast is safe
                    Some(i as u16)
                } else {
                    None
                }
            })
            .collect()
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    /// Write `ch` at the current cursor position and advance the cursor.
    ///
    /// Wraps to the next line when the cursor reaches the right margin;
    /// scrolls the grid up by one row when the cursor moves past the bottom.
    fn write_char(&mut self, ch: char) {
        if self.cols == 0 || self.rows == 0 {
            return;
        }
        let row = usize::from(self.cursor.row);
        let col = usize::from(self.cursor.col);
        let idx = row * usize::from(self.cols) + col;
        self.cells[idx] = Cell {
            ch,
            fg: self.current_fg,
            bg: self.current_bg,
            attr: self.current_attr,
        };
        self.dirty[row] = true;

        // Advance cursor; wrap + scroll if needed.
        if self.cursor.col + 1 >= self.cols {
            self.cursor.col = 0;
            self.newline();
        } else {
            self.cursor.col += 1;
        }
    }

    /// Move the cursor down one row, scrolling the grid if at the bottom.
    fn newline(&mut self) {
        if self.cursor.row + 1 >= self.rows {
            self.scroll_up();
        } else {
            self.cursor.row += 1;
        }
    }

    /// Scroll the entire grid up by one row; the bottom row is cleared.
    fn scroll_up(&mut self) {
        let cols = usize::from(self.cols);
        let rows = usize::from(self.rows);
        // Shift rows up by one.
        self.cells.copy_within(cols..rows * cols, 0);
        // Clear the last row.
        let last_start = (rows - 1) * cols;
        for cell in &mut self.cells[last_start..] {
            *cell = Cell::default();
        }
        // All rows are dirty after a scroll.
        self.dirty.iter_mut().for_each(|d| *d = true);
    }

    /// Erase part or all of the display (CSI J / ED).
    fn erase_display(&mut self, param: u16) {
        match param {
            // Erase from cursor to end of screen.
            0 => {
                let start =
                    usize::from(self.cursor.row) * usize::from(self.cols) + usize::from(self.cursor.col);
                for cell in &mut self.cells[start..] {
                    *cell = Cell::default();
                }
                for r in usize::from(self.cursor.row)..usize::from(self.rows) {
                    self.dirty[r] = true;
                }
            }
            // Erase from beginning of screen to cursor.
            1 => {
                let end =
                    usize::from(self.cursor.row) * usize::from(self.cols) + usize::from(self.cursor.col) + 1;
                for cell in &mut self.cells[..end] {
                    *cell = Cell::default();
                }
                for r in 0..=usize::from(self.cursor.row) {
                    self.dirty[r] = true;
                }
            }
            // Erase entire display.
            2 | _ => {
                for cell in &mut self.cells {
                    *cell = Cell::default();
                }
                self.dirty.iter_mut().for_each(|d| *d = true);
            }
        }
    }

    /// Erase part or all of the current line (CSI K / EL).
    fn erase_line(&mut self, param: u16) {
        let row = usize::from(self.cursor.row);
        let col = usize::from(self.cursor.col);
        let cols = usize::from(self.cols);
        let row_start = row * cols;
        match param {
            // Erase from cursor to end of line.
            0 => {
                for cell in &mut self.cells[row_start + col..row_start + cols] {
                    *cell = Cell::default();
                }
            }
            // Erase from beginning of line to cursor.
            1 => {
                for cell in &mut self.cells[row_start..row_start + col + 1] {
                    *cell = Cell::default();
                }
            }
            // Erase entire line.
            2 | _ => {
                for cell in &mut self.cells[row_start..row_start + cols] {
                    *cell = Cell::default();
                }
            }
        }
        self.dirty[row] = true;
    }

    /// Apply a single SGR parameter value to the current rendering state.
    fn apply_sgr_param(&mut self, p: u16) {
        match p {
            0 => {
                // Reset all attributes and colours.
                self.current_attr = Attr::empty();
                self.current_fg = DEFAULT_FG;
                self.current_bg = DEFAULT_BG;
            }
            1 => self.current_attr.insert(Attr::BOLD),
            3 => self.current_attr.insert(Attr::ITALIC),
            4 => self.current_attr.insert(Attr::UNDERLINE),
            5 => self.current_attr.insert(Attr::BLINK),
            7 => self.current_attr.insert(Attr::INVERSE),
            22 => self.current_attr.remove(Attr::BOLD),
            23 => self.current_attr.remove(Attr::ITALIC),
            24 => self.current_attr.remove(Attr::UNDERLINE),
            25 => self.current_attr.remove(Attr::BLINK),
            27 => self.current_attr.remove(Attr::INVERSE),
            // Standard foreground colours (30-37).
            30..=37 => self.current_fg = ansi_to_color(p - 30),
            39 => self.current_fg = DEFAULT_FG,
            // Standard background colours (40-47).
            40..=47 => self.current_bg = ansi_to_color(p - 40),
            49 => self.current_bg = DEFAULT_BG,
            // Bright foreground colours (90-97).
            90..=97 => self.current_fg = ansi_to_color(p - 90 + 8),
            // Bright background colours (100-107).
            100..=107 => self.current_bg = ansi_to_color(p - 100 + 8),
            // Ignore unrecognised parameters.
            _ => {}
        }
    }

    /// Process a complete SGR (Select Graphic Rendition) CSI sequence.
    ///
    /// Parameters are read from `params`; the parameter list may contain
    /// subparameter groups (e.g. `38:2:r:g:b` for 24-bit colour), but this
    /// implementation processes only the first element of each group.
    fn handle_sgr(&mut self, params: &Params) {
        let mut iter = params.iter();
        loop {
            let subparams = match iter.next() {
                Some(sp) => sp,
                None => break,
            };
            // Each group is a slice of u16; we look at the first element only
            // for attribute codes and ignore subparameter 24-bit / 256 colour
            // extensions (treating them as no-op rather than emitting garbage).
            let p = subparams.first().copied().unwrap_or(0);
            self.apply_sgr_param(p);
        }
    }

    /// Clamp `v` so that it is at least `1`, for use with CSI params where
    /// a missing or zero value means "1".
    #[inline]
    fn param_or_one(v: u16) -> u16 {
        if v == 0 { 1 } else { v }
    }
}

// ── vte::Perform implementation ───────────────────────────────────────────────

impl Perform for TerminalState {
    fn print(&mut self, c: char) {
        self.write_char(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            // CR — carriage return.
            b'\r' => self.cursor.col = 0,
            // LF, VT, FF — line feed variants.
            b'\n' | 0x0B | 0x0C => {
                self.newline();
            }
            // BS — backspace.
            b'\x08' => {
                if self.cursor.col > 0 {
                    self.cursor.col -= 1;
                }
            }
            // HT — horizontal tab (advance to next 8-column tab stop).
            b'\t' => {
                let next_tab = (self.cursor.col / 8 + 1) * 8;
                self.cursor.col = next_tab.min(self.cols.saturating_sub(1));
            }
            // BEL — ignored (no audio output from a screen buffer).
            0x07 => {}
            // All other C0 controls are ignored.
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &Params, _intermediates: &[u8], _ignore: bool, action: char) {
        // Collect the first two numeric parameters, defaulting to 0.
        let mut param_iter = params.iter();
        let p1: u16 = param_iter
            .next()
            .and_then(|sp| sp.first().copied())
            .unwrap_or(0);
        let p2: u16 = param_iter
            .next()
            .and_then(|sp| sp.first().copied())
            .unwrap_or(0);

        match action {
            // CUU — cursor up N rows.
            'A' => {
                let n = Self::param_or_one(p1);
                self.cursor.row = self.cursor.row.saturating_sub(n);
            }
            // CUD — cursor down N rows.
            'B' => {
                let n = Self::param_or_one(p1);
                self.cursor.row = (self.cursor.row + n).min(self.rows.saturating_sub(1));
            }
            // CUF — cursor forward N columns.
            'C' => {
                let n = Self::param_or_one(p1);
                self.cursor.col = (self.cursor.col + n).min(self.cols.saturating_sub(1));
            }
            // CUB — cursor backward N columns.
            'D' => {
                let n = Self::param_or_one(p1);
                self.cursor.col = self.cursor.col.saturating_sub(n);
            }
            // CUP / HVP — cursor position (row; col) — 1-based parameters.
            'H' | 'f' => {
                let row = Self::param_or_one(p1).saturating_sub(1);
                let col = Self::param_or_one(p2).saturating_sub(1);
                self.cursor.row = row.min(self.rows.saturating_sub(1));
                self.cursor.col = col.min(self.cols.saturating_sub(1));
            }
            // ED — erase in display.
            'J' => self.erase_display(p1),
            // EL — erase in line.
            'K' => self.erase_line(p1),
            // SGR — select graphic rendition.
            'm' => self.handle_sgr(params),
            // All other CSI sequences are ignored.
            _ => {}
        }
    }

    // The remaining Perform methods handle DCS and OSC sequences which are
    // not required for the VT100/xterm subset implemented here.  They are
    // left as empty bodies (the trait provides default no-ops, but we write
    // explicit empty implementations to make the intent clear and to satisfy
    // clippy::pedantic).

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}

    fn put(&mut self, _byte: u8) {}

    fn unhook(&mut self) {}

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {}

    fn esc_dispatch(&mut self, _intermediates: &[u8], _ignore: bool, _byte: u8) {}
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build a small terminal and feed it the given bytes.
    fn make(cols: u16, rows: u16, input: &[u8]) -> TerminalState {
        let mut t = TerminalState::new(cols, rows);
        t.feed(input);
        t
    }

    #[test]
    fn simple_ascii_print() {
        let t = make(80, 24, b"Hello");
        assert_eq!(t.cell(0, 0).ch, 'H');
        assert_eq!(t.cell(0, 1).ch, 'e');
        assert_eq!(t.cell(0, 4).ch, 'o');
        assert_eq!(t.cursor().col, 5);
    }

    #[test]
    fn lf_advances_to_next_row() {
        // LF advances the row but does NOT reset the column (that is CR's job).
        // After 'A', cursor is at (0, 1).  After '\n', cursor is at (1, 1).
        // 'B' therefore lands at (1, 1).
        let t = make(80, 24, b"A\nB");
        assert_eq!(t.cell(0, 0).ch, 'A');
        // Column 1, row 1: where cursor was after LF.
        assert_eq!(t.cell(1, 1).ch, 'B');
        // A CR+LF pair puts the next char at column 0 of the next row.
        let t2 = make(80, 24, b"A\r\nB");
        assert_eq!(t2.cell(0, 0).ch, 'A');
        assert_eq!(t2.cell(1, 0).ch, 'B');
    }

    #[test]
    fn cr_returns_to_column_zero() {
        let t = make(80, 24, b"ABC\rX");
        // CR returns col to 0; 'X' overwrites 'A'.
        assert_eq!(t.cell(0, 0).ch, 'X');
        assert_eq!(t.cell(0, 1).ch, 'B');
    }

    #[test]
    fn csi_2j_clears_screen() {
        let mut t = make(80, 24, b"Hello");
        // All cells should be space after CSI 2J.
        t.feed(b"\x1b[2J");
        for cell in t.cells() {
            assert_eq!(cell.ch, ' ');
        }
    }

    #[test]
    fn csi_h_homes_cursor() {
        let mut t = TerminalState::new(80, 24);
        t.feed(b"\x1b[10;20H"); // move to row 10, col 20
        assert_eq!(t.cursor().row, 9); // 0-based
        assert_eq!(t.cursor().col, 19);
        t.feed(b"\x1b[H"); // home
        assert_eq!(t.cursor().row, 0);
        assert_eq!(t.cursor().col, 0);
    }

    #[test]
    fn csi_5a_moves_cursor_up_five() {
        let mut t = TerminalState::new(80, 24);
        t.feed(b"\x1b[10;1H"); // row 10, col 1 (1-based)
        assert_eq!(t.cursor().row, 9);
        t.feed(b"\x1b[5A"); // up 5
        assert_eq!(t.cursor().row, 4);
    }

    #[test]
    fn csi_31m_sets_red_foreground() {
        let mut t = TerminalState::new(80, 24);
        t.feed(b"\x1b[31m"); // SGR 31 = red fg
        t.feed(b"R");
        let cell = t.cell(0, 0);
        assert_eq!(cell.fg, RED);
        assert_eq!(cell.ch, 'R');
    }

    #[test]
    fn csi_1_31m_bold_red() {
        let mut t = TerminalState::new(80, 24);
        t.feed(b"\x1b[1;31m"); // bold + red fg
        t.feed(b"B");
        let cell = t.cell(0, 0);
        assert_eq!(cell.fg, RED);
        assert!(cell.attr.contains(Attr::BOLD));
    }

    #[test]
    fn dirty_rows_drain_returns_changed_rows_and_resets() {
        let mut t = TerminalState::new(80, 24);
        t.feed(b"A"); // row 0 dirty
        let dirty = t.dirty_rows_drain();
        assert!(dirty.contains(&0));
        // After drain, no more dirty rows.
        let dirty2 = t.dirty_rows_drain();
        assert!(dirty2.is_empty());
    }

    #[test]
    fn resize_preserves_content_and_clamps_cursor() {
        let mut t = TerminalState::new(80, 24);
        t.feed(b"Hi");
        assert_eq!(t.cell(0, 0).ch, 'H');
        t.resize(40, 12);
        assert_eq!(t.cell(0, 0).ch, 'H');
        assert_eq!(t.cell(0, 1).ch, 'i');
        // Cursor was at (0, 2); still within bounds.
        assert_eq!(t.cursor().col, 2);
    }

    #[test]
    fn sgr_reset_restores_defaults() {
        let mut t = TerminalState::new(80, 24);
        t.feed(b"\x1b[1;31m"); // bold red
        t.feed(b"\x1b[0m");    // reset
        t.feed(b"X");
        let cell = t.cell(0, 0);
        assert_eq!(cell.fg, DEFAULT_FG);
        assert!(!cell.attr.contains(Attr::BOLD));
    }
}
