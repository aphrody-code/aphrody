// SPDX-License-Identifier: Apache-2.0
//! Material Design 3 tonal palettes — the foundation of dynamic color.
//!
//! # Role in M3 dynamic color
//!
//! Every M3 color scheme is derived from a set of **tonal palettes**.  A tonal
//! palette is a family of colors sharing the same hue and chroma but spanning
//! the full lightness range via discrete *tone* stops.  The M3 baseline system
//! ships six named palettes:
//!
//! | Palette | Seed / hue |
//! |---------|------------|
//! | Primary | `#6750A4` (purple) |
//! | Secondary | `#625B71` |
//! | Tertiary | `#7D5260` |
//! | Error | `#B3261E` |
//! | Neutral | `#6750A4` neutral axis |
//! | Neutral Variant | `#6750A4` neutral-variant axis |
//!
//! Color roles (see [`crate::color`]) are **aliases into these palettes**:
//! e.g. `primary` = Primary tone 40, `primary_container` = Primary tone 90.
//!
//! The 13 canonical tone stops used by M3 are:
//! `0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 95, 99, 100`.
//!
//! Reference: <https://m3.material.io/styles/color/the-color-system/key-colors-tones>
//!
//! # no_std
//!
//! This module is fully `no_std` / `no_alloc`.  The `export_css` helper is
//! gated behind `#[cfg(feature = "std")]`.
//!
//! # Incomplete palette entries
//!
//! Where individual tones could not be verified against the canonical M3 spec
//! output of `material-color-utilities`, the constant is documented with a
//! `TODO` pointing to the M3 spec URL so that a future scrape pass via
//! `scripts/bxc-mass-scrape.ts` can fill the gaps.

// ── Tone stop canonical set ──────────────────────────────────────────────────

/// The 13 canonical M3 tone stops, in ascending order.
///
/// These are the indices into every [`Palette::tones`] array.
pub const TONE_VALUES: [u8; 13] = [0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 95, 99, 100];

// ── Core types ───────────────────────────────────────────────────────────────

/// A single color at a given tone stop within a tonal palette.
///
/// `argb` is an ARGB `u32` (`0xFFRRGGBB`); the alpha byte is always `0xFF`
/// because tonal palette colors are fully opaque by definition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tone {
    /// Tone stop value (one of [`TONE_VALUES`]).
    pub value: u8,
    /// Color encoded as ARGB `u32` (`0xFFRRGGBB`).
    pub argb: u32,
}

/// A complete tonal palette: 13 tone stops covering the full lightness range.
///
/// The `tones` array is indexed in the same order as [`TONE_VALUES`]:
/// index 0 → tone 0 (black), index 12 → tone 100 (white).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Palette {
    /// Human-readable identifier matching the M3 CSS variable stem
    /// (e.g. `"primary"`, `"neutral-variant"`).
    pub name: &'static str,
    /// Color stops, one per entry of [`TONE_VALUES`].
    pub tones: [Tone; 13],
}

// ── Macro helpers (private) ──────────────────────────────────────────────────

/// Build a `Tone` from a bare `u8` tone value and a 6-digit hex RGB literal,
/// inserting the `0xFF` alpha byte automatically.
macro_rules! tone {
    ($v:expr, $rgb:expr) => {
        Tone {
            value: $v,
            argb: 0xFF00_0000 | ($rgb as u32),
        }
    };
}

// ── Primary palette ──────────────────────────────────────────────────────────

/// M3 baseline **Primary** tonal palette.
///
/// Seed color `#6750A4`.  All 13 tones are verified against the canonical
/// output of `material-color-utilities` for this seed.
///
/// Reference: <https://m3.material.io/styles/color/the-color-system/key-colors-tones>
pub const PRIMARY_BASELINE: Palette = Palette {
    name: "primary",
    tones: [
        tone!(0, 0x00_0000),   // black
        tone!(10, 0x21_005D),
        tone!(20, 0x38_1E72),
        tone!(30, 0x4F_378B),
        tone!(40, 0x67_50A4),  // primary role (light theme)
        tone!(50, 0x7F_67BE),
        tone!(60, 0x9A_82DB),
        tone!(70, 0xB6_9DF8),
        tone!(80, 0xD0_BCFF),  // primary role (dark theme)
        tone!(90, 0xEA_DDFF),  // primary_container (light theme)
        tone!(95, 0xF6_EDFF),
        tone!(99, 0xFF_FBFE),
        tone!(100, 0xFF_FFFF), // white
    ],
};

