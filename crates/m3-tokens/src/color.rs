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

    // ── Expanded surfaces (M3 2023+) ─────────────────────────────────────────
    /// Dimmest neutral surface tone (lowest in the surface set).
    pub surface_dim: u32,
    /// Brightest neutral surface tone.
    pub surface_bright: u32,
    /// Lowest-emphasis surface container (furthest back).
    pub surface_container_lowest: u32,
    /// Low-emphasis surface container.
    pub surface_container_low: u32,
    /// Default surface container.
    pub surface_container: u32,
    /// High-emphasis surface container.
    pub surface_container_high: u32,
    /// Highest-emphasis surface container (closest to foreground).
    pub surface_container_highest: u32,
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

    // Expanded surfaces
    surface_dim: 0xFFDED8E1,
    surface_bright: 0xFFFEF7FF,
    surface_container_lowest: 0xFFFFFFFF,
    surface_container_low: 0xFFF7F2FA,
    surface_container: 0xFFF3EDF7,
    surface_container_high: 0xFFECE6F0,
    surface_container_highest: 0xFFE6E0E9,
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

    // Expanded surfaces
    surface_dim: 0xFF141218,
    surface_bright: 0xFF3B383E,
    surface_container_lowest: 0xFF0F0D13,
    surface_container_low: 0xFF1D1B20,
    surface_container: 0xFF211F26,
    surface_container_high: 0xFF2B2930,
    surface_container_highest: 0xFF36343B,
};

/// aphrody brand light-theme color roles (rust seed `#CE422B`).
///
/// Generated with the M3 `SchemeTonalSpot` algorithm
/// (`@material/material-color-utilities`, the Material Theme Builder engine)
/// from aphrody's brand seed. See `docs/design/aphrody-m3-theme.json`.
pub const APHRODY: ColorRoles = ColorRoles {
    primary: 0xFF904B3E,
    on_primary: 0xFFFFFFFF,
    primary_container: 0xFFFFDAD3,
    on_primary_container: 0xFF733428,
    secondary: 0xFF775650,
    on_secondary: 0xFFFFFFFF,
    secondary_container: 0xFFFFDAD3,
    on_secondary_container: 0xFF5D3F3A,
    tertiary: 0xFF6F5C2E,
    on_tertiary: 0xFFFFFFFF,
    tertiary_container: 0xFFFAE0A6,
    on_tertiary_container: 0xFF554519,
    error: 0xFFBA1A1A,
    on_error: 0xFFFFFFFF,
    error_container: 0xFFFFDAD6,
    on_error_container: 0xFF93000A,
    background: 0xFFFFF8F6,
    on_background: 0xFF231918,
    surface: 0xFFFFF8F6,
    on_surface: 0xFF231918,
    surface_variant: 0xFFF5DDD9,
    on_surface_variant: 0xFF534340,
    outline: 0xFF85736F,
    outline_variant: 0xFFD8C2BD,
    shadow: 0xFF000000,
    scrim: 0xFF000000,
    inverse_surface: 0xFF392E2C,
    inverse_on_surface: 0xFFFFEDE9,
    inverse_primary: 0xFFFFB4A6,
    surface_dim: 0xFFE8D6D3,
    surface_bright: 0xFFFFF8F6,
    surface_container_lowest: 0xFFFFFFFF,
    surface_container_low: 0xFFFFF0EE,
    surface_container: 0xFFFCEAE6,
    surface_container_high: 0xFFF7E4E1,
    surface_container_highest: 0xFFF1DFDB,
};

/// aphrody brand dark-theme color roles (rust seed `#CE422B`). Dark-first
/// default for aphrody UIs. See `docs/design/aphrody-m3-tokens.md`.
pub const APHRODY_DARK: ColorRoles = ColorRoles {
    primary: 0xFFFFB4A6,
    on_primary: 0xFF561E14,
    primary_container: 0xFF733428,
    on_primary_container: 0xFFFFDAD3,
    secondary: 0xFFE7BDB5,
    on_secondary: 0xFF442A24,
    secondary_container: 0xFF5D3F3A,
    on_secondary_container: 0xFFFFDAD3,
    tertiary: 0xFFDCC48C,
    on_tertiary: 0xFF3D2E04,
    tertiary_container: 0xFF554519,
    on_tertiary_container: 0xFFFAE0A6,
    error: 0xFFFFB4AB,
    on_error: 0xFF690005,
    error_container: 0xFF93000A,
    on_error_container: 0xFFFFDAD6,
    background: 0xFF1A1110,
    on_background: 0xFFF1DFDB,
    surface: 0xFF1A1110,
    on_surface: 0xFFF1DFDB,
    surface_variant: 0xFF534340,
    on_surface_variant: 0xFFD8C2BD,
    outline: 0xFFA08C89,
    outline_variant: 0xFF534340,
    shadow: 0xFF000000,
    scrim: 0xFF000000,
    inverse_surface: 0xFFF1DFDB,
    inverse_on_surface: 0xFF392E2C,
    inverse_primary: 0xFF904B3E,
    surface_dim: 0xFF1A1110,
    surface_bright: 0xFF423735,
    surface_container_lowest: 0xFF140C0B,
    surface_container_low: 0xFF231918,
    surface_container: 0xFF271D1C,
    surface_container_high: 0xFF322826,
    surface_container_highest: 0xFF3D3230,
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
         --md-sys-color-inverse-primary: {inverse_primary};\n\x20 \
         --md-sys-color-surface-dim: {surface_dim};\n\x20 --md-sys-color-surface-bright: \
         {surface_bright};\n\x20 --md-sys-color-surface-container-lowest: \
         {surface_container_lowest};\n\x20 --md-sys-color-surface-container-low: \
         {surface_container_low};\n\x20 --md-sys-color-surface-container: \
         {surface_container};\n\x20 --md-sys-color-surface-container-high: \
         {surface_container_high};\n\x20 --md-sys-color-surface-container-highest: \
         {surface_container_highest};\n}}",
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
        surface_dim = hex(theme.surface_dim),
        surface_bright = hex(theme.surface_bright),
        surface_container_lowest = hex(theme.surface_container_lowest),
        surface_container_low = hex(theme.surface_container_low),
        surface_container = hex(theme.surface_container),
        surface_container_high = hex(theme.surface_container_high),
        surface_container_highest = hex(theme.surface_container_highest),
    )
}

