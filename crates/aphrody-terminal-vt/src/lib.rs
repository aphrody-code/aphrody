// SPDX-License-Identifier: Apache-2.0
//! Pure Rust VT/ANSI parser and screen buffer.
//!
//! Implements the subset of the VT100/VT220/xterm protocol required by
//! modern TUIs (Ink + React, Bubble Tea, ratatui), Claude Code, and Gemini
//! CLI. On top of the basics (printable characters, C0 controls, SGR, ED,
//! EL, cursor movement) the parser supports:
//!
//! - Alternate screen buffer (DECSET/DECRST 1049)
//! - SGR 1006 mouse tracking enable + coordinate encoder/decoder
//! - 24-bit true colour (`SGR 38;2;R;G;B`, `48;2;R;G;B`)
//! - Cursor save / restore (DECSC `ESC 7` / DECRC `ESC 8`)
//! - Bracketed paste mode (DECSET 2004 + `ESC [ 200~` / `ESC [ 201~`)
//! - Focus in / focus out events (DECSET 1004 + `ESC [ I` / `ESC [ O`)
//! - DECSTBM scrolling region with IL / DL support
//! - OSC 0 window title and OSC 52 clipboard set (base64-decoded)

#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::many_single_char_names,
    clippy::match_same_arms,
    clippy::struct_excessive_bools,
    clippy::cast_possible_truncation,
    clippy::range_plus_one,
    clippy::explicit_iter_loop,
)]

use bitflags::bitflags;
use vte::{Params, Perform};

mod alt_screen;
pub mod envelope;
pub mod mouse;
mod osc;

use alt_screen::ScreenSnapshot;
pub use envelope::{
    BEL, OSC_INTRODUCER, ST, strip_osc_envelope, strip_osc_envelope_required,
    strip_osc_envelope_str,
};
pub use mouse::{
    decode_sgr_1006, encode_sgr_1006, MouseButton, MouseEvent, MouseEventKind, MouseModifiers,
};

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

/// Map an ANSI colour index (0-15) to a [`Color`].
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

// ── Saved cursor (DECSC) ─────────────────────────────────────────────────────

#[derive(Clone, Copy, Debug)]
struct SavedCursor {
    cursor: Cursor,
    fg: Color,
    bg: Color,
    attr: Attr,
}

// ── Focus / paste events ─────────────────────────────────────────────────────

/// Window focus change reported by `CSI I` / `CSI O` while focus-event
/// tracking is enabled (DECSET 1004).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FocusEvent {
    /// `ESC [ I` — window gained focus.
    In,
    /// `ESC [ O` — window lost focus.
    Out,
}

/// Bracketed paste boundary event (`ESC [ 200~` / `ESC [ 201~`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PasteEvent {
    Start,
    End,
}

// ── Terminal state ────────────────────────────────────────────────────────────

