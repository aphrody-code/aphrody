// SPDX-License-Identifier: Apache-2.0
//! Gemini AI brand tokens.
//!
//! Reference: <https://design.google/library/gemini-ai-visual-design>.
//!
//! The Gemini brand layer sits on top of the M3 baseline color cascade
//! and contributes the signature elements that distinguish the product
//! from generic Material Design surfaces:
//!
//! - The canonical blue-to-purple-to-pink "spectrum-shift" gradient
//!   used on the prompt-send affordance, the empty-state sparkle, the
//!   user avatar ring, and the "thinking" streaming indicator.
//! - The four sparkle stops (the visual analogue of the Google
//!   four-color dot lineage cited in the article).
//! - The rounded-pill brand-shape constants (Gemini's
//!   "warm, spatial, rounded quality" per Anna Sera Garcia).
//!
//! All ARGB constants use `0xFFRRGGBB` (alpha-first u32) to match the
//! pattern established in [`crate::color`] and [`crate::tonal`].

/// One stop in a multi-color gradient.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GradientStop {
    /// 0.0 = start, 1.0 = end of the gradient axis.
    pub position: f32,
    /// ARGB color in `0xFFRRGGBB` form (alpha-opaque by default).
    pub argb: u32,
}

/// A linear gradient parameterised by its angle (degrees, 0 = east, CSS
/// convention) and its ordered stop list.
#[derive(Debug, Clone, Copy)]
pub struct LinearGradient {
    /// Direction in CSS `linear-gradient` degrees. 90 = top-to-bottom,
    /// 45 = top-left to bottom-right, etc.
    pub angle_deg: f32,
    /// Up to 6 stops. Use 0xFF000000 + position 0.0 + count <= 6 for
    /// truncated palettes; trailing unused stops are flagged at
    /// `position == f32::NAN` so consumers can skip them.
    pub stops: [GradientStop; 6],
    /// Number of populated stops in [`Self::stops`].
    pub count: u8,
}

// ---------------------------------------------------------------------------
// Canonical Gemini brand colors
// ---------------------------------------------------------------------------

/// Gemini brand blue (canonical: matches `#4285F4`, Google blue 500).
pub const GEMINI_BLUE: u32 = 0xFF42_85F4;

/// Gemini brand purple (canonical: matches `#9168C0`, the violet midpoint
/// in the spectrum-shift gradient documented in the design.google article).
pub const GEMINI_PURPLE: u32 = 0xFF91_68C0;

/// Gemini brand pink (canonical: matches `#EC4899`, the warm endpoint of
/// the spectrum-shift gradient).
pub const GEMINI_PINK: u32 = 0xFFEC_4899;

/// Gemini accent yellow (the warm-tone counterpart documented in the
/// design.google article color sampling).
pub const GEMINI_YELLOW: u32 = 0xFFFA_E366;

/// Gemini accent green (rounded, optimistic counterpart per Anna Sera
/// Garcia's "warm, spatial, rounded quality" description).
pub const GEMINI_GREEN: u32 = 0xFFBF_F28D;

/// The four-color dot lineage colors cited in the article as the visual
/// reference point for Gemini's rounded language. Order: blue, red,
/// yellow, green — matches the Google logo lineage.
pub const FOUR_COLOR_DOTS: [u32; 4] = [
    0xFF42_85F4, // blue
    0xFFEA_4335, // red
    0xFFFB_BC04, // yellow
    0xFF34_A853, // green
];

// ---------------------------------------------------------------------------
// Canonical Gemini gradients
// ---------------------------------------------------------------------------

const NAN_STOP: GradientStop = GradientStop {
    position: f32::NAN,
    argb: 0,
};

/// The signature 3-stop blue-to-purple-to-pink gradient used on the
/// prompt-send affordance, sparkle, and "thinking" streaming indicator.
pub const SPECTRUM_SHIFT_GRADIENT: LinearGradient = LinearGradient {
    angle_deg: 90.0,
    stops: [
        GradientStop { position: 0.00, argb: GEMINI_BLUE },
        GradientStop { position: 0.50, argb: GEMINI_PURPLE },
        GradientStop { position: 1.00, argb: GEMINI_PINK },
        NAN_STOP,
        NAN_STOP,
        NAN_STOP,
    ],
    count: 3,
};

