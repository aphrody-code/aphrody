// SPDX-License-Identifier: Apache-2.0
//! Material Design 3 color roles.
//!
//! All values are ARGB `u32` (alpha in the high byte, i.e. `0xFFrrggbb`).
//! The baseline palette is derived from the M3 reference Purple seed color
//! (`#6750A4`) processed through the M3 tonal palette algorithm.
//!
//! Reference: <https://m3.material.io/styles/color/roles>

/// The complete set of M3 color roles for a single theme (light or dark).
///
/// Fields follow the M3 naming convention: each *surface* role has a
/// corresponding *on-* role that guarantees accessible contrast.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColorRoles {
    // ── Primary ──────────────────────────────────────────────────────────────
    /// Key brand color; used for prominent UI elements.
    pub primary: u32,
    /// Text/icons on top of [`primary`](ColorRoles::primary).
    pub on_primary: u32,
    /// Container tinted with primary hue (lower emphasis).
    pub primary_container: u32,
    /// Text/icons on top of [`primary_container`](ColorRoles::primary_container).
    pub on_primary_container: u32,

    // ── Secondary ────────────────────────────────────────────────────────────
    /// Accent color for less prominent UI elements.
    pub secondary: u32,
    /// Text/icons on top of [`secondary`](ColorRoles::secondary).
    pub on_secondary: u32,
    /// Container tinted with secondary hue.
    pub secondary_container: u32,
    /// Text/icons on top of [`secondary_container`](ColorRoles::secondary_container).
    pub on_secondary_container: u32,

    // ── Tertiary ─────────────────────────────────────────────────────────────
    /// Contrasting accent for decorative or differentiated elements.
    pub tertiary: u32,
    /// Text/icons on top of [`tertiary`](ColorRoles::tertiary).
    pub on_tertiary: u32,
    /// Container tinted with tertiary hue.
    pub tertiary_container: u32,
    /// Text/icons on top of [`tertiary_container`](ColorRoles::tertiary_container).
    pub on_tertiary_container: u32,

    // ── Error ─────────────────────────────────────────────────────────────────
    /// Signals destructive/error states.
    pub error: u32,
    /// Text/icons on top of [`error`](ColorRoles::error).
    pub on_error: u32,
    /// Container for error-state elements.
    pub error_container: u32,
    /// Text/icons on top of [`error_container`](ColorRoles::error_container).
    pub on_error_container: u32,

    // ── Neutral surfaces ─────────────────────────────────────────────────────
    /// Background of the app canvas.
    pub background: u32,
    /// Text/icons on [`background`](ColorRoles::background).
    pub on_background: u32,
    /// Surface of components (cards, sheets, menus).
    pub surface: u32,
    /// Text/icons on [`surface`](ColorRoles::surface).
    pub on_surface: u32,
    /// Subtle surface tint (nav drawer, modal bottom sheet).
    pub surface_variant: u32,
    /// Text/icons on [`surface_variant`](ColorRoles::surface_variant).
    pub on_surface_variant: u32,
    /// Outline for form components and dividers.
    pub outline: u32,
    /// Subtle outline variant (decorative, lower emphasis).
    pub outline_variant: u32,
    /// Opaque counterpart to background for scrims/shadows.
    pub shadow: u32,
    /// Scrim overlay color.
    pub scrim: u32,
    /// Color for inverse surfaces (e.g. snackbars on light theme).
    pub inverse_surface: u32,
    /// Text/icons on [`inverse_surface`](ColorRoles::inverse_surface).
    pub inverse_on_surface: u32,
    /// Accent color used on inverse surface.
    pub inverse_primary: u32,
}