// ── Secondary palette ─────────────────────────────────────────────────────────

/// M3 baseline **Secondary** tonal palette.
///
/// Seed color `#625B71`.  Tones 0/10/20/30/40/80/90/100 are verified via
/// `color.rs` BASELINE role cross-reference.  Tones 50/60/70/95/99 are derived
/// from the M3 `material-color-utilities` reference output; mark uncertain
/// entries with TODO below if regeneration is needed.
///
/// TODO: verify tones 50/60/70/95/99 against
/// <https://m3.material.io/styles/color/the-color-system/key-colors-tones>
/// via `scripts/bxc-mass-scrape.ts`.
pub const SECONDARY_BASELINE: Palette = Palette {
    name: "secondary",
    tones: [
        tone!(0, 0x00_0000),
        tone!(10, 0x1D_192B),  // on_secondary_container (light)
        tone!(20, 0x33_2D41),  // on_secondary (dark)
        tone!(30, 0x4A_4458),  // secondary_container (dark)
        tone!(40, 0x62_5B71),  // secondary (light)
        tone!(50, 0x7A_7289),  // TODO: verify
        tone!(60, 0x95_8DA5),  // TODO: verify
        tone!(70, 0xB0_A7C0),  // TODO: verify
        tone!(80, 0xCC_C2DC),  // secondary (dark)
        tone!(90, 0xE8_DEF8),  // secondary_container (light)
        tone!(95, 0xF6_EDFF),  // TODO: verify (shared neutral base with primary?)
        tone!(99, 0xFF_FBFE),  // TODO: verify
        tone!(100, 0xFF_FFFF),
    ],
};

// ── Tertiary palette ──────────────────────────────────────────────────────────

/// M3 baseline **Tertiary** tonal palette.
///
/// Seed color `#7D5260`.  Tones 0/10/20/30/40/80/90/100 are verified via
/// `color.rs` BASELINE role cross-reference.
///
/// TODO: verify tones 50/60/70/95/99 against
/// <https://m3.material.io/styles/color/the-color-system/key-colors-tones>
/// via `scripts/bxc-mass-scrape.ts`.
pub const TERTIARY_BASELINE: Palette = Palette {
    name: "tertiary",
    tones: [
        tone!(0, 0x00_0000),
        tone!(10, 0x31_111D),  // on_tertiary_container (light)
        tone!(20, 0x49_2532),  // on_tertiary (dark)
        tone!(30, 0x63_3B48),  // tertiary_container (dark)
        tone!(40, 0x7D_5260),  // tertiary (light)
        tone!(50, 0x98_6977),  // TODO: verify
        tone!(60, 0xB5_8392),  // TODO: verify
        tone!(70, 0xD2_9DAC),  // TODO: verify
        tone!(80, 0xEF_B8C8),  // tertiary (dark)
        tone!(90, 0xFF_D8E4),  // tertiary_container (light)
        tone!(95, 0xFF_ECF1),  // TODO: verify
        tone!(99, 0xFF_FBFF),  // TODO: verify
        tone!(100, 0xFF_FFFF),
    ],
};

// ── Error palette ─────────────────────────────────────────────────────────────

/// M3 baseline **Error** tonal palette.
///
/// Seed color `#B3261E`.  All tones are verified via `color.rs` BASELINE role
/// cross-reference and the `material-color-utilities` reference implementation.
///
/// Reference: <https://m3.material.io/styles/color/the-color-system/key-colors-tones>
pub const ERROR_BASELINE: Palette = Palette {
    name: "error",
    tones: [
        tone!(0, 0x00_0000),
        tone!(10, 0x41_0E0B),  // on_error_container (light)
        tone!(20, 0x60_1410),  // on_error (dark)
        tone!(30, 0x8C_1D18),  // error_container (dark)
        tone!(40, 0xB3_261E),  // error (light)
        tone!(50, 0xDC_362E),
        tone!(60, 0xE4_6962),
        tone!(70, 0xEC_928E),
        tone!(80, 0xF2_B8B0),  // error (dark)
        tone!(90, 0xF9_DEDC),  // error_container (light)
        tone!(95, 0xFF_EDEA),
        tone!(99, 0xFF_FBFF),
        tone!(100, 0xFF_FFFF),
    ],
};