/// The 4-color sparkle gradient used on the empty-state Gemini logo,
/// matching the four-color dot lineage. Used radially in practice.
pub const SPARKLE_GRADIENT: LinearGradient = LinearGradient {
    angle_deg: 135.0,
    stops: [
        GradientStop { position: 0.00, argb: FOUR_COLOR_DOTS[0] },
        GradientStop { position: 0.33, argb: FOUR_COLOR_DOTS[1] },
        GradientStop { position: 0.66, argb: FOUR_COLOR_DOTS[2] },
        GradientStop { position: 1.00, argb: FOUR_COLOR_DOTS[3] },
        NAN_STOP,
        NAN_STOP,
    ],
    count: 4,
};

/// The warm-tone gradient documented in the article as the rounded
/// optimistic counterpart (yellow → soft pink for empty-state warmth).
pub const WARM_TONE_GRADIENT: LinearGradient = LinearGradient {
    angle_deg: 45.0,
    stops: [
        GradientStop { position: 0.00, argb: GEMINI_YELLOW },
        GradientStop { position: 1.00, argb: GEMINI_PINK },
        NAN_STOP,
        NAN_STOP,
        NAN_STOP,
        NAN_STOP,
    ],
    count: 2,
};

/// Rounded brand-shape corner radius (px) — the article's "rounded
/// foundational shapes" guideline. Larger than M3 [`crate::shape::FULL`]
/// pill use cases would suggest, because Gemini uses generous radii on
/// containers (prompt bar, message bubbles) for a softer feel.
pub const BRAND_CORNER_PROMPT_BAR_PX: u16 = 28;

/// Rounded brand-shape corner radius for chat message bubbles.
pub const BRAND_CORNER_MESSAGE_PX: u16 = 24;

/// Rounded brand-shape corner radius for suggestion chips.
pub const BRAND_CORNER_CHIP_PX: u16 = 18;

/// Convert a `0xFFRRGGBB` ARGB value to a CSS `#RRGGBB` string slice
/// (allocates only when the `std` feature is on).
#[cfg(feature = "std")]
#[must_use]
pub fn argb_to_css(argb: u32) -> String {
    format!("#{:06x}", argb & 0x00_FF_FF_FF)
}

/// Serialize a [`LinearGradient`] as a CSS `linear-gradient(...)` string.
#[cfg(feature = "std")]
#[must_use]
pub fn gradient_to_css(gradient: &LinearGradient) -> String {
    let mut parts: Vec<String> = Vec::with_capacity(usize::from(gradient.count) + 1);
    parts.push(format!("{:.0}deg", gradient.angle_deg));
    for stop in gradient.stops.iter().take(usize::from(gradient.count)) {
        if stop.position.is_nan() {
            continue;
        }
        parts.push(format!(
            "{} {:.0}%",
            argb_to_css(stop.argb),
            stop.position * 100.0
        ));
    }
    format!("linear-gradient({})", parts.join(", "))
}

