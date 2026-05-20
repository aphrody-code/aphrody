// SPDX-License-Identifier: Apache-2.0
//! Material Design 3 type scale.
//!
//! Provides the 15 canonical type styles as compile-time constants.
//! Values are taken from the M3 type scale specification:
//! <https://m3.material.io/styles/typography/type-scale-tokens>
//!
//! All sizes and spacings are in density-independent pixels (dp) as specified
//! by M3.  On the web these map 1:1 to `rem` (base 16 px) or `px` at 1×.

/// A single M3 type style.
///
/// Every field is `'static` or a primitive so the struct can live in read-only
/// data without allocation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TypeStyle {
    /// Font family name as declared in the M3 spec.
    /// Baseline value is `"Google Sans Flex"` for all 15 styles.
    pub font: &'static str,
    /// CSS / OpenType numeric weight (100–900).
    pub weight: u16,
    /// Type size in dp.
    pub size_dp: f32,
    /// Line height in dp.
    pub line_height_dp: f32,
    /// Letter spacing in dp (negative values tighten tracking).
    pub letter_spacing_dp: f32,
}

// ── Display ─────────────────────────────────────────────────────────────────

/// Display Large — the largest display style, used for short, important text.
pub const DISPLAY_LARGE: TypeStyle = TypeStyle {
    font: "Google Sans Flex",
    weight: 400,
    size_dp: 57.0,
    line_height_dp: 64.0,
    letter_spacing_dp: -0.25,
};

/// Display Medium.
pub const DISPLAY_MEDIUM: TypeStyle = TypeStyle {
    font: "Google Sans Flex",
    weight: 400,
    size_dp: 45.0,
    line_height_dp: 52.0,
    letter_spacing_dp: 0.0,
};

/// Display Small.
pub const DISPLAY_SMALL: TypeStyle = TypeStyle {
    font: "Google Sans Flex",
    weight: 400,
    size_dp: 36.0,
    line_height_dp: 44.0,
    letter_spacing_dp: 0.0,
};

// ── Headline ────────────────────────────────────────────────────────────────

/// Headline Large — high-emphasis, shorter spans (article titles, dialog headers).
pub const HEADLINE_LARGE: TypeStyle = TypeStyle {
    font: "Google Sans Flex",
    weight: 400,
    size_dp: 32.0,
    line_height_dp: 40.0,
    letter_spacing_dp: 0.0,
};

/// Headline Medium.
pub const HEADLINE_MEDIUM: TypeStyle = TypeStyle {
    font: "Google Sans Flex",
    weight: 400,
    size_dp: 28.0,
    line_height_dp: 36.0,
    letter_spacing_dp: 0.0,
};

/// Headline Small.
pub const HEADLINE_SMALL: TypeStyle = TypeStyle {
    font: "Google Sans Flex",
    weight: 400,
    size_dp: 24.0,
    line_height_dp: 32.0,
    letter_spacing_dp: 0.0,
};

// ── Title ───────────────────────────────────────────────────────────────────

/// Title Large — large, semi-bold titles (app bar, dialog title).
pub const TITLE_LARGE: TypeStyle = TypeStyle {
    font: "Google Sans Flex",
    weight: 400,
    size_dp: 22.0,
    line_height_dp: 28.0,
    letter_spacing_dp: 0.0,
};

/// Title Medium — medium emphasis (list item titles, card titles).
pub const TITLE_MEDIUM: TypeStyle = TypeStyle {
    font: "Google Sans Flex",
    weight: 500,
    size_dp: 16.0,
    line_height_dp: 24.0,
    letter_spacing_dp: 0.15,
};

/// Title Small — smaller component labels (tab labels, chip labels).
pub const TITLE_SMALL: TypeStyle = TypeStyle {
    font: "Google Sans Flex",
    weight: 500,
    size_dp: 14.0,
    line_height_dp: 20.0,
    letter_spacing_dp: 0.1,
};

// ── Body ────────────────────────────────────────────────────────────────────

/// Body Large — default body copy, readable at length.
pub const BODY_LARGE: TypeStyle = TypeStyle {
    font: "Google Sans Flex",
    weight: 400,
    size_dp: 16.0,
    line_height_dp: 24.0,
    letter_spacing_dp: 0.5,
};