/// Mapping from a semantic UI-system token name to the M3 `--md-sys-color-*`
/// role it aliases. Shared by [`export_shadcn_aliases`] (shadcn/ui variable
/// names) and [`export_tailwind_theme`] (Tailwind v4 `--color-*` names) so the
/// two fusion sheets stay in lockstep. See `docs/ui/FUSION-PLAN.md`.
pub const FUSION_ALIAS_MAP: &[(&str, &str)] = &[
    ("background", "surface"),
    ("foreground", "on-surface"),
    ("card", "surface-container-low"),
    ("card-foreground", "on-surface"),
    ("popover", "surface-container"),
    ("popover-foreground", "on-surface"),
    ("primary", "primary"),
    ("primary-foreground", "on-primary"),
    ("secondary", "secondary-container"),
    ("secondary-foreground", "on-secondary-container"),
    ("muted", "surface-variant"),
    ("muted-foreground", "on-surface-variant"),
    ("accent", "tertiary-container"),
    ("accent-foreground", "on-tertiary-container"),
    ("destructive", "error"),
    ("destructive-foreground", "on-error"),
    ("border", "outline-variant"),
    ("input", "outline-variant"),
    ("ring", "primary"),
];

/// Emits a shadcn/ui semantic-token `:root` block aliasing shadcn variables to
/// the M3 system colors produced by [`export_css`]. The aliases are pure
/// `var()` references (theme-independent): emit this sheet *after* the
/// `export_css` output so shadcn components inherit the live M3 palette, and
/// light/dark switching flows through automatically. Backs the `registry:theme`
/// fusion item in `docs/ui/FUSION-PLAN.md`.
///
/// Only available with the `std` feature.
///
/// # Example
///
/// ```rust
/// use m3_tokens::color::export_shadcn_aliases;
/// let css = export_shadcn_aliases();
/// assert!(css.contains("--primary: var(--md-sys-color-primary);"));
/// ```
#[cfg(feature = "std")]
pub fn export_shadcn_aliases() -> std::string::String {
    let mut out = std::string::String::from(":root {\n");
    for (name, role) in FUSION_ALIAS_MAP {
        out.push_str(&std::format!("  --{name}: var(--md-sys-color-{role});\n"));
    }
    out.push('}');
    out
}

/// Emits a Tailwind v4 `@theme inline { ... }` block mapping Tailwind's
/// `--color-*` design tokens to the M3 system colors produced by
/// [`export_css`]. Import this so utilities (`bg-primary`, `text-foreground`,
/// `border-border`, ...) resolve to the live M3 palette. Backs the Tailwind
/// side of the fusion described in `docs/ui/FUSION-PLAN.md`.
///
/// Only available with the `std` feature.
///
/// # Example
///
/// ```rust
/// use m3_tokens::color::export_tailwind_theme;
/// let css = export_tailwind_theme();
/// assert!(css.starts_with("@theme inline {"));
/// assert!(css.contains("--color-primary: var(--md-sys-color-primary);"));
/// ```
#[cfg(feature = "std")]
pub fn export_tailwind_theme() -> std::string::String {
    let mut out = std::string::String::from("@theme inline {\n");
    for (name, role) in FUSION_ALIAS_MAP {
        out.push_str(&std::format!("  --color-{name}: var(--md-sys-color-{role});\n"));
    }
    out.push('}');
    out
}

