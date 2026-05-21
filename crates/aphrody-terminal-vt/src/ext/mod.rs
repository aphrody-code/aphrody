// SPDX-License-Identifier: Apache-2.0
//! Standalone VT extension parsers for Ink/React TUI compatibility.
//!
//! Where the top-level [`crate::TerminalState`] threads everything through a
//! single `vte::Perform` state machine, the helpers in this module are
//! **stateless decoders** that exist so external integrators (Ink/React
//! TUIs, headless test rigs, the LLM bridge, the JSON-out filter) can pull
//! a single VT extension off the wire without spinning up a full screen
//! buffer.
//!
//! Each sub-module owns exactly one of the five "Ink-critical" extensions
//! identified in `docs/PLAN.md` §0.5 item #1:
//!
//! - [`alt_screen`] — alternate screen buffer (DECSET 1049 / 47 / 1047).
//! - [`mouse_sgr_1006`] — `CSI <` mouse encoding with motion + modifier flags.
//! - [`osc_52`] — clipboard get/set (RFC tmux extension, xterm OSC 52).
//! - [`bracketed_paste`] — DECSET 2004 + paste boundary accumulator.
//! - [`decstbm`] — DEC top/bottom scroll margins parser.
//!
//! These mirror semantics established by the reference parsers in
//! wterm's `packages/vt-decoder` (Vercel) and Windows Terminal's
//! `src/terminal/parser` (Microsoft) — read for algorithmic guidance only;
//! the Microsoft sources are never linked on Linux/WASM per the project's
//! terminal-integration policy.

pub mod alt_screen;
pub mod bracketed_paste;
pub mod decstbm;
pub mod mouse_sgr_1006;
pub mod osc_52;

pub use alt_screen::AltScreenEvent;
pub use bracketed_paste::{BracketedPasteState, PasteAccumulator};
pub use decstbm::ScrollMargins;
pub use mouse_sgr_1006::{SgrMouseEvent, SgrMouseKind};
pub use osc_52::{ClipboardKind, Osc52};
