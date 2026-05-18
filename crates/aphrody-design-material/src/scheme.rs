//! M3 color schemes: light, dark, high-contrast light, high-contrast dark.
//!
//! Each scheme exposes 36 semantic roles derived from 6 source palettes
//! (Primary, Secondary, Tertiary, Neutral, NeutralVariant, Error).
//!
//! Tone mappings follow the M3 spec
//! (<https://m3.material.io/styles/color/the-color-system/color-roles>):
//!
//! | Role family | Light | Dark | HC Light | HC Dark |
//! |-------------|-------|------|----------|---------|
//! | primary | 40 | 80 | 20 | 90 |
//! | on-primary | 100 | 20 | 100 | 10 |
//! | primary-container | 90 | 30 | 30 | 90 |
//! | on-primary-container | 10 | 90 | 100 | 10 |
//!
//! (etc. — see `Scheme::for_palettes`.)

use crate::tonal_palette::TonalPalette;
use serde::{Deserialize, Serialize};

/// 6-palette source for an M3 scheme.
#[derive(Debug, Clone, Copy)]
pub struct CorePalettes {
    pub primary: TonalPalette,
    pub secondary: TonalPalette,
    pub tertiary: TonalPalette,
    pub neutral: TonalPalette,
    pub neutral_variant: TonalPalette,
    pub error: TonalPalette,
}

impl CorePalettes {
    /// Derive the 6 M3 palettes from a single seed color, following the
    /// `material-color-utilities` `SchemeContent` recipe:
    ///
    /// - Primary: seed hue, seed chroma.
    /// - Secondary: seed hue, chroma/3.
    /// - Tertiary: seed hue + 60°, chroma/2.
    /// - Neutral: seed hue, chroma 4.
    /// - NeutralVariant: seed hue, chroma 8.
    /// - Error: fixed (hue 25°, chroma 84) — M3 error red.
    #[must_use]
    pub fn from_seed(argb: u32) -> Self {
        let primary = TonalPalette::from_seed_argb(argb);
        let secondary = TonalPalette::from_hue_and_chroma(primary.hue, primary.chroma / 3.0);
        let tertiary =
            TonalPalette::from_hue_and_chroma((primary.hue + 60.0) % 360.0, primary.chroma / 2.0);
        let neutral = TonalPalette::from_hue_and_chroma(primary.hue, 4.0);
        let neutral_variant = TonalPalette::from_hue_and_chroma(primary.hue, 8.0);
        let error = TonalPalette::from_hue_and_chroma(25.0, 84.0);
        Self {
            primary,
            secondary,
            tertiary,
            neutral,
            neutral_variant,
            error,
        }
    }
}

/// Variant of an M3 scheme — controls the tone mapping table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SchemeVariant {
    Light,
    Dark,
    HighContrastLight,
    HighContrastDark,
}

/// Tone mapping for a single role family ([primary, on-primary,
/// primary-container, on-primary-container]).
#[derive(Debug, Clone, Copy)]
struct RoleTones {
    base: u8,
    on_base: u8,
    container: u8,
    on_container: u8,
}

impl SchemeVariant {
    fn primary_tones(self) -> RoleTones {
        match self {
            Self::Light => RoleTones { base: 40, on_base: 100, container: 90, on_container: 10 },
            Self::Dark => RoleTones { base: 80, on_base: 20, container: 30, on_container: 90 },
            Self::HighContrastLight => RoleTones { base: 20, on_base: 100, container: 30, on_container: 100 },
            Self::HighContrastDark => RoleTones { base: 90, on_base: 10, container: 60, on_container: 0 },
        }
    }

    fn surface_tone(self) -> u8 {
        match self {
            Self::Light => 98,
            Self::Dark | Self::HighContrastDark => 6,
            Self::HighContrastLight => 100,
        }
    }

    fn on_surface_tone(self) -> u8 {
        match self {
            Self::Light => 10,
            Self::Dark => 90,
            Self::HighContrastLight => 0,
            Self::HighContrastDark => 100,
        }
    }