/// Emits the complete fusion stylesheet: the M3 `:root` system colors
/// ([`export_css`]) followed by the shadcn/ui alias block
/// ([`export_shadcn_aliases`]) and the Tailwind v4 `@theme inline` block
/// ([`export_tailwind_theme`]), separated by blank lines. This single sheet is
/// the materialised output of the three-way UI fusion (Material 3 + shadcn +
/// Tailwind) — see `docs/ui/FUSION-PLAN.md`.
///
/// Only available with the `std` feature.
///
/// # Example
///
/// ```rust
/// use m3_tokens::color::{BASELINE, export_fusion_css};
/// let css = export_fusion_css(&BASELINE);
/// assert!(css.contains("--md-sys-color-primary:"));
/// assert!(css.contains("--primary: var(--md-sys-color-primary);"));
/// assert!(css.contains("@theme inline {"));
/// ```
#[cfg(feature = "std")]
pub fn export_fusion_css(theme: &ColorRoles) -> std::string::String {
    std::format!(
        "{}\n\n{}\n\n{}",
        export_css(theme),
        export_shadcn_aliases(),
        export_tailwind_theme()
    )
}

/// Returns every M3 system color as `(suffix, "#RRGGBB")` pairs, where `suffix`
/// is the part after `--md-sys-color-` (e.g. `"primary"`, `"on-surface-variant"`).
/// Structured counterpart of [`export_css`], consumed by tooling that builds
/// JSON token catalogs — e.g. the shadcn `registry:theme` generator behind
/// `aphrody design tokens --format shadcn-registry` (see `docs/ui/FUSION-PLAN.md`).
/// Order matches [`export_css`].
///
/// Only available with the `std` feature.
///
/// # Example
///
/// ```rust
/// use m3_tokens::color::{BASELINE, color_vars};
/// let vars = color_vars(&BASELINE);
/// assert_eq!(vars.len(), 36);
/// assert!(vars.iter().any(|(k, v)| *k == "primary" && v == "#6750A4"));
/// ```
#[cfg(feature = "std")]
pub fn color_vars(theme: &ColorRoles) -> std::vec::Vec<(&'static str, std::string::String)> {
    #[inline]
    fn hex(argb: u32) -> std::string::String {
        std::format!("#{:06X}", argb & 0x00FF_FFFF)
    }
    std::vec![
        ("primary", hex(theme.primary)),
        ("on-primary", hex(theme.on_primary)),
        ("primary-container", hex(theme.primary_container)),
        ("on-primary-container", hex(theme.on_primary_container)),
        ("secondary", hex(theme.secondary)),
        ("on-secondary", hex(theme.on_secondary)),
        ("secondary-container", hex(theme.secondary_container)),
        ("on-secondary-container", hex(theme.on_secondary_container)),
        ("tertiary", hex(theme.tertiary)),
        ("on-tertiary", hex(theme.on_tertiary)),
        ("tertiary-container", hex(theme.tertiary_container)),
        ("on-tertiary-container", hex(theme.on_tertiary_container)),
        ("error", hex(theme.error)),
        ("on-error", hex(theme.on_error)),
        ("error-container", hex(theme.error_container)),
        ("on-error-container", hex(theme.on_error_container)),
        ("background", hex(theme.background)),
        ("on-background", hex(theme.on_background)),
        ("surface", hex(theme.surface)),
        ("on-surface", hex(theme.on_surface)),
        ("surface-variant", hex(theme.surface_variant)),
        ("on-surface-variant", hex(theme.on_surface_variant)),
        ("outline", hex(theme.outline)),
        ("outline-variant", hex(theme.outline_variant)),
        ("shadow", hex(theme.shadow)),
        ("scrim", hex(theme.scrim)),
        ("inverse-surface", hex(theme.inverse_surface)),
        ("inverse-on-surface", hex(theme.inverse_on_surface)),
        ("inverse-primary", hex(theme.inverse_primary)),
        ("surface-dim", hex(theme.surface_dim)),
        ("surface-bright", hex(theme.surface_bright)),
        ("surface-container-lowest", hex(theme.surface_container_lowest)),
        ("surface-container-low", hex(theme.surface_container_low)),
        ("surface-container", hex(theme.surface_container)),
        ("surface-container-high", hex(theme.surface_container_high)),
        ("surface-container-highest", hex(theme.surface_container_highest)),
    ]
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
    fn css_export_contains_all_36_variables() {
        let css = export_css(&BASELINE);
        let count = css.matches("--md-sys-color-").count();
        assert_eq!(count, 36, "expected 36 color variables, found {count}");
    }

    #[cfg(feature = "std")]
    #[test]
    fn css_export_contains_expanded_surfaces() {
        let css = export_css(&BASELINE);
        for var in [
            "--md-sys-color-surface-dim:",
            "--md-sys-color-surface-bright:",
            "--md-sys-color-surface-container-lowest:",
            "--md-sys-color-surface-container-highest:",
        ] {
            assert!(css.contains(var), "missing {var}");
        }
    }

    #[test]
    fn aphrody_theme_is_rust_seeded_and_distinct() {
        // Dark-first brand: aphrody primary differs from the purple baseline.
        assert_eq!(APHRODY_DARK.primary, 0xFFFFB4A6);
        assert_ne!(APHRODY_DARK.primary, BASELINE_DARK.primary);
        // tertiary is the auto-derived gold.
        assert_eq!(APHRODY_DARK.tertiary, 0xFFDCC48C);
        // expanded surfaces populated.
        assert_eq!(APHRODY_DARK.surface_container, 0xFF271D1C);
    }
}