/// M3 baseline light-theme color roles (Purple seed `#6750A4`).
///
/// Values sourced from the Material Design 3 baseline color scheme as
/// published in the M3 design kit and the `material-color-utilities` reference
/// implementation output for seed color `#6750A4`.
pub const BASELINE: ColorRoles = ColorRoles {
    // Primary
    primary: 0xFF6750A4,
    on_primary: 0xFFFFFFFF,
    primary_container: 0xFFEADDFF,
    on_primary_container: 0xFF21005D,

    // Secondary
    secondary: 0xFF625B71,
    on_secondary: 0xFFFFFFFF,
    secondary_container: 0xFFE8DEF8,
    on_secondary_container: 0xFF1D192B,

    // Tertiary
    tertiary: 0xFF7D5260,
    on_tertiary: 0xFFFFFFFF,
    tertiary_container: 0xFFFFD8E4,
    on_tertiary_container: 0xFF31111D,

    // Error
    error: 0xFFB3261E,
    on_error: 0xFFFFFFFF,
    error_container: 0xFFF9DEDC,
    on_error_container: 0xFF410E0B,

    // Neutral surfaces
    background: 0xFFFFFBFE,
    on_background: 0xFF1C1B1F,
    surface: 0xFFFFFBFE,
    on_surface: 0xFF1C1B1F,
    surface_variant: 0xFFE7E0EC,
    on_surface_variant: 0xFF49454F,
    outline: 0xFF79747E,
    outline_variant: 0xFFCAC4D0,
    shadow: 0xFF000000,
    scrim: 0xFF000000,
    inverse_surface: 0xFF313033,
    inverse_on_surface: 0xFFF4EFF4,
    inverse_primary: 0xFFD0BCFF,
};

/// M3 baseline dark-theme color roles (Purple seed `#6750A4`).
pub const BASELINE_DARK: ColorRoles = ColorRoles {
    // Primary
    primary: 0xFFD0BCFF,
    on_primary: 0xFF381E72,
    primary_container: 0xFF4F378B,
    on_primary_container: 0xFFEADDFF,

    // Secondary
    secondary: 0xFFCCC2DC,
    on_secondary: 0xFF332D41,
    secondary_container: 0xFF4A4458,
    on_secondary_container: 0xFFE8DEF8,

    // Tertiary
    tertiary: 0xFFEFB8C8,
    on_tertiary: 0xFF492532,
    tertiary_container: 0xFF633B48,
    on_tertiary_container: 0xFFFFD8E4,

    // Error
    error: 0xFFF2B8B0,
    on_error: 0xFF601410,
    error_container: 0xFF8C1D18,
    on_error_container: 0xFFF9DEDC,

    // Neutral surfaces
    background: 0xFF1C1B1F,
    on_background: 0xFFE6E1E5,
    surface: 0xFF1C1B1F,
    on_surface: 0xFFE6E1E5,
    surface_variant: 0xFF49454F,
    on_surface_variant: 0xFFCAC4D0,
    outline: 0xFF938F99,
    outline_variant: 0xFF49454F,
    shadow: 0xFF000000,
    scrim: 0xFF000000,
    inverse_surface: 0xFFE6E1E5,
    inverse_on_surface: 0xFF313033,
    inverse_primary: 0xFF6750A4,
};

