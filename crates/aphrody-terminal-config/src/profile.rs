// SPDX-License-Identifier: Apache-2.0
//! Terminal profile primitives (font, cursor, M3 palette).
//!
//! A [`Profile`] captures the *visible* surface of a terminal session: the
//! font face used by the renderer, its point size, the cursor shape, and the
//! Material Design 3 baseline palette that drives ANSI 16-color mapping +
//! background / surface tints.
//!
//! Palette values are sourced from the M3 baseline reference scheme (Purple
//! seed `#6750A4`).  Mirrored from `crates/m3-tokens/src/color.rs` so this
//! crate stays free of a path-dep on `m3-tokens` (config layer ↛ token layer).
//! When `m3-tokens` ships a new baseline, bump both in lock-step.
//!
//! # Strictness
//!
//! All structs derive `Serialize + Deserialize + JsonSchema` with
//! `deny_unknown_fields` so a malformed `profiles[]` entry in
//! `~/.aphrody/terminal.json` is rejected at load time instead of silently
//! losing data.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// ── CursorShape ───────────────────────────────────────────────────────────────

/// Cursor rendering style (matches the VT 5.5 DECSCUSR shapes).
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum CursorShape {
    /// Full-cell block.
    Block,
    /// Vertical bar (I-beam).
    Bar,
    /// Horizontal underline.
    Underline,
}

impl Default for CursorShape {
    fn default() -> Self {
        Self::Bar
    }
}

// ── PaletteM3 ─────────────────────────────────────────────────────────────────

/// Material Design 3 baseline palette (ARGB `u32`, big-endian alpha-first).
///
/// We expose only the subset of M3 roles the terminal actually consumes:
/// `primary` (cursor + selection), `on_primary` (selected text), `background`
/// (default cell bg), `on_background` (default cell fg), `surface` (status bar
/// bg), `error` (red ANSI).  Other roles can be added without breaking the
/// schema — they are deserialised as `None` when absent.
#[derive(Clone, Copy, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PaletteM3 {
    /// `--md-sys-color-primary` (ARGB).
    pub primary: u32,
    /// `--md-sys-color-on-primary` (ARGB).
    pub on_primary: u32,
    /// `--md-sys-color-background` (ARGB).
    pub background: u32,
    /// `--md-sys-color-on-background` (ARGB).
    pub on_background: u32,
    /// `--md-sys-color-surface` (ARGB).
    pub surface: u32,
    /// `--md-sys-color-error` (ARGB).
    pub error: u32,
}

/// M3 baseline-dark primary (purple seed `#6750A4` → tone 80).
///
/// Mirrors `m3_tokens::color::BASELINE_DARK.primary`.  Asserted in the
/// `default_profile_is_m3_dark` integration test.
pub const M3_BASELINE_DARK_PRIMARY: u32 = 0xFFD0_BCFF;

impl PaletteM3 {
    /// M3 baseline-dark palette (Purple seed `#6750A4`).
    ///
    /// Mirrors `m3_tokens::color::BASELINE_DARK`.
    #[must_use]
    pub const fn baseline_dark() -> Self {
        Self {
            primary: M3_BASELINE_DARK_PRIMARY,
            on_primary: 0xFF38_1E72,
            background: 0xFF14_1218,
            on_background: 0xFFE6_E0E9,
            surface: 0xFF14_1218,
            error: 0xFFF2_B8B5,
        }
    }
}

impl Default for PaletteM3 {
    fn default() -> Self {
        Self::baseline_dark()
    }
}

// ── Profile ───────────────────────────────────────────────────────────────────

/// A named terminal profile.
///
/// Profiles are stored in [`crate::TerminalConfig::profiles`] keyed by
/// [`Profile::name`]; one of them is referenced as the active session by
/// [`crate::TerminalConfig::default_profile`].
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    /// Profile identifier (must be unique within a [`crate::TerminalConfig`]).
    pub name: String,
    /// Font family CSS-style string (e.g. `"Cascadia Code"`).
    pub font_family: String,
    /// Font size in points.
    pub font_size: u16,
    /// Cursor shape.
    #[serde(default)]
    pub cursor_shape: CursorShape,
    /// Material Design 3 palette.
    #[serde(default)]
    pub palette: PaletteM3,
}

impl Default for Profile {
    fn default() -> Self {
        default_profile()
    }
}

/// Build the baseline-dark M3 profile used when no user override exists.
///
/// Font / size match the aphrody-terminal-vt golden defaults (Cascadia Code,
/// 14 pt) and the cursor shape is the LLM-friendly bar (smallest vertical
/// footprint, easiest to spot in dense output panes).
#[must_use]
pub fn default_profile() -> Profile {
    Profile {
        name: "default".to_owned(),
        font_family: "Cascadia Code".to_owned(),
        font_size: 14,
        cursor_shape: CursorShape::Bar,
        palette: PaletteM3::baseline_dark(),
    }
}