/// Body Medium — compact body copy (supporting text, captions with detail).
pub const BODY_MEDIUM: TypeStyle = TypeStyle {
    font: "Google Sans Flex",
    weight: 400,
    size_dp: 14.0,
    line_height_dp: 20.0,
    letter_spacing_dp: 0.25,
};

/// Body Small — smallest body copy (captions, helper text).
pub const BODY_SMALL: TypeStyle = TypeStyle {
    font: "Google Sans Flex",
    weight: 400,
    size_dp: 12.0,
    line_height_dp: 16.0,
    letter_spacing_dp: 0.4,
};

// ── Label ───────────────────────────────────────────────────────────────────

/// Label Large — prominent label for actionable elements (buttons).
pub const LABEL_LARGE: TypeStyle = TypeStyle {
    font: "Google Sans Flex",
    weight: 500,
    size_dp: 14.0,
    line_height_dp: 20.0,
    letter_spacing_dp: 0.1,
};

/// Label Medium — for icon labels, tab bar labels.
pub const LABEL_MEDIUM: TypeStyle = TypeStyle {
    font: "Google Sans Flex",
    weight: 500,
    size_dp: 12.0,
    line_height_dp: 16.0,
    letter_spacing_dp: 0.5,
};

/// Label Small — smallest label, used sparingly (badges, annotation).
pub const LABEL_SMALL: TypeStyle = TypeStyle {
    font: "Google Sans Flex",
    weight: 500,
    size_dp: 11.0,
    line_height_dp: 16.0,
    letter_spacing_dp: 0.5,
};

/// All 15 M3 type styles in canonical specification order.
///
/// Order: Display (L/M/S), Headline (L/M/S), Title (L/M/S),
///        Body (L/M/S), Label (L/M/S).
pub const ALL: [TypeStyle; 15] = [
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

/// Kebab-case names for each token in [`ALL`] order.
pub const NAMES: [&str; 15] = [
    "display-large", "display-medium", "display-small",
    "headline-large", "headline-medium", "headline-small",
    "title-large", "title-medium", "title-small",
    "body-large", "body-medium", "body-small",
    "label-large", "label-medium", "label-small",
];

/// Emits a CSS `:root` block declaring all M3 typography custom properties.
#[cfg(feature = "std")]
#[must_use]
pub fn export_css() -> std::string::String {
    let mut out = std::string::String::with_capacity(2048);
    out.push_str(":root {\n");
    for (style, name) in ALL.iter().zip(NAMES.iter()) {
        out.push_str(&std::format!(
            "  --md-sys-typescale-{name}-font-family: '{}';\n\
             \x20 --md-sys-typescale-{name}-font-weight: {weight};\n\
             \x20 --md-sys-typescale-{name}-size: {size}px;\n\
             \x20 --md-sys-typescale-{name}-line-height: {line_height}px;\n\
             \x20 --md-sys-typescale-{name}-letter-spacing: {letter_spacing}px;\n",
            style.font,
            name = name,
            weight = style.weight,
            size = style.size_dp,
            line_height = style.line_height_dp,
            letter_spacing = style.letter_spacing_dp,
        ));
    }
    out.push('}');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_scale_has_exactly_15_styles() {
        assert_eq!(ALL.len(), 15, "M3 type scale must have exactly 15 styles");
    }

    #[test]
    fn display_large_matches_spec() {
        assert_eq!(DISPLAY_LARGE.size_dp, 57.0);
        assert_eq!(DISPLAY_LARGE.line_height_dp, 64.0);
        assert_eq!(DISPLAY_LARGE.weight, 400);
    }

    #[test]
    fn label_large_matches_spec() {
        assert_eq!(LABEL_LARGE.size_dp, 14.0);
        assert_eq!(LABEL_LARGE.weight, 500);
    }

    #[test]
    fn all_fonts_are_google_sans_flex() {
        for style in &ALL {
            assert_eq!(style.font, "Google Sans Flex");
        }
    }

    #[test]
    fn all_sizes_are_positive() {
        for style in &ALL {
            assert!(style.size_dp > 0.0);
            assert!(style.line_height_dp > 0.0);
        }
    }
}