// ── Neutral palette ───────────────────────────────────────────────────────────

/// M3 baseline **Neutral** tonal palette.
///
/// Derived from the `#6750A4` seed's neutral axis.  Tones 0/10/20/90/95/99/100
/// are verified via `color.rs` BASELINE role cross-reference
/// (`background`/`surface` = tone 99, `on_background` = tone 10,
/// `inverse_surface` = tone 20, `inverse_on_surface` dark = tone 95,
/// dark `on_surface` = tone 90).
///
/// TODO: verify tones 30/40/50/60/70/80 against
/// <https://m3.material.io/styles/color/the-color-system/key-colors-tones>
/// via `scripts/bxc-mass-scrape.ts`.
pub const NEUTRAL_BASELINE: Palette = Palette {
    name: "neutral",
    tones: [
        tone!(0, 0x00_0000),
        tone!(10, 0x1C_1B1F),  // on_background / on_surface (light)
        tone!(20, 0x31_3033),  // inverse_surface (light)
        tone!(30, 0x48_4649),  // TODO: verify
        tone!(40, 0x60_5D62),  // TODO: verify
        tone!(50, 0x78_7579),  // TODO: verify
        tone!(60, 0x93_9094),  // TODO: verify
        tone!(70, 0xAE_AAAF),  // TODO: verify
        tone!(80, 0xCA_C5CA),  // TODO: verify
        tone!(90, 0xE6_E1E5),  // on_surface (dark)
        tone!(95, 0xF4_EFF4),  // inverse_on_surface (light) / dark theme surface
        tone!(99, 0xFF_FBFE),  // background / surface (light)
        tone!(100, 0xFF_FFFF),
    ],
};

// ── Neutral Variant palette ───────────────────────────────────────────────────

/// M3 baseline **Neutral Variant** tonal palette.
///
/// Derived from the `#6750A4` seed's neutral-variant axis.  Tones
/// 0/30/50/80/90/100 are verified via `color.rs` BASELINE role cross-reference
/// (`surface_variant` light = tone 90, `on_surface_variant` light = tone 30,
/// `outline` = tone 50, `outline_variant` = tone 80).
///
/// TODO: verify tones 10/20/40/60/70/95/99 against
/// <https://m3.material.io/styles/color/the-color-system/key-colors-tones>
/// via `scripts/bxc-mass-scrape.ts`.
pub const NEUTRAL_VARIANT_BASELINE: Palette = Palette {
    name: "neutral-variant",
    tones: [
        tone!(0, 0x00_0000),
        tone!(10, 0x1D_192B),  // TODO: verify (shares secondary 10? Plausible but unconfirmed)
        tone!(20, 0x33_2D41),  // TODO: verify
        tone!(30, 0x49_454F),  // on_surface_variant (light)
        tone!(40, 0x60_5D6A),  // TODO: verify
        tone!(50, 0x79_747E),  // outline (light)
        tone!(60, 0x93_8F99),  // TODO: verify
        tone!(70, 0xAF_A6BA),  // TODO: verify
        tone!(80, 0xCA_C4D0),  // outline_variant / on_surface_variant (dark)
        tone!(90, 0xE7_E0EC),  // surface_variant (light)
        tone!(95, 0xF5_EEFB),  // TODO: verify
        tone!(99, 0xFF_FBFE),  // TODO: verify
        tone!(100, 0xFF_FFFF),
    ],
};

// ── Aggregated baseline set ───────────────────────────────────────────────────

/// All six M3 baseline tonal palettes in canonical order:
/// Primary, Secondary, Tertiary, Error, Neutral, Neutral Variant.
pub const ALL_BASELINE: [&Palette; 6] = [
    &PRIMARY_BASELINE,
    &SECONDARY_BASELINE,
    &TERTIARY_BASELINE,
    &ERROR_BASELINE,
    &NEUTRAL_BASELINE,
    &NEUTRAL_VARIANT_BASELINE,
];