/// Emit a `:root` block with the Gemini brand custom properties.
///
/// The returned CSS exposes:
///   --gemini-brand-blue, --gemini-brand-purple, --gemini-brand-pink,
///   --gemini-brand-yellow, --gemini-brand-green,
///   --gemini-spectrum-shift, --gemini-sparkle, --gemini-warm-tone,
///   --gemini-corner-prompt-bar, --gemini-corner-message, --gemini-corner-chip.
#[cfg(feature = "std")]
#[must_use]
pub fn export_css() -> String {
    let mut s = String::with_capacity(1024);
    s.push_str(":root {\n");
    s.push_str(&format!("  --gemini-brand-blue: {};\n", argb_to_css(GEMINI_BLUE)));
    s.push_str(&format!(
        "  --gemini-brand-purple: {};\n",
        argb_to_css(GEMINI_PURPLE)
    ));
    s.push_str(&format!("  --gemini-brand-pink: {};\n", argb_to_css(GEMINI_PINK)));
    s.push_str(&format!(
        "  --gemini-brand-yellow: {};\n",
        argb_to_css(GEMINI_YELLOW)
    ));
    s.push_str(&format!(
        "  --gemini-brand-green: {};\n",
        argb_to_css(GEMINI_GREEN)
    ));
    s.push_str(&format!(
        "  --gemini-spectrum-shift: {};\n",
        gradient_to_css(&SPECTRUM_SHIFT_GRADIENT)
    ));
    s.push_str(&format!(
        "  --gemini-sparkle: {};\n",
        gradient_to_css(&SPARKLE_GRADIENT)
    ));
    s.push_str(&format!(
        "  --gemini-warm-tone: {};\n",
        gradient_to_css(&WARM_TONE_GRADIENT)
    ));
    s.push_str(&format!(
        "  --gemini-corner-prompt-bar: {}px;\n",
        BRAND_CORNER_PROMPT_BAR_PX
    ));
    s.push_str(&format!(
        "  --gemini-corner-message: {}px;\n",
        BRAND_CORNER_MESSAGE_PX
    ));
    s.push_str(&format!(
        "  --gemini-corner-chip: {}px;\n",
        BRAND_CORNER_CHIP_PX
    ));
    s.push_str("}\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brand_colors_are_opaque() {
        for argb in [
            GEMINI_BLUE,
            GEMINI_PURPLE,
            GEMINI_PINK,
            GEMINI_YELLOW,
            GEMINI_GREEN,
        ] {
            assert_eq!(argb >> 24, 0xFF, "expected opaque ARGB: 0x{argb:08X}");
        }
    }

    #[test]
    fn four_color_dots_match_google_logo_lineage() {
        assert_eq!(FOUR_COLOR_DOTS, [0xFF4285F4, 0xFFEA4335, 0xFFFBBC04, 0xFF34A853]);
    }

    #[test]
    fn spectrum_shift_gradient_has_3_stops() {
        assert_eq!(SPECTRUM_SHIFT_GRADIENT.count, 3);
        assert_eq!(SPECTRUM_SHIFT_GRADIENT.stops[0].argb, GEMINI_BLUE);
        assert_eq!(SPECTRUM_SHIFT_GRADIENT.stops[1].argb, GEMINI_PURPLE);
        assert_eq!(SPECTRUM_SHIFT_GRADIENT.stops[2].argb, GEMINI_PINK);
    }

    #[test]
    fn sparkle_gradient_uses_four_color_dots() {
        assert_eq!(SPARKLE_GRADIENT.count, 4);
        for (i, &dot) in FOUR_COLOR_DOTS.iter().enumerate() {
            assert_eq!(SPARKLE_GRADIENT.stops[i].argb, dot);
        }
    }

    #[test]
    fn brand_corners_are_rounded() {
        assert!(BRAND_CORNER_PROMPT_BAR_PX >= 24);
        assert!(BRAND_CORNER_MESSAGE_PX >= 16);
        assert!(BRAND_CORNER_CHIP_PX >= 12);
    }

    #[cfg(feature = "std")]
    #[test]
    fn argb_to_css_strips_alpha() {
        assert_eq!(argb_to_css(0xFF4285F4), "#4285f4");
        assert_eq!(argb_to_css(0xFF000000), "#000000");
        assert_eq!(argb_to_css(0xFFFFFFFF), "#ffffff");
    }

    #[cfg(feature = "std")]
    #[test]
    fn gradient_to_css_renders_canonical_spectrum_shift() {
        let css = gradient_to_css(&SPECTRUM_SHIFT_GRADIENT);
        assert!(css.contains("90deg"), "expected 90deg axis: {css}");
        assert!(css.contains("#4285f4"), "expected blue stop: {css}");
        assert!(css.contains("#9168c0"), "expected purple stop: {css}");
        assert!(css.contains("#ec4899"), "expected pink stop: {css}");
    }

    #[cfg(feature = "std")]
    #[test]
    fn export_css_contains_required_tokens() {
        let css = export_css();
        for token in [
            "--gemini-brand-blue",
            "--gemini-brand-purple",
            "--gemini-brand-pink",
            "--gemini-spectrum-shift",
            "--gemini-sparkle",
            "--gemini-corner-prompt-bar",
        ] {
            assert!(css.contains(token), "expected `{token}` in export_css output");
        }
    }
}