/// Complete VT parser + screen buffer.
///
/// Feed raw bytes via [`Self::feed`]; inspect the grid via [`Self::cell`] or
/// [`Self::cells`]; drain auxiliary events (mouse, focus, paste, OSC) via
/// the dedicated `drain_*` accessors.
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

    // ── Extensions ──────────────────────────────────────────────────────────
    /// Snapshot of the primary buffer while alt-screen mode is active.
    alt_buffer: Option<ScreenSnapshot>,
    /// DECSC saved cursor + pen (independent of alt-screen swap).
    saved: Option<SavedCursor>,
    /// DECSTBM scrolling region top (0-based, inclusive).
    scroll_top: u16,
    /// DECSTBM scrolling region bottom (0-based, inclusive).
    scroll_bot: u16,
    /// Whether mouse-tracking has been requested (DECSET 1000 / 1006).
    mouse_tracking: bool,
    /// Whether SGR 1006 encoding has been negotiated.
    mouse_sgr_1006: bool,
    /// Whether bracketed paste mode is enabled (DECSET 2004).
    bracketed_paste: bool,
    /// Whether focus-event reporting is enabled (DECSET 1004).
    focus_events: bool,
    /// Set while we are between `ESC [ 200~` and `ESC [ 201~`.
    paste_active: bool,

    // ── Drainable event queues ──────────────────────────────────────────────
    focus_queue: Vec<FocusEvent>,
    paste_queue: Vec<PasteEvent>,
    title_queue: Vec<String>,
    clipboard_queue: Vec<String>,
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

            alt_buffer: None,
            saved: None,
            scroll_top: 0,
            scroll_bot: rows.saturating_sub(1),
            mouse_tracking: false,
            mouse_sgr_1006: false,
            bracketed_paste: false,
            focus_events: false,
            paste_active: false,
            focus_queue: Vec::new(),
            paste_queue: Vec::new(),
            title_queue: Vec::new(),
            clipboard_queue: Vec::new(),
        }
    }

    /// Feed raw bytes from the pty/shell into the parser.
    pub fn feed(&mut self, bytes: &[u8]) {
        let mut parser = core::mem::replace(&mut self.parser, vte::Parser::new());
        parser.advance(self, bytes);
        self.parser = parser;
    }

    /// Resize the terminal to `cols` × `rows`.
    pub fn resize(&mut self, cols: u16, rows: u16) {
        let new_total = usize::from(cols) * usize::from(rows);
        let mut new_cells = vec![Cell::default(); new_total];
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
        self.dirty = vec![true; usize::from(rows)];
        self.cursor.row = self.cursor.row.min(rows.saturating_sub(1));
        self.cursor.col = self.cursor.col.min(cols.saturating_sub(1));
        // Re-clamp scroll region.
        self.scroll_top = self.scroll_top.min(rows.saturating_sub(1));
        self.scroll_bot = self.scroll_bot.min(rows.saturating_sub(1));
        if self.scroll_top > self.scroll_bot {
            self.scroll_top = 0;
            self.scroll_bot = rows.saturating_sub(1);
        }
    }

    /// Return the cell at the given position.
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

    /// Drain the dirty-row flags.
    pub fn dirty_rows_drain(&mut self) -> Vec<u16> {
        self.dirty
            .iter_mut()
            .enumerate()
            .filter_map(|(i, flag)| {
                if *flag {
                    *flag = false;
                    Some(i as u16)
                } else {
                    None
                }
            })
            .collect()
    }

    // ── Public extension accessors ──────────────────────────────────────────

    /// True while the alternate screen buffer is active.
    #[must_use]
    pub fn alt_screen_active(&self) -> bool {
        self.alt_buffer.is_some()
    }

    /// Current SGR foreground colour (for inspection / encoder reuse).
    #[must_use]
    pub fn current_fg(&self) -> Color {
        self.current_fg
    }
    /// Current SGR background colour.
    #[must_use]
    pub fn current_bg(&self) -> Color {
        self.current_bg
    }
    /// Current SGR attribute flags.
    #[must_use]
    pub fn current_attr(&self) -> Attr {
        self.current_attr
    }

    /// Inclusive `(top, bottom)` rows of the current DECSTBM scroll region.
    #[must_use]
    pub fn scroll_region(&self) -> (u16, u16) {
        (self.scroll_top, self.scroll_bot)
    }

    /// Whether mouse tracking has been requested by the application.
    #[must_use]
    pub fn mouse_tracking_enabled(&self) -> bool {
        self.mouse_tracking
    }
    /// Whether SGR 1006 mouse encoding is the active wire format.
    #[must_use]
    pub fn mouse_sgr_1006_enabled(&self) -> bool {
        self.mouse_sgr_1006
    }
    /// Whether bracketed-paste mode is enabled.
    #[must_use]
    pub fn bracketed_paste_enabled(&self) -> bool {
        self.bracketed_paste
    }
    /// Whether focus-event tracking is enabled.
    #[must_use]
    pub fn focus_events_enabled(&self) -> bool {
        self.focus_events
    }
    /// True between the start and end of an active paste.
    #[must_use]
    pub fn paste_active(&self) -> bool {
        self.paste_active
    }

    /// Drain queued focus events, returning them in arrival order.
    pub fn drain_focus_events(&mut self) -> Vec<FocusEvent> {
        core::mem::take(&mut self.focus_queue)
    }

    /// Drain queued paste boundary events.
    pub fn drain_paste_events(&mut self) -> Vec<PasteEvent> {
        core::mem::take(&mut self.paste_queue)
    }

    /// Drain queued window-title updates (most recent last).
    pub fn drain_window_titles(&mut self) -> Vec<String> {
        core::mem::take(&mut self.title_queue)
    }

    /// Drain queued OSC 52 clipboard payloads (already base64-decoded).
    pub fn drain_clipboard_set(&mut self) -> Vec<String> {
        core::mem::take(&mut self.clipboard_queue)
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

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

        if self.cursor.col + 1 >= self.cols {
            self.cursor.col = 0;
            self.newline();
        } else {
            self.cursor.col += 1;
        }
    }

    fn newline(&mut self) {
        // Inside the scroll region, scroll up when crossing the bottom margin.
        // Outside, fall back to grid bottom.
        let bottom = self.scroll_bot;
        if self.cursor.row >= bottom {
            self.scroll_region_up(1);
        } else if self.cursor.row + 1 < self.rows {
            self.cursor.row += 1;
        }
    }

    /// Scroll the active region up by `n` rows; the bottom rows are cleared.
    fn scroll_region_up(&mut self, n: u16) {
        let n = n.min(self.scroll_bot - self.scroll_top + 1);
        let cols = usize::from(self.cols);
        let top = usize::from(self.scroll_top);
        let bot = usize::from(self.scroll_bot);
        let n_us = usize::from(n);
        // Move rows [top+n ..= bot] up by n.
        let src_start = (top + n_us) * cols;
        let src_end = (bot + 1) * cols;
        let dst_start = top * cols;
        if src_start <= src_end && src_end <= self.cells.len() && dst_start <= src_start {
            self.cells.copy_within(src_start..src_end, dst_start);
        }
        // Clear the last n rows of the region.
        let clear_start = (bot + 1 - n_us) * cols;
        let clear_end = (bot + 1) * cols;
        for cell in &mut self.cells[clear_start..clear_end] {
            *cell = Cell::default();
        }
        for r in top..=bot {
            self.dirty[r] = true;
        }
    }

    /// Insert `n` blank lines at the cursor (IL).
    fn insert_lines(&mut self, n: u16) {
        if self.cursor.row < self.scroll_top || self.cursor.row > self.scroll_bot {
            return;
        }
        let n = n.min(self.scroll_bot - self.cursor.row + 1);
        let cols = usize::from(self.cols);
        let top = usize::from(self.cursor.row);
        let bot = usize::from(self.scroll_bot);
        let n_us = usize::from(n);
        // Move rows [top ..= bot-n] down by n.
        if bot >= top + n_us {
            let src_start = top * cols;
            let src_end = (bot - n_us + 1) * cols;
            let dst_start = (top + n_us) * cols;
            // copy_within handles overlap (moves down).
            self.cells.copy_within(src_start..src_end, dst_start);
        }
        // Clear the n inserted lines at the cursor.
        let clear_start = top * cols;
        let clear_end = (top + n_us).min(bot + 1) * cols;
        for cell in &mut self.cells[clear_start..clear_end] {
            *cell = Cell::default();
        }
        for r in top..=bot {
            self.dirty[r] = true;
        }
    }

    /// Delete `n` lines at the cursor (DL).
    fn delete_lines(&mut self, n: u16) {
        if self.cursor.row < self.scroll_top || self.cursor.row > self.scroll_bot {
            return;
        }
        let n = n.min(self.scroll_bot - self.cursor.row + 1);
        let cols = usize::from(self.cols);
        let top = usize::from(self.cursor.row);
        let bot = usize::from(self.scroll_bot);
        let n_us = usize::from(n);
        // Move rows [top+n ..= bot] up by n.
        if bot >= top + n_us {
            let src_start = (top + n_us) * cols;
            let src_end = (bot + 1) * cols;
            let dst_start = top * cols;
            self.cells.copy_within(src_start..src_end, dst_start);
        }
        // Clear the n trailing lines.
        let clear_start = (bot + 1 - n_us) * cols;
        let clear_end = (bot + 1) * cols;
        for cell in &mut self.cells[clear_start..clear_end] {
            *cell = Cell::default();
        }
        for r in top..=bot {
            self.dirty[r] = true;
        }
    }

    fn erase_display(&mut self, param: u16) {
        match param {
            0 => {
                let start = usize::from(self.cursor.row) * usize::from(self.cols)
                    + usize::from(self.cursor.col);
                for cell in &mut self.cells[start..] {
                    *cell = Cell::default();
                }
                for r in usize::from(self.cursor.row)..usize::from(self.rows) {
                    self.dirty[r] = true;
                }
            }
            1 => {
                let end = usize::from(self.cursor.row) * usize::from(self.cols)
                    + usize::from(self.cursor.col)
                    + 1;
                for cell in &mut self.cells[..end] {
                    *cell = Cell::default();
                }
                for r in 0..=usize::from(self.cursor.row) {
                    self.dirty[r] = true;
                }
            }
            _ => {
                for cell in &mut self.cells {
                    *cell = Cell::default();
                }
                self.dirty.iter_mut().for_each(|d| *d = true);
            }
        }
    }

    fn erase_line(&mut self, param: u16) {
        let row = usize::from(self.cursor.row);
        let col = usize::from(self.cursor.col);
        let cols = usize::from(self.cols);
        let row_start = row * cols;
        match param {
            0 => {
                for cell in &mut self.cells[row_start + col..row_start + cols] {
                    *cell = Cell::default();
                }
            }
            1 => {
                for cell in &mut self.cells[row_start..row_start + col + 1] {
                    *cell = Cell::default();
                }
            }
            _ => {
                for cell in &mut self.cells[row_start..row_start + cols] {
                    *cell = Cell::default();
                }
            }
        }
        self.dirty[row] = true;
    }

    fn apply_sgr_param(&mut self, p: u16) {
        match p {
            0 => {
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
            30..=37 => self.current_fg = ansi_to_color(p - 30),
            39 => self.current_fg = DEFAULT_FG,
            40..=47 => self.current_bg = ansi_to_color(p - 40),
            49 => self.current_bg = DEFAULT_BG,
            90..=97 => self.current_fg = ansi_to_color(p - 90 + 8),
            100..=107 => self.current_bg = ansi_to_color(p - 100 + 8),
            _ => {}
        }
    }

    /// Process SGR (`m`) parameters, honouring 24-bit `38;2;R;G;B` and
    /// `48;2;R;G;B` extensions (both `;`-separated and `:`-sub-parameter
    /// forms are accepted).
    fn handle_sgr(&mut self, params: &Params) {
        // Flatten the parameter list into a single Vec<u16>, but remember
        // sub-parameter group boundaries by also recording whether the next
        // element is a continuation. For SGR 38;2;R;G;B / 48;2;R;G;B we need
        // to consume 4 elements after the leading 38/48 regardless of how
        // they were grouped.
        let mut flat: Vec<u16> = Vec::with_capacity(8);
        for group in params.iter() {
            for &v in group {
                flat.push(v);
            }
        }

        let mut i = 0;
        while i < flat.len() {
            let p = flat[i];
            match p {
                38 | 48 => {
                    // Need at least one selector after.
                    if i + 1 >= flat.len() {
                        i += 1;
                        continue;
                    }
                    let selector = flat[i + 1];
                    if selector == 2 && i + 4 < flat.len() {
                        // 24-bit RGB.
                        let r = flat[i + 2].min(255) as u8;
                        let g = flat[i + 3].min(255) as u8;
                        let b = flat[i + 4].min(255) as u8;
                        let c = Color::new(r, g, b);
                        if p == 38 {
                            self.current_fg = c;
                        } else {
                            self.current_bg = c;
                        }
                        i += 5;
                        continue;
                    }
                    if selector == 5 && i + 2 < flat.len() {
                        // 256-colour index. Map the first 16 via the palette;
                        // for 16..=231 use the standard xterm 6x6x6 cube; for
                        // 232..=255 use the greyscale ramp.
                        let idx = flat[i + 2];
                        let c = palette_256(idx);
                        if p == 38 {
                            self.current_fg = c;
                        } else {
                            self.current_bg = c;
                        }
                        i += 3;
                        continue;
                    }
                    i += 2;
                }
                _ => {
                    self.apply_sgr_param(p);
                    i += 1;
                }
            }
        }
    }

    #[inline]
    fn param_or_one(v: u16) -> u16 {
        if v == 0 { 1 } else { v }
    }

    // ── Mode handling (DECSET / DECRST) ─────────────────────────────────────

    fn set_private_mode(&mut self, mode: u16, enable: bool) {
        match mode {
            // Mouse tracking (X10/X11 button event).
            1000 => self.mouse_tracking = enable,
            // Focus in/out events.
            1004 => self.focus_events = enable,
            // SGR 1006 mouse encoding.
            1006 => self.mouse_sgr_1006 = enable,
            // Alternate screen + save cursor combo.
            1049 => {
                if enable {
                    self.enter_alt_screen();
                } else {
                    self.exit_alt_screen();
                }
            }
            // Bracketed paste.
            2004 => self.bracketed_paste = enable,
            _ => {}
        }
    }

    fn enter_alt_screen(&mut self) {
        if self.alt_buffer.is_some() {
            return;
        }
        // Save the primary buffer + pen.
        let snap = ScreenSnapshot {
            cols: self.cols,
            rows: self.rows,
            cells: self.cells.clone(),
            cursor: self.cursor,
            fg: self.current_fg,
            bg: self.current_bg,
            attr: self.current_attr,
        };
        self.alt_buffer = Some(snap);
        // Wipe the live grid for the alt buffer.
        for cell in &mut self.cells {
            *cell = Cell::default();
        }
        self.cursor = Cursor::default();
        self.current_fg = DEFAULT_FG;
        self.current_bg = DEFAULT_BG;
        self.current_attr = Attr::empty();
        self.dirty.iter_mut().for_each(|d| *d = true);
    }

    fn exit_alt_screen(&mut self) {
        let Some(snap) = self.alt_buffer.take() else {
            return;
        };
        // Restore primary buffer + pen. The snapshot is sized for the
        // dimensions at save time; if a resize happened in between we
        // truncate / pad to the current grid.
        let want = usize::from(self.cols) * usize::from(self.rows);
        if snap.cells.len() == want {
            self.cells = snap.cells;
        } else {
            // Mismatch — clear then copy the overlap.
            self.cells = vec![Cell::default(); want];
            let copy_cols = usize::from(self.cols.min(snap.cols));
            let copy_rows = usize::from(self.rows.min(snap.rows));
            for r in 0..copy_rows {
                for c in 0..copy_cols {
                    let s = r * usize::from(snap.cols) + c;
                    let d = r * usize::from(self.cols) + c;
                    self.cells[d] = snap.cells[s];
                }
            }
        }
        self.cursor = Cursor {
            row: snap.cursor.row.min(self.rows.saturating_sub(1)),
            col: snap.cursor.col.min(self.cols.saturating_sub(1)),
            visible: snap.cursor.visible,
        };
        self.current_fg = snap.fg;
        self.current_bg = snap.bg;
        self.current_attr = snap.attr;
        self.dirty.iter_mut().for_each(|d| *d = true);
    }

    fn save_cursor(&mut self) {
        self.saved = Some(SavedCursor {
            cursor: self.cursor,
            fg: self.current_fg,
            bg: self.current_bg,
            attr: self.current_attr,
        });
    }

    fn restore_cursor(&mut self) {
        if let Some(s) = self.saved {
            self.cursor = Cursor {
                row: s.cursor.row.min(self.rows.saturating_sub(1)),
                col: s.cursor.col.min(self.cols.saturating_sub(1)),
                visible: s.cursor.visible,
            };
            self.current_fg = s.fg;
            self.current_bg = s.bg;
            self.current_attr = s.attr;
        }
    }
}

