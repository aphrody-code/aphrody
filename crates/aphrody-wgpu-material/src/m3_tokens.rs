// SPDX-License-Identifier: Apache-2.0
//! Bundled Material Design 3 baseline tokens.
//!
//! These values are exact mirrors of the canonical M3 baseline palette
//! (seeded from `#6750A4`) and the canonical type / shape / elevation
//! scales documented at <https://m3.material.io>.  They are duplicated
//! here so the crate stays buildable in isolation (self-rooted
//! workspace).  When the `external-tokens` feature is enabled by a
//! parent build, a path dependency on `m3-tokens` overrides these
//! constants — see `Cargo.toml`.

use crate::canvas::Color;

// ─── Color roles ────────────────────────────────────────────────────────────

/// Complete set of M3 color roles for a given theme.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorRoles {
    /// Brand-prominent fill.
    pub primary: Color,
    /// Foreground on primary.
    pub on_primary: Color,
    /// Lower-emphasis primary fill.
    pub primary_container: Color,
    /// Foreground on `primary_container`.
    pub on_primary_container: Color,
    /// Accent.
    pub secondary: Color,
    /// Foreground on secondary.
    pub on_secondary: Color,
    /// Lower-emphasis secondary fill.
    pub secondary_container: Color,
    /// Foreground on `secondary_container`.
    pub on_secondary_container: Color,
    /// Contrasting decorative accent.
    pub tertiary: Color,
    /// Foreground on tertiary.
    pub on_tertiary: Color,
    /// Lower-emphasis tertiary fill.
    pub tertiary_container: Color,
    /// Foreground on `tertiary_container`.
    pub on_tertiary_container: Color,
    /// Destructive-state fill.
    pub error: Color,
    /// Foreground on error.
    pub on_error: Color,
    /// Lower-emphasis error fill.
    pub error_container: Color,
    /// Foreground on `error_container`.
    pub on_error_container: Color,
    /// App canvas.
    pub background: Color,
    /// Foreground on background.
    pub on_background: Color,
    /// Component surface.
    pub surface: Color,
    /// Foreground on surface.
    pub on_surface: Color,
    /// Subtle surface tint.
    pub surface_variant: Color,
    /// Foreground on `surface_variant`.
    pub on_surface_variant: Color,
    /// Outline for form components.
    pub outline: Color,
    /// Subtle outline.
    pub outline_variant: Color,
    /// Inverse surface (e.g. snackbar on light theme).
    pub inverse_surface: Color,
    /// Foreground on inverse surface.
    pub inverse_on_surface: Color,
    /// Primary tint on inverse surface (e.g. snackbar action).
    pub inverse_primary: Color,
    /// Scrim color used for modal dialogs.
    pub scrim: Color,
    /// Shadow color.
    pub shadow: Color,
}

/// Build a [`Color`] from a packed `0xAARRGGBB` literal.
const fn argb(p: u32) -> Color {
    let a = ((p >> 24) & 0xFF) as u8;
    let r = ((p >> 16) & 0xFF) as u8;
    let g = ((p >> 8) & 0xFF) as u8;
    let b = (p & 0xFF) as u8;
    Color {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: a as f32 / 255.0,
    }
}

/// M3 baseline light theme.  Seed color `#6750A4` (Purple).
pub const BASELINE_LIGHT: ColorRoles = ColorRoles {
    primary: argb(0xFF6750A4),
    on_primary: argb(0xFFFFFFFF),
    primary_container: argb(0xFFEADDFF),
    on_primary_container: argb(0xFF21005D),
    secondary: argb(0xFF625B71),
    on_secondary: argb(0xFFFFFFFF),
    secondary_container: argb(0xFFE8DEF8),
    on_secondary_container: argb(0xFF1D192B),
    tertiary: argb(0xFF7D5260),
    on_tertiary: argb(0xFFFFFFFF),
    tertiary_container: argb(0xFFFFD8E4),
    on_tertiary_container: argb(0xFF31111D),
    error: argb(0xFFB3261E),
    on_error: argb(0xFFFFFFFF),
    error_container: argb(0xFFF9DEDC),
    on_error_container: argb(0xFF410E0B),
    background: argb(0xFFFFFBFE),
    on_background: argb(0xFF1C1B1F),
    surface: argb(0xFFFFFBFE),
    on_surface: argb(0xFF1C1B1F),
    surface_variant: argb(0xFFE7E0EC),
    on_surface_variant: argb(0xFF49454F),
    outline: argb(0xFF79747E),
    outline_variant: argb(0xFFCAC4D0),
    inverse_surface: argb(0xFF313033),
    inverse_on_surface: argb(0xFFF4EFF4),
    inverse_primary: argb(0xFFD0BCFF),
    scrim: argb(0x80000000),
    shadow: argb(0xFF000000),
};