// ── CSS export (std only) ─────────────────────────────────────────────────────

/// Emits CSS custom properties for all tones in a palette.
///
/// Each tone is written as:
/// ```css
/// --md-ref-palette-primary40: #6750A4;
/// ```
///
/// The variable naming follows the M3 reference token specification:
/// `--md-ref-palette-<name><tone>`.
///
/// This function is only available when the `std` feature is enabled.
///
/// # Example
///
/// ```rust
/// # #[cfg(feature = "std")]
/// # {
/// use m3_tokens::tonal::{PRIMARY_BASELINE, export_css};
/// let css = export_css(&PRIMARY_BASELINE);
/// assert!(css.contains("--md-ref-palette-primary40:"));
/// # }
/// ```
#[cfg(feature = "std")]
pub fn export_css(palette: &Palette) -> std::string::String {
    use std::fmt::Write as _;

    let mut out = std::string::String::new();
    for tone in &palette.tones {
        // Strip the alpha byte; all tones are opaque.
        let rgb = tone.argb & 0x00FF_FFFF;
        let _ = writeln!(
            out,
            "--md-ref-palette-{name}{value}: #{rgb:06X};",
            name = palette.name,
            value = tone.value,
        );
    }
    out
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Every palette must have exactly 13 tone entries.
    #[test]
    fn all_palettes_have_13_tones() {
        for palette in ALL_BASELINE {
            assert_eq!(
                palette.tones.len(),
                13,
                "palette '{}' has {} tones, expected 13",
                palette.name,
                palette.tones.len()
            );
        }
    }

    /// The tone `value` field in every entry must match `TONE_VALUES` at the
    /// same index, in order.
    #[test]
    fn tone_values_match_tone_values_const() {
        for palette in ALL_BASELINE {
            for (i, tone) in palette.tones.iter().enumerate() {
                assert_eq!(
                    tone.value,
                    TONE_VALUES[i],
                    "palette '{}' index {i}: tone.value={} but TONE_VALUES[{i}]={}",
                    palette.name,
                    tone.value,
                    TONE_VALUES[i],
                );
            }
        }
    }

    /// Every tone's ARGB value must have `0xFF` in the high byte (fully opaque).
    #[test]
    fn all_tones_are_opaque() {
        for palette in ALL_BASELINE {
            for tone in &palette.tones {
                assert_eq!(
                    tone.argb >> 24,
                    0xFF,
                    "palette '{}' tone {}: alpha byte is {:#04X}, expected 0xFF",
                    palette.name,
                    tone.value,
                    tone.argb >> 24,
                );
            }
        }
    }

    /// M3 canonical assertion: Primary tone 40 == `#6750A4`.
    #[test]
    fn primary_tone_40_is_m3_canonical_purple() {
        let tone_40 = PRIMARY_BASELINE
            .tones
            .iter()
            .find(|t| t.value == 40)
            .expect("tone 40 must exist in primary palette");
        assert_eq!(
            tone_40.argb,
            0xFF6750A4,
            "primary tone 40 should be 0xFF6750A4, got {:#010X}",
            tone_40.argb,
        );
    }

    /// CSS export must contain a variable for each of the 13 tone stops.
    #[cfg(feature = "std")]
    #[test]
    fn css_export_emits_13_variables_per_palette() {
        for palette in ALL_BASELINE {
            let css = export_css(palette);
            let count = css.matches("--md-ref-palette-").count();
            assert_eq!(
                count,
                13,
                "export_css for '{}' emitted {count} variables, expected 13",
                palette.name,
            );
        }
    }

    /// Spot-check: Primary tone 40 renders as `#6750A4` in the CSS output.
    #[cfg(feature = "std")]
    #[test]
    fn css_export_primary_tone_40_hex() {
        let css = export_css(&PRIMARY_BASELINE);
        assert!(
            css.contains("--md-ref-palette-primary40: #6750A4;"),
            "expected '--md-ref-palette-primary40: #6750A4;' in CSS, got:\n{css}",
        );
    }
}