/// Emits a CSS `:root` block containing all M3 `--md-sys-color-*` variables.
///
/// Each color is formatted as `#RRGGBB` (alpha byte is dropped because CSS
/// custom properties do not need it for opaque colors).  The returned
/// [`String`] is ready to be injected into a `<style>` tag or written to a
/// `.css` file.
///
/// This function is only available when the `std` feature is enabled.
///
/// # Example
///
/// ```rust
/// use m3_tokens::color::{BASELINE, export_css};
/// let css = export_css(&BASELINE);
/// assert!(css.contains("--md-sys-color-primary:"));
/// ```
#[cfg(feature = "std")]
pub fn export_css(theme: &ColorRoles) -> std::string::String {
    /// Format an ARGB u32 as `#RRGGBB`.
    #[inline]
    fn hex(argb: u32) -> std::string::String {
        std::format!("#{:06X}", argb & 0x00FF_FFFF)
    }

    std::format!(
        ":root {{\n\x20 --md-sys-color-primary: {primary};\n\x20 --md-sys-color-on-primary: \
         {on_primary};\n\x20 --md-sys-color-primary-container: {primary_container};\n\x20 \
         --md-sys-color-on-primary-container: {on_primary_container};\n\x20 \
         --md-sys-color-secondary: {secondary};\n\x20 --md-sys-color-on-secondary: \
         {on_secondary};\n\x20 --md-sys-color-secondary-container: {secondary_container};\n\x20 \
         --md-sys-color-on-secondary-container: {on_secondary_container};\n\x20 \
         --md-sys-color-tertiary: {tertiary};\n\x20 --md-sys-color-on-tertiary: \
         {on_tertiary};\n\x20 --md-sys-color-tertiary-container: {tertiary_container};\n\x20 \
         --md-sys-color-on-tertiary-container: {on_tertiary_container};\n\x20 \
         --md-sys-color-error: {error};\n\x20 --md-sys-color-on-error: {on_error};\n\x20 \
         --md-sys-color-error-container: {error_container};\n\x20 \
         --md-sys-color-on-error-container: {on_error_container};\n\x20 \
         --md-sys-color-background: {background};\n\x20 --md-sys-color-on-background: \
         {on_background};\n\x20 --md-sys-color-surface: {surface};\n\x20 \
         --md-sys-color-on-surface: {on_surface};\n\x20 --md-sys-color-surface-variant: \
         {surface_variant};\n\x20 --md-sys-color-on-surface-variant: {on_surface_variant};\n\x20 \
         --md-sys-color-outline: {outline};\n\x20 --md-sys-color-outline-variant: \
         {outline_variant};\n\x20 --md-sys-color-shadow: {shadow};\n\x20 --md-sys-color-scrim: \
         {scrim};\n\x20 --md-sys-color-inverse-surface: {inverse_surface};\n\x20 \
         --md-sys-color-inverse-on-surface: {inverse_on_surface};\n\x20 \
         --md-sys-color-inverse-primary: {inverse_primary};\n}}",
        primary = hex(theme.primary),
        on_primary = hex(theme.on_primary),
        primary_container = hex(theme.primary_container),
        on_primary_container = hex(theme.on_primary_container),
        secondary = hex(theme.secondary),
        on_secondary = hex(theme.on_secondary),
        secondary_container = hex(theme.secondary_container),
        on_secondary_container = hex(theme.on_secondary_container),
        tertiary = hex(theme.tertiary),
        on_tertiary = hex(theme.on_tertiary),
        tertiary_container = hex(theme.tertiary_container),
        on_tertiary_container = hex(theme.on_tertiary_container),
        error = hex(theme.error),
        on_error = hex(theme.on_error),
        error_container = hex(theme.error_container),
        on_error_container = hex(theme.on_error_container),
        background = hex(theme.background),
        on_background = hex(theme.on_background),
        surface = hex(theme.surface),
        on_surface = hex(theme.on_surface),
        surface_variant = hex(theme.surface_variant),
        on_surface_variant = hex(theme.on_surface_variant),
        outline = hex(theme.outline),
        outline_variant = hex(theme.outline_variant),
        shadow = hex(theme.shadow),
        scrim = hex(theme.scrim),
        inverse_surface = hex(theme.inverse_surface),
        inverse_on_surface = hex(theme.inverse_on_surface),
        inverse_primary = hex(theme.inverse_primary),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baseline_primary_is_m3_reference_purple() {
        assert_eq!(BASELINE.primary, 0xFF6750A4);
    }

    #[test]
    fn baseline_dark_primary_differs_from_light() {
        assert_ne!(BASELINE.primary, BASELINE_DARK.primary);
    }

    #[cfg(feature = "std")]
    #[test]
    fn css_export_contains_primary_variable() {
        let css = export_css(&BASELINE);
        assert!(
            css.contains("--md-sys-color-primary:"),
            "CSS output must declare --md-sys-color-primary"
        );
    }

    #[cfg(feature = "std")]
    #[test]
    fn css_export_hex_format_is_six_digit() {
        let css = export_css(&BASELINE);
        // Primary should render as #6750A4 (no alpha byte)
        assert!(css.contains("#6750A4"), "primary hex should be #6750A4, got:\n{css}");
    }

    #[cfg(feature = "std")]
    #[test]
    fn css_export_contains_all_29_variables() {
        let css = export_css(&BASELINE);
        let count = css.matches("--md-sys-color-").count();
        assert_eq!(count, 29, "expected 29 color variables, found {count}");
    }
}
