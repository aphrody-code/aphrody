// SPDX-License-Identifier: Apache-2.0
//! Markdown theme tokens — sourced from the workspace's M3 baseline palette.
//!
//! Each renderable region (body text, headings, fenced code, inline code,
//! links, emphasis) maps to a 24-bit RGB triple. The mapping is intentionally
//! deterministic so the same input always produces byte-identical output.
//!
//! Source palettes:
//! - [`m3_tokens::color::BASELINE`]      → light theme.
//! - [`m3_tokens::color::BASELINE_DARK`] → dark theme.
//!
//! Role mapping (M3 → markdown):
//!
//! | Markdown region        | M3 role               |
//! |------------------------|-----------------------|
//! | body text              | `on_surface`          |
//! | heading                | `primary`             |
//! | fenced code body       | `on_secondary_container` |
//! | fenced code background | `secondary_container` |
//! | inline link            | `tertiary`            |
//! | emphasised text        | `secondary`           |
//!
//! Helpers [`fg_sgr`] and [`bg_sgr`] convert an [`Rgb`] to its
//! `ESC[38;2;R;G;B m` / `ESC[48;2;R;G;B m` ANSI escape, ready to splice
//! into a rendered buffer.

use m3_tokens::color::{BASELINE, BASELINE_DARK, ColorRoles};

/// 24-bit RGB colour (each channel 0..=255).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rgb {
    /// Red channel (0..=255).
    pub r: u8,
    /// Green channel (0..=255).
    pub g: u8,
    /// Blue channel (0..=255).
    pub b: u8,
}

impl Rgb {
    /// Build an [`Rgb`] from an ARGB `u32` (M3 token format, alpha discarded).
    #[must_use]
    pub const fn from_argb(argb: u32) -> Self {
        Self {
            r: ((argb >> 16) & 0xFF) as u8,
            g: ((argb >> 8) & 0xFF) as u8,
            b: (argb & 0xFF) as u8,
        }
    }
}

/// Emit the ANSI SGR introducer for a 24-bit foreground colour:
/// `ESC [ 38 ; 2 ; R ; G ; B m`.
#[must_use]
pub fn fg_sgr(c: Rgb) -> String {
    format!("\x1b[38;2;{};{};{}m", c.r, c.g, c.b)
}

/// Emit the ANSI SGR introducer for a 24-bit background colour:
/// `ESC [ 48 ; 2 ; R ; G ; B m`.
#[must_use]
pub fn bg_sgr(c: Rgb) -> String {
    format!("\x1b[48;2;{};{};{}m", c.r, c.g, c.b)
}

/// Markdown theme: one colour per renderable region.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MdTheme {
    /// Body text.
    pub fg_text: Rgb,
    /// Heading colour (all levels share the same hue; weight differentiates).
    pub fg_heading: Rgb,
    /// Code body (inside fenced ``` blocks) when syntect highlighting is
    /// disabled, or the fallback foreground for unknown languages.
    pub fg_code: Rgb,
    /// Background swatch for fenced code blocks.
    pub bg_code_block: Rgb,
    /// Foreground colour for hyperlinks (underlined separately).
    pub fg_link: Rgb,
    /// Foreground colour for emphasised inline runs (`*…*`, `_…_`).
    pub fg_emphasis: Rgb,
}

impl MdTheme {
    /// Markdown theme derived from the M3 dark palette (`BASELINE_DARK`).
    #[must_use]
    pub const fn from_m3_dark() -> Self {
        Self::from_roles(&BASELINE_DARK)
    }

    /// Markdown theme derived from the M3 light palette (`BASELINE`).
    #[must_use]
    pub const fn from_m3_light() -> Self {
        Self::from_roles(&BASELINE)
    }

    /// Build a theme from an arbitrary [`ColorRoles`] palette.
    #[must_use]
    pub const fn from_roles(p: &ColorRoles) -> Self {
        Self {
            fg_text: Rgb::from_argb(p.on_surface),
            fg_heading: Rgb::from_argb(p.primary),
            fg_code: Rgb::from_argb(p.on_secondary_container),
            bg_code_block: Rgb::from_argb(p.secondary_container),
            fg_link: Rgb::from_argb(p.tertiary),
            fg_emphasis: Rgb::from_argb(p.secondary),
        }
    }
}

impl Default for MdTheme {
    fn default() -> Self {
        Self::from_m3_dark()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argb_strips_alpha() {
        let c = Rgb::from_argb(0xFF6750A4);
        assert_eq!(c, Rgb { r: 0x67, g: 0x50, b: 0xA4 });
    }

    #[test]
    fn dark_and_light_themes_differ() {
        let d = MdTheme::from_m3_dark();
        let l = MdTheme::from_m3_light();
        assert_ne!(d.fg_text, l.fg_text);
        assert_ne!(d.fg_heading, l.fg_heading);
    }

    #[test]
    fn fg_sgr_format() {
        let s = fg_sgr(Rgb { r: 12, g: 34, b: 56 });
        assert_eq!(s, "\x1b[38;2;12;34;56m");
    }

    #[test]
    fn bg_sgr_format() {
        let s = bg_sgr(Rgb { r: 200, g: 100, b: 50 });
        assert_eq!(s, "\x1b[48;2;200;100;50m");
    }
}