// Map an xterm 256-colour index to an RGB triple.
fn palette_256(idx: u16) -> Color {
    match idx {
        0..=15 => ansi_to_color(idx),
        16..=231 => {
            let v = idx - 16;
            let r6 = v / 36;
            let g6 = (v / 6) % 6;
            let b6 = v % 6;
            let level = |n: u16| -> u8 {
                if n == 0 {
                    0
                } else {
                    (55 + n * 40).min(255) as u8
                }
            };
            Color::new(level(r6), level(g6), level(b6))
        }
        232..=255 => {
            let grey = (8 + (idx - 232) * 10).min(255) as u8;
            Color::new(grey, grey, grey)
        }
        _ => DEFAULT_FG,
    }
}

// ── vte::Perform implementation ───────────────────────────────────────────────

impl Perform for TerminalState {
    fn print(&mut self, c: char) {
        self.write_char(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\r' => self.cursor.col = 0,
            b'\n' | 0x0B | 0x0C => {
                self.newline();
            }
            b'\x08' => {
                if self.cursor.col > 0 {
                    self.cursor.col -= 1;
                }
            }
            b'\t' => {
                let next_tab = (self.cursor.col / 8 + 1) * 8;
                self.cursor.col = next_tab.min(self.cols.saturating_sub(1));
            }
            0x07 => {}
            _ => {}
        }
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], _ignore: bool, action: char) {
        let mut param_iter = params.iter();
        let p1: u16 = param_iter
            .next()
            .and_then(|sp| sp.first().copied())
            .unwrap_or(0);
        let p2: u16 = param_iter
            .next()
            .and_then(|sp| sp.first().copied())
            .unwrap_or(0);

        // Private DECSET / DECRST (`CSI ? Ps h` / `CSI ? Ps l`).
        if intermediates.first() == Some(&b'?') {
            match action {
                'h' | 'l' => {
                    let enable = action == 'h';
                    for group in params.iter() {
                        if let Some(&mode) = group.first() {
                            self.set_private_mode(mode, enable);
                        }
                    }
                    return;
                }
                _ => return,
            }
        }

        match action {
            // CUU
            'A' => {
                let n = Self::param_or_one(p1);
                self.cursor.row = self.cursor.row.saturating_sub(n);
            }
            // CUD
            'B' => {
                let n = Self::param_or_one(p1);
                self.cursor.row = (self.cursor.row + n).min(self.rows.saturating_sub(1));
            }
            // CUF
            'C' => {
                let n = Self::param_or_one(p1);
                self.cursor.col = (self.cursor.col + n).min(self.cols.saturating_sub(1));
            }
            // CUB
            'D' => {
                let n = Self::param_or_one(p1);
                self.cursor.col = self.cursor.col.saturating_sub(n);
            }
            // CUP / HVP
            'H' | 'f' => {
                let row = Self::param_or_one(p1).saturating_sub(1);
                let col = Self::param_or_one(p2).saturating_sub(1);
                self.cursor.row = row.min(self.rows.saturating_sub(1));
                self.cursor.col = col.min(self.cols.saturating_sub(1));
            }
            // Focus events: ESC [ I (in) / ESC [ O (out) arrive as CSI sequences.
            'I' => {
                if self.focus_events {
                    self.focus_queue.push(FocusEvent::In);
                }
            }
            'O' => {
                if self.focus_events {
                    self.focus_queue.push(FocusEvent::Out);
                }
            }
            // ED
            'J' => self.erase_display(p1),
            // EL
            'K' => self.erase_line(p1),
            // IL — Insert Line.
            'L' => {
                let n = Self::param_or_one(p1);
                self.insert_lines(n);
            }
            // DL — Delete Line.
            'M' => {
                let n = Self::param_or_one(p1);
                self.delete_lines(n);
            }
            // SGR.
            'm' => self.handle_sgr(params),
            // DECSTBM — Set Top/Bottom Margins.
            'r' => {
                let top = if p1 == 0 { 1 } else { p1 } - 1;
                let bot = if p2 == 0 { self.rows } else { p2 } - 1;
                if top < bot && bot < self.rows {
                    self.scroll_top = top;
                    self.scroll_bot = bot;
                    // DECSTBM also homes the cursor.
                    self.cursor.row = 0;
                    self.cursor.col = 0;
                }
            }
            // Bracketed paste boundaries: ESC [ 200~ / ESC [ 201~ arrive
            // with `~` as the final byte and the parameter encoding the
            // tilde sequence.
            '~' => {
                match p1 {
                    200 => {
                        if self.bracketed_paste {
                            self.paste_active = true;
                            self.paste_queue.push(PasteEvent::Start);
                        }
                    }
                    201 => {
                        if self.bracketed_paste {
                            self.paste_active = false;
                            self.paste_queue.push(PasteEvent::End);
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}

    fn put(&mut self, _byte: u8) {}

    fn unhook(&mut self) {}

    fn osc_dispatch(&mut self, params: &[&[u8]], _bell_terminated: bool) {
        let Some(selector) = params.first() else {
            return;
        };
        match *selector {
            // OSC 0 = set icon name + window title; OSC 2 = window title only.
            b"0" | b"2" => {
                if let Some(title) = osc::decode_title(params) {
                    self.title_queue.push(title);
                }
            }
            // OSC 52 — clipboard set/query.
            b"52" => {
                if let Some(text) = osc::decode_clipboard(params) {
                    self.clipboard_queue.push(text);
                }
            }
            _ => {}
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        // DECSC = ESC 7 ; DECRC = ESC 8 . No intermediates expected.
        if !intermediates.is_empty() {
            return;
        }
        match byte {
            b'7' => self.save_cursor(),
            b'8' => self.restore_cursor(),
            _ => {}
        }
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

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
        let t = make(80, 24, b"A\nB");
        assert_eq!(t.cell(0, 0).ch, 'A');
        assert_eq!(t.cell(1, 1).ch, 'B');
        let t2 = make(80, 24, b"A\r\nB");
        assert_eq!(t2.cell(0, 0).ch, 'A');
        assert_eq!(t2.cell(1, 0).ch, 'B');
    }

    #[test]
    fn cr_returns_to_column_zero() {
        let t = make(80, 24, b"ABC\rX");
        assert_eq!(t.cell(0, 0).ch, 'X');
        assert_eq!(t.cell(0, 1).ch, 'B');
    }

    #[test]
    fn csi_2j_clears_screen() {
        let mut t = make(80, 24, b"Hello");
        t.feed(b"\x1b[2J");
        for cell in t.cells() {
            assert_eq!(cell.ch, ' ');
        }
    }

    #[test]
    fn csi_h_homes_cursor() {
        let mut t = TerminalState::new(80, 24);
        t.feed(b"\x1b[10;20H");
        assert_eq!(t.cursor().row, 9);
        assert_eq!(t.cursor().col, 19);
        t.feed(b"\x1b[H");
        assert_eq!(t.cursor().row, 0);
        assert_eq!(t.cursor().col, 0);
    }

    #[test]
    fn csi_5a_moves_cursor_up_five() {
        let mut t = TerminalState::new(80, 24);
        t.feed(b"\x1b[10;1H");
        assert_eq!(t.cursor().row, 9);
        t.feed(b"\x1b[5A");
        assert_eq!(t.cursor().row, 4);
    }

    #[test]
    fn csi_31m_sets_red_foreground() {
        let mut t = TerminalState::new(80, 24);
        t.feed(b"\x1b[31m");
        t.feed(b"R");
        let cell = t.cell(0, 0);
        assert_eq!(cell.fg, RED);
        assert_eq!(cell.ch, 'R');
    }

    #[test]
    fn csi_1_31m_bold_red() {
        let mut t = TerminalState::new(80, 24);
        t.feed(b"\x1b[1;31m");
        t.feed(b"B");
        let cell = t.cell(0, 0);
        assert_eq!(cell.fg, RED);
        assert!(cell.attr.contains(Attr::BOLD));
    }

    #[test]
    fn dirty_rows_drain_returns_changed_rows_and_resets() {
        let mut t = TerminalState::new(80, 24);
        t.feed(b"A");
        let dirty = t.dirty_rows_drain();
        assert!(dirty.contains(&0));
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
        assert_eq!(t.cursor().col, 2);
    }

    #[test]
    fn sgr_reset_restores_defaults() {
        let mut t = TerminalState::new(80, 24);
        t.feed(b"\x1b[1;31m");
        t.feed(b"\x1b[0m");
        t.feed(b"X");
        let cell = t.cell(0, 0);
        assert_eq!(cell.fg, DEFAULT_FG);
        assert!(!cell.attr.contains(Attr::BOLD));
    }

    // ── Extension tests ──────────────────────────────────────────────────────

    #[test]
    fn alt_screen_enter_restores_on_exit() {
        let mut t = TerminalState::new(20, 5);
        t.feed(b"primary");
        assert_eq!(t.cell(0, 0).ch, 'p');
        assert!(!t.alt_screen_active());

        // DECSET 1049 — enter alt screen.
        t.feed(b"\x1b[?1049h");
        assert!(t.alt_screen_active());
        // Alt buffer starts blank.
        for c in t.cells() {
            assert_eq!(c.ch, ' ');
        }
        // Write into alt buffer.
        t.feed(b"alt");
        assert_eq!(t.cell(0, 0).ch, 'a');

        // DECRST 1049 — exit alt screen, primary content must be back.
        t.feed(b"\x1b[?1049l");
        assert!(!t.alt_screen_active());
        assert_eq!(t.cell(0, 0).ch, 'p');
        assert_eq!(t.cell(0, 1).ch, 'r');
        assert_eq!(t.cell(0, 6).ch, 'y');
    }

    #[test]
    fn mouse_sgr_1006_parses_extended_coords() {
        // Enable SGR 1006 mode via DECSET.
        let mut t = TerminalState::new(80, 24);
        assert!(!t.mouse_sgr_1006_enabled());
        t.feed(b"\x1b[?1000;1006h");
        assert!(t.mouse_tracking_enabled());
        assert!(t.mouse_sgr_1006_enabled());

        // Round-trip an extended coordinate that the legacy X10 protocol
        // cannot represent (cols > 223).
        let ev = MouseEvent {
            x: 320,
            y: 480,
            button: MouseButton::Left,
            kind: MouseEventKind::Press,
            modifiers: MouseModifiers::default(),
        };
        let bytes = encode_sgr_1006(ev);
        assert_eq!(bytes, b"\x1b[<0;320;480M");
        let parsed = decode_sgr_1006(&bytes).expect("decode");
        assert_eq!(parsed, ev);

        // Disable via DECRST.
        t.feed(b"\x1b[?1006l");
        assert!(!t.mouse_sgr_1006_enabled());
    }

    #[test]
    fn truecolor_24bit_fg_bg_parsed() {
        let mut t = TerminalState::new(20, 2);
        // SGR 38;2;R;G;B for foreground; 48;2;R;G;B for background.
        t.feed(b"\x1b[38;2;200;100;50m\x1b[48;2;10;20;30mX");
        let cell = t.cell(0, 0);
        assert_eq!(cell.ch, 'X');
        assert_eq!(cell.fg, Color::new(200, 100, 50));
        assert_eq!(cell.bg, Color::new(10, 20, 30));

        // Sub-parameter (`:`) form must work too.
        let mut t2 = TerminalState::new(20, 2);
        t2.feed(b"\x1b[38:2:1:2:3mY");
        assert_eq!(t2.cell(0, 0).fg, Color::new(1, 2, 3));
    }

    #[test]
    fn cursor_save_restore_preserves_sgr() {
        let mut t = TerminalState::new(40, 10);
        // Move + set red bold, save (DECSC = ESC 7).
        t.feed(b"\x1b[5;15H\x1b[1;31m\x1b7");
        // Move elsewhere and switch to default attrs.
        t.feed(b"\x1b[1;1H\x1b[0m");
        assert_eq!(t.cursor().row, 0);
        assert_eq!(t.cursor().col, 0);
        assert_eq!(t.current_fg(), DEFAULT_FG);
        // Restore (DECRC = ESC 8).
        t.feed(b"\x1b8");
        assert_eq!(t.cursor().row, 4);
        assert_eq!(t.cursor().col, 14);
        assert_eq!(t.current_fg(), RED);
        assert!(t.current_attr().contains(Attr::BOLD));
    }

    #[test]
    fn bracketed_paste_wraps_input() {
        let mut t = TerminalState::new(80, 24);
        assert!(!t.bracketed_paste_enabled());
        t.feed(b"\x1b[?2004h");
        assert!(t.bracketed_paste_enabled());
        assert!(!t.paste_active());
        // Paste start.
        t.feed(b"\x1b[200~");
        assert!(t.paste_active());
        // Pasted text is still rendered.
        t.feed(b"PASTED");
        assert_eq!(t.cell(0, 0).ch, 'P');
        // Paste end.
        t.feed(b"\x1b[201~");
        assert!(!t.paste_active());

        let events = t.drain_paste_events();
        assert_eq!(events, vec![PasteEvent::Start, PasteEvent::End]);
        // Second drain is empty.
        assert!(t.drain_paste_events().is_empty());

        // After disabling, the boundary sequences must not flip the flag.
        t.feed(b"\x1b[?2004l");
        assert!(!t.bracketed_paste_enabled());
        t.feed(b"\x1b[200~");
        assert!(!t.paste_active());
    }

    #[test]
    fn focus_events_in_out_emitted() {
        let mut t = TerminalState::new(80, 24);
        // Without enabling, CSI I / O must be ignored.
        t.feed(b"\x1b[I\x1b[O");
        assert!(t.drain_focus_events().is_empty());

        t.feed(b"\x1b[?1004h");
        assert!(t.focus_events_enabled());
        t.feed(b"\x1b[I"); // focus in
        t.feed(b"\x1b[O"); // focus out
        let evs = t.drain_focus_events();
        assert_eq!(evs, vec![FocusEvent::In, FocusEvent::Out]);
        assert!(t.drain_focus_events().is_empty());

        t.feed(b"\x1b[?1004l");
        assert!(!t.focus_events_enabled());
    }

    #[test]
    fn decstbm_constrains_scroll_region() {
        let mut t = TerminalState::new(10, 10);
        // Default region spans the entire grid.
        assert_eq!(t.scroll_region(), (0, 9));
        // Set DECSTBM to rows 3..=6 (1-based 4;7).
        t.feed(b"\x1b[4;7r");
        assert_eq!(t.scroll_region(), (3, 6));
        // DECSTBM homes the cursor.
        assert_eq!(t.cursor().row, 0);
        assert_eq!(t.cursor().col, 0);

        // Move cursor into the region and fill it.
        t.feed(b"\x1b[4;1H"); // row 4 (=index 3)
        t.feed(b"A\r\nB\r\nC\r\nD");
        // 'A' at row 3, 'B' at row 4, 'C' at row 5, 'D' at row 6 (region bottom).
        assert_eq!(t.cell(3, 0).ch, 'A');
        assert_eq!(t.cell(6, 0).ch, 'D');
        // Force a scroll: emit two LFs while at the bottom of the region.
        t.feed(b"\r\nE");
        // Region must have scrolled: 'A' is gone, 'B' is now at row 3.
        assert_eq!(t.cell(3, 0).ch, 'B');
        assert_eq!(t.cell(6, 0).ch, 'E');
    }

    #[test]
    fn insert_delete_line_in_region() {
        let mut t = TerminalState::new(5, 6);
        // Fill rows 0..6 with marker chars.
        for i in 0..6_u16 {
            t.feed(format!("\x1b[{};1H", i + 1).as_bytes());
            let ch: &[u8] = match i {
                0 => b"A", 1 => b"B", 2 => b"C", 3 => b"D", 4 => b"E", _ => b"F",
            };
            t.feed(ch);
        }
        // Restrict region to rows 1..=4 (1-based 2;5).
        t.feed(b"\x1b[2;5r");
        // Move to row 2 (index 1) and insert 1 line.
        t.feed(b"\x1b[2;1H\x1b[L");
        // After IL: row 1 is blank, B moved to row 2, C to row 3, D pushed
        // off the region bottom (lost; E remains at row 4 because E is outside).
        assert_eq!(t.cell(1, 0).ch, ' ');
        assert_eq!(t.cell(2, 0).ch, 'B');
        assert_eq!(t.cell(3, 0).ch, 'C');
        // Row 4 is the region bottom — D landed here.
        assert_eq!(t.cell(4, 0).ch, 'D');
        // Row 5 is outside the region and must be untouched.
        assert_eq!(t.cell(5, 0).ch, 'F');

        // Now delete a line at row 2 (index 1).
        t.feed(b"\x1b[2;1H\x1b[M");
        // Row 1: B (from row 2 sliding up); row 2: C; row 3: D; row 4: blank.
        assert_eq!(t.cell(1, 0).ch, 'B');
        assert_eq!(t.cell(2, 0).ch, 'C');
        assert_eq!(t.cell(3, 0).ch, 'D');
        assert_eq!(t.cell(4, 0).ch, ' ');
    }

    #[test]
    fn osc_0_window_title_set() {
        let mut t = TerminalState::new(80, 24);
        // BEL terminator.
        t.feed(b"\x1b]0;hello world\x07");
        let titles = t.drain_window_titles();
        assert_eq!(titles, vec!["hello world".to_string()]);
        assert!(t.drain_window_titles().is_empty());

        // ST terminator (ESC \).
        t.feed(b"\x1b]2;second\x1b\\");
        assert_eq!(t.drain_window_titles(), vec!["second".to_string()]);
    }

    #[test]
    fn osc_52_clipboard_set_decoded() {
        let mut t = TerminalState::new(80, 24);
        // base64("copy this") = "Y29weSB0aGlz"
        t.feed(b"\x1b]52;c;Y29weSB0aGlz\x07");
        let clip = t.drain_clipboard_set();
        assert_eq!(clip, vec!["copy this".to_string()]);
        assert!(t.drain_clipboard_set().is_empty());

        // Query form (`?`) must NOT queue a set.
        t.feed(b"\x1b]52;c;?\x07");
        assert!(t.drain_clipboard_set().is_empty());
    }
}