/// M3 baseline dark theme.
pub const BASELINE_DARK: ColorRoles = ColorRoles {
    primary: argb(0xFFD0BCFF),
    on_primary: argb(0xFF381E72),
    primary_container: argb(0xFF4F378B),
    on_primary_container: argb(0xFFEADDFF),
    secondary: argb(0xFFCCC2DC),
    on_secondary: argb(0xFF332D41),
    secondary_container: argb(0xFF4A4458),
    on_secondary_container: argb(0xFFE8DEF8),
    tertiary: argb(0xFFEFB8C8),
    on_tertiary: argb(0xFF492532),
    tertiary_container: argb(0xFF633B48),
    on_tertiary_container: argb(0xFFFFD8E4),
    error: argb(0xFFF2B8B5),
    on_error: argb(0xFF601410),
    error_container: argb(0xFF8C1D18),
    on_error_container: argb(0xFFF9DEDC),
    background: argb(0xFF1C1B1F),
    on_background: argb(0xFFE6E1E5),
    surface: argb(0xFF1C1B1F),
    on_surface: argb(0xFFE6E1E5),
    surface_variant: argb(0xFF49454F),
    on_surface_variant: argb(0xFFCAC4D0),
    outline: argb(0xFF938F99),
    outline_variant: argb(0xFF49454F),
    inverse_surface: argb(0xFFE6E1E5),
    inverse_on_surface: argb(0xFF313033),
    inverse_primary: argb(0xFF6750A4),
    scrim: argb(0x80000000),
    shadow: argb(0xFF000000),
};

// ─── Shape (corner radii) ───────────────────────────────────────────────────

/// M3 shape scale (corner radii in dp).  Values from the M3 shape system.
pub mod shape {
    /// 0 dp — sharp.
    pub const NONE: f32 = 0.0;
    /// 4 dp — extra small.
    pub const EXTRA_SMALL: f32 = 4.0;
    /// 8 dp — small.
    pub const SMALL: f32 = 8.0;
    /// 12 dp — medium.
    pub const MEDIUM: f32 = 12.0;
    /// 16 dp — large.
    pub const LARGE: f32 = 16.0;
    /// 28 dp — extra large.
    pub const EXTRA_LARGE: f32 = 28.0;
    /// Half the smaller side — fully rounded "pill" shape.
    pub const FULL: f32 = f32::INFINITY;
}

// ─── Elevation (shadow offsets) ─────────────────────────────────────────────

/// M3 elevation levels (dp of resting elevation).  These map to shadow
/// blur/offset pairs in [`elevation::shadow_for_level`].
pub mod elevation {
    /// Shadow descriptor — offset, blur, opacity.
    #[derive(Clone, Copy, Debug, PartialEq)]
    pub struct Shadow {
        /// X offset in dp.
        pub offset_x: f32,
        /// Y offset in dp.
        pub offset_y: f32,
        /// Blur radius in dp.
        pub blur: f32,
        /// Opacity (0.0..=1.0).
        pub opacity: f32,
    }

    /// Resting M3 elevation levels (dp).
    pub const LEVEL_0: f32 = 0.0;
    /// Level 1 (cards, small surfaces).
    pub const LEVEL_1: f32 = 1.0;
    /// Level 2 (elevated buttons, snackbars).
    pub const LEVEL_2: f32 = 3.0;
    /// Level 3 (FAB resting).
    pub const LEVEL_3: f32 = 6.0;
    /// Level 4 (nav drawer).
    pub const LEVEL_4: f32 = 8.0;
    /// Level 5 (FAB pressed).
    pub const LEVEL_5: f32 = 12.0;

    /// Lookup the canonical M3 shadow for an elevation level (1..=5).
    #[must_use]
    pub fn shadow_for_level(level: u8) -> Shadow {
        match level {
            0 => Shadow { offset_x: 0.0, offset_y: 0.0, blur: 0.0, opacity: 0.0 },
            1 => Shadow { offset_x: 0.0, offset_y: 1.0, blur: 3.0, opacity: 0.15 },
            2 => Shadow { offset_x: 0.0, offset_y: 2.0, blur: 6.0, opacity: 0.15 },
            3 => Shadow { offset_x: 0.0, offset_y: 4.0, blur: 8.0, opacity: 0.15 },
            4 => Shadow { offset_x: 0.0, offset_y: 6.0, blur: 10.0, opacity: 0.15 },
            _ => Shadow { offset_x: 0.0, offset_y: 8.0, blur: 12.0, opacity: 0.15 },
        }
    }
}

// ─── Typography (type scale) ────────────────────────────────────────────────

/// M3 type scale — font size in sp + line-height + weight (CSS weight units).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TypeStyle {
    /// Font size in scalable pixels (1 sp = 1 dp at default density).
    pub size_sp: f32,
    /// Line height in sp.
    pub line_height_sp: f32,
    /// Letter spacing in sp.
    pub letter_spacing_sp: f32,
    /// CSS weight (400 = regular, 500 = medium).
    pub weight: u16,
}