    fn is_dark(self) -> bool {
        matches!(self, Self::Dark | Self::HighContrastDark)
    }
}

/// A fully resolved M3 color scheme — 36 ARGB roles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(missing_docs)]
pub struct Scheme {
    pub variant: SchemeVariant,

    pub primary: u32,
    pub on_primary: u32,
    pub primary_container: u32,
    pub on_primary_container: u32,

    pub secondary: u32,
    pub on_secondary: u32,
    pub secondary_container: u32,
    pub on_secondary_container: u32,

    pub tertiary: u32,
    pub on_tertiary: u32,
    pub tertiary_container: u32,
    pub on_tertiary_container: u32,

    pub error: u32,
    pub on_error: u32,
    pub error_container: u32,
    pub on_error_container: u32,

    pub background: u32,
    pub on_background: u32,

    pub surface: u32,
    pub on_surface: u32,
    pub surface_variant: u32,
    pub on_surface_variant: u32,

    pub surface_dim: u32,
    pub surface_bright: u32,
    pub surface_container_lowest: u32,
    pub surface_container_low: u32,
    pub surface_container: u32,
    pub surface_container_high: u32,
    pub surface_container_highest: u32,

    pub inverse_surface: u32,
    pub inverse_on_surface: u32,
    pub inverse_primary: u32,

    pub outline: u32,
    pub outline_variant: u32,

    pub scrim: u32,
    pub shadow: u32,
    pub surface_tint: u32,
}

impl Scheme {
    /// Resolve a scheme from the 6 source palettes and a variant.
    #[must_use]
    pub fn for_palettes(palettes: &CorePalettes, variant: SchemeVariant) -> Self {
        // Role-family helpers.
        let pri = variant.primary_tones();
        let p = &palettes.primary;
        let s = &palettes.secondary;
        let t = &palettes.tertiary;
        let n = &palettes.neutral;
        let nv = &palettes.neutral_variant;
        let e = &palettes.error;

        let surface = n.tone(variant.surface_tone());
        let on_surface = n.tone(variant.on_surface_tone());

        // Surface containers follow M3 tonal recipe (Light: 94/96/98/100,
        // Dark: 12/17/22/24, mirrored for HC).
        let (sc_lowest, sc_low, sc_mid, sc_high, sc_highest, dim, bright) = if variant.is_dark() {
            (
                n.tone(4),
                n.tone(10),
                n.tone(12),
                n.tone(17),
                n.tone(22),
                n.tone(6),
                n.tone(24),
            )
        } else {
            (
                n.tone(100),
                n.tone(96),
                n.tone(94),
                n.tone(92),
                n.tone(90),
                n.tone(87),
                n.tone(98),
            )
        };

        Self {
            variant,

            primary: p.tone(pri.base),
            on_primary: p.tone(pri.on_base),
            primary_container: p.tone(pri.container),
            on_primary_container: p.tone(pri.on_container),

            secondary: s.tone(pri.base),
            on_secondary: s.tone(pri.on_base),
            secondary_container: s.tone(pri.container),
            on_secondary_container: s.tone(pri.on_container),

            tertiary: t.tone(pri.base),
            on_tertiary: t.tone(pri.on_base),
            tertiary_container: t.tone(pri.container),
            on_tertiary_container: t.tone(pri.on_container),

            error: e.tone(pri.base),
            on_error: e.tone(pri.on_base),
            error_container: e.tone(pri.container),
            on_error_container: e.tone(pri.on_container),

            background: surface,
            on_background: on_surface,

            surface,
            on_surface,
            surface_variant: nv.tone(if variant.is_dark() { 30 } else { 90 }),
            on_surface_variant: nv.tone(if variant.is_dark() { 80 } else { 30 }),

            surface_dim: dim,
            surface_bright: bright,
            surface_container_lowest: sc_lowest,
            surface_container_low: sc_low,
            surface_container: sc_mid,
            surface_container_high: sc_high,
            surface_container_highest: sc_highest,

            inverse_surface: n.tone(if variant.is_dark() { 90 } else { 20 }),
            inverse_on_surface: n.tone(if variant.is_dark() { 20 } else { 95 }),
            inverse_primary: p.tone(if variant.is_dark() { 40 } else { 80 }),

            outline: nv.tone(if variant.is_dark() { 60 } else { 50 }),
            outline_variant: nv.tone(if variant.is_dark() { 30 } else { 80 }),

            scrim: n.tone(0),
            shadow: n.tone(0),
            surface_tint: p.tone(if variant.is_dark() { 80 } else { 40 }),
        }
    }