/// M3 type scale — Display large.
pub const DISPLAY_LARGE: TypeStyle =
    TypeStyle { size_sp: 57.0, line_height_sp: 64.0, letter_spacing_sp: -0.25, weight: 400 };
/// M3 type scale — Display medium.
pub const DISPLAY_MEDIUM: TypeStyle =
    TypeStyle { size_sp: 45.0, line_height_sp: 52.0, letter_spacing_sp: 0.0, weight: 400 };
/// M3 type scale — Display small.
pub const DISPLAY_SMALL: TypeStyle =
    TypeStyle { size_sp: 36.0, line_height_sp: 44.0, letter_spacing_sp: 0.0, weight: 400 };
/// M3 type scale — Headline large.
pub const HEADLINE_LARGE: TypeStyle =
    TypeStyle { size_sp: 32.0, line_height_sp: 40.0, letter_spacing_sp: 0.0, weight: 400 };
/// M3 type scale — Headline medium.
pub const HEADLINE_MEDIUM: TypeStyle =
    TypeStyle { size_sp: 28.0, line_height_sp: 36.0, letter_spacing_sp: 0.0, weight: 400 };
/// M3 type scale — Headline small.
pub const HEADLINE_SMALL: TypeStyle =
    TypeStyle { size_sp: 24.0, line_height_sp: 32.0, letter_spacing_sp: 0.0, weight: 400 };
/// M3 type scale — Title large.
pub const TITLE_LARGE: TypeStyle =
    TypeStyle { size_sp: 22.0, line_height_sp: 28.0, letter_spacing_sp: 0.0, weight: 400 };
/// M3 type scale — Title medium.
pub const TITLE_MEDIUM: TypeStyle =
    TypeStyle { size_sp: 16.0, line_height_sp: 24.0, letter_spacing_sp: 0.15, weight: 500 };
/// M3 type scale — Title small.
pub const TITLE_SMALL: TypeStyle =
    TypeStyle { size_sp: 14.0, line_height_sp: 20.0, letter_spacing_sp: 0.1, weight: 500 };
/// M3 type scale — Body large.
pub const BODY_LARGE: TypeStyle =
    TypeStyle { size_sp: 16.0, line_height_sp: 24.0, letter_spacing_sp: 0.5, weight: 400 };
/// M3 type scale — Body medium.
pub const BODY_MEDIUM: TypeStyle =
    TypeStyle { size_sp: 14.0, line_height_sp: 20.0, letter_spacing_sp: 0.25, weight: 400 };
/// M3 type scale — Body small.
pub const BODY_SMALL: TypeStyle =
    TypeStyle { size_sp: 12.0, line_height_sp: 16.0, letter_spacing_sp: 0.4, weight: 400 };
/// M3 type scale — Label large (button label).
pub const LABEL_LARGE: TypeStyle =
    TypeStyle { size_sp: 14.0, line_height_sp: 20.0, letter_spacing_sp: 0.1, weight: 500 };
/// M3 type scale — Label medium.
pub const LABEL_MEDIUM: TypeStyle =
    TypeStyle { size_sp: 12.0, line_height_sp: 16.0, letter_spacing_sp: 0.5, weight: 500 };
/// M3 type scale — Label small.
pub const LABEL_SMALL: TypeStyle =
    TypeStyle { size_sp: 11.0, line_height_sp: 16.0, letter_spacing_sp: 0.5, weight: 500 };

/// Full M3 type scale ordered display → label (15 entries).
pub const TYPE_SCALE: [TypeStyle; 15] = [
    DISPLAY_LARGE,
    DISPLAY_MEDIUM,
    DISPLAY_SMALL,
    HEADLINE_LARGE,
    HEADLINE_MEDIUM,
    HEADLINE_SMALL,
    TITLE_LARGE,
    TITLE_MEDIUM,
    TITLE_SMALL,
    BODY_LARGE,
    BODY_MEDIUM,
    BODY_SMALL,
    LABEL_LARGE,
    LABEL_MEDIUM,
    LABEL_SMALL,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_scale_body_medium() {
        assert_eq!(BODY_MEDIUM.size_sp, 14.0);
        assert_eq!(BODY_MEDIUM.line_height_sp, 20.0);
        assert_eq!(BODY_MEDIUM.weight, 400);
    }

    #[test]
    fn elevation_shadow_offset() {
        let s = elevation::shadow_for_level(3);
        assert_eq!(s.offset_y, 4.0);
        assert_eq!(s.blur, 8.0);
        assert!((s.opacity - 0.15).abs() < f32::EPSILON);
    }

    #[test]
    fn type_scale_full_count_is_15() {
        assert_eq!(TYPE_SCALE.len(), 15);
    }
}