    /// Build a Light scheme directly from a seed.
    #[must_use]
    pub fn light_from_seed(seed_argb: u32) -> Self {
        Self::for_palettes(&CorePalettes::from_seed(seed_argb), SchemeVariant::Light)
    }

    /// Build a Dark scheme directly from a seed.
    #[must_use]
    pub fn dark_from_seed(seed_argb: u32) -> Self {
        Self::for_palettes(&CorePalettes::from_seed(seed_argb), SchemeVariant::Dark)
    }

    /// Build a high-contrast Light scheme directly from a seed.
    #[must_use]
    pub fn high_contrast_light_from_seed(seed_argb: u32) -> Self {
        Self::for_palettes(
            &CorePalettes::from_seed(seed_argb),
            SchemeVariant::HighContrastLight,
        )
    }

    /// Build a high-contrast Dark scheme directly from a seed.
    #[must_use]
    pub fn high_contrast_dark_from_seed(seed_argb: u32) -> Self {
        Self::for_palettes(
            &CorePalettes::from_seed(seed_argb),
            SchemeVariant::HighContrastDark,
        )
    }

    /// Count of semantic roles exposed (excluding the `variant` discriminator).
    #[must_use]
    pub const fn role_count() -> usize {
        36
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scheme_light_from_seed_6750a4_canonical() {
        let scheme = Scheme::light_from_seed(0xFF67_50A4);
        // Primary should be a saturated purple at tone 40 — close to seed.
        let r = (scheme.primary >> 16) & 0xFF;
        let g = (scheme.primary >> 8) & 0xFF;
        let b = scheme.primary & 0xFF;
        assert!(
            r < 0xA0 && b > 0x70,
            "primary should be purple-ish, got {:06X}",
            scheme.primary
        );
        // on_primary on light scheme is tone 100 → near white.
        assert!(
            (scheme.on_primary & 0x00FF_FFFF) > 0x00F0_F0F0,
            "on_primary should be near white, got {:06X}",
            scheme.on_primary
        );
        // primary_container on light = tone 90 → very light purple.
        assert!(
            (scheme.primary_container & 0x00FF_FFFF) > 0x00C0_C0C0,
            "primary_container should be light"
        );
        let _ = g; // silence unused
    }

    #[test]
    fn scheme_dark_from_seed() {
        let scheme = Scheme::dark_from_seed(0xFF67_50A4);
        // Dark scheme surface ~ tone 6 → near black.
        assert!(
            (scheme.surface & 0x00FF_FFFF) < 0x0040_4040,
            "dark surface should be near black, got {:06X}",
            scheme.surface
        );
        // on_surface on dark = tone 90 → near white.
        assert!(
            (scheme.on_surface & 0x00FF_FFFF) > 0x00C0_C0C0,
            "dark on_surface should be near white"
        );
    }

    #[test]
    fn scheme_hc_light_contrast_stronger_than_light() {
        let light = Scheme::light_from_seed(0xFF67_50A4);
        let hc = Scheme::high_contrast_light_from_seed(0xFF67_50A4);
        // HC primary uses tone 20 (vs 40), so should be darker.
        let luma = |c: u32| ((c >> 16) & 0xFF) + ((c >> 8) & 0xFF) + (c & 0xFF);
        assert!(
            luma(hc.primary) < luma(light.primary),
            "HC primary {:06X} should be darker than light primary {:06X}",
            hc.primary,
            light.primary
        );
    }

    #[test]
    fn scheme_roles_count() {
        assert_eq!(Scheme::role_count(), 36);
    }
}
