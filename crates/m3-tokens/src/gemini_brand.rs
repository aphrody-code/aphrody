// SPDX-License-Identifier: Apache-2.0
//! Gemini AI brand tokens.
//!
//! Reference: <https://design.google/library/gemini-ai-visual-design>.
//!
//! The Gemini brand layer sits on top of the M3 baseline color cascade
//! and contributes the signature elements that distinguish the product
//! from generic Material Design surfaces:
//!
//! - The canonical blue-to-purple-to-pink "spectrum-shift" gradient used on the prompt-send
//!   affordance, the empty-state sparkle, the user avatar ring, and the "thinking" streaming
//!   indicator.
//! - The four sparkle stops (the visual analogue of the Google four-color dot lineage cited in the
//!   article).
//! - The rounded-pill brand-shape constants (Gemini's "warm, spatial, rounded quality" per Anna
//!   Sera Garcia).
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

/// Aphrody-extension cyan tone — used by the terminal demo for cool-status
/// affordances (info chips, secondary metrics). Sourced from Material You
/// teal-400 (`#00BCD4`).
pub const APHRODY_CYAN: u32 = 0xFF00_BCD4;

/// Aphrody-extension orange tone — used for warn-status affordances and
/// "in-flight" markers in the terminal demo. Sourced from Material You
/// deep-orange-400 (`#FF7043`).
pub const APHRODY_ORANGE: u32 = 0xFFFF_7043;

/// Aphrody-extension red tone — used for error-status affordances. Sourced
/// from Material You red-600 (`#E53935`).
pub const APHRODY_RED: u32 = 0xFFE5_3935;

/// Companion deep-green tone used as the start of the "success" gradient
/// (Material You green-600 `#34A853`).
pub const APHRODY_GREEN_DEEP: u32 = 0xFF34_A853;

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

const NAN_STOP: GradientStop = GradientStop { position: f32::NAN, argb: 0 };

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
        parts.push(format!("{} {:.0}%", argb_to_css(stop.argb), stop.position * 100.0));
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
    s.push_str(&format!("  --gemini-brand-purple: {};\n", argb_to_css(GEMINI_PURPLE)));
    s.push_str(&format!("  --gemini-brand-pink: {};\n", argb_to_css(GEMINI_PINK)));
    s.push_str(&format!("  --gemini-brand-yellow: {};\n", argb_to_css(GEMINI_YELLOW)));
    s.push_str(&format!("  --gemini-brand-green: {};\n", argb_to_css(GEMINI_GREEN)));
    s.push_str(&format!(
        "  --gemini-spectrum-shift: {};\n",
        gradient_to_css(&SPECTRUM_SHIFT_GRADIENT)
    ));
    s.push_str(&format!("  --gemini-sparkle: {};\n", gradient_to_css(&SPARKLE_GRADIENT)));
    s.push_str(&format!("  --gemini-warm-tone: {};\n", gradient_to_css(&WARM_TONE_GRADIENT)));
    s.push_str(&format!("  --gemini-corner-prompt-bar: {}px;\n", BRAND_CORNER_PROMPT_BAR_PX));
    s.push_str(&format!("  --gemini-corner-message: {}px;\n", BRAND_CORNER_MESSAGE_PX));
    s.push_str(&format!("  --gemini-corner-chip: {}px;\n", BRAND_CORNER_CHIP_PX));
    s.push_str("}\n");
    s
}

/// Emit a comprehensive `:root` block exposing the **Aphrody** brand cascade
/// consumed by the HTML/WASM examples (notably
/// `crates/aphrody-wasm/examples/aphrody-terminal-demo.html`, which used to
/// hand-mirror these tokens).
///
/// The emitted CSS contains, in this order:
///
/// 1. Eight `--aphrody-brand-*` color tokens (blue, purple, pink, yellow,
///    green, cyan, orange, red) — uppercase `#RRGGBB`.
/// 2. Four `--aphrody-*-tone` / `--aphrody-spectrum-shift` linear gradients
///    (spectrum-shift, warm-tone, success-tone, shimmer).
/// 3. Terminal monospace font stack (`--aphrody-mono-font`).
/// 4. Three corner radii (`--aphrody-corner-card`, `-pill`, `-chip`).
/// 5. The companion M3 baseline scale referenced by the HTML demo
///    (`--m3-color-primary` family + `--m3-corner-*` + `--m3-typescale-*`).
///
/// All values are computed from the Rust constants in this module and from
/// the M3 baseline cascade ([`crate::color::BASELINE`], [`crate::shape`],
/// [`crate::typography`]) so that the HTML examples never drift out of sync
/// with the canonical source.
#[cfg(feature = "std")]
#[must_use]
pub fn export_aphrody_brand_css() -> String {
    use crate::color::BASELINE;
    use crate::shape as sh;
    use crate::typography as ty;

    // Uppercase hex helper, matching the existing HTML convention.
    fn hex_upper(argb: u32) -> String {
        format!("#{:06X}", argb & 0x00_FF_FF_FF)
    }

    let mut s = String::with_capacity(4096);
    s.push_str(":root {\n");

    // ── M3 baseline color cascade (mirrors crates/m3-tokens/src/color.rs) ──
    s.push_str("  /* M3 baseline color roles (crates/m3-tokens/src/color.rs) */\n");
    let m3_pairs: [(&str, u32); 14] = [
        ("primary", BASELINE.primary),
        ("on-primary", BASELINE.on_primary),
        ("primary-container", BASELINE.primary_container),
        ("on-primary-container", BASELINE.on_primary_container),
        ("secondary", BASELINE.secondary),
        ("on-secondary", BASELINE.on_secondary),
        ("secondary-container", BASELINE.secondary_container),
        ("on-secondary-container", BASELINE.on_secondary_container),
        ("tertiary", BASELINE.tertiary),
        ("background", BASELINE.background),
        ("on-background", BASELINE.on_background),
        ("surface", BASELINE.surface),
        ("on-surface", BASELINE.on_surface),
        ("outline", BASELINE.outline),
    ];
    for (name, argb) in m3_pairs {
        s.push_str(&format!("  --m3-color-{name}: {};\n", hex_upper(argb)));
    }

    // ── M3 corner radius scale (mirrors crates/m3-tokens/src/shape.rs) ─────
    s.push_str("\n  /* M3 shape corner scale (crates/m3-tokens/src/shape.rs) */\n");
    let m3_corners: [(&str, u16); 6] = [
        ("xs", sh::EXTRA_SMALL.dp),
        ("sm", sh::SMALL.dp),
        ("md", sh::MEDIUM.dp),
        ("lg", sh::LARGE.dp),
        ("xl", sh::EXTRA_LARGE.dp),
        ("full", sh::FULL.dp),
    ];
    for (name, dp) in m3_corners {
        s.push_str(&format!("  --m3-corner-{name}: {dp}px;\n"));
    }

    // ── M3 typescale (mirrors crates/m3-tokens/src/typography.rs) ──────────
    s.push_str("\n  /* M3 typescale (crates/m3-tokens/src/typography.rs) */\n");
    let m3_type: [(&str, ty::TypeStyle); 15] = [
        ("display-large", ty::DISPLAY_LARGE),
        ("display-medium", ty::DISPLAY_MEDIUM),
        ("display-small", ty::DISPLAY_SMALL),
        ("headline-large", ty::HEADLINE_LARGE),
        ("headline-medium", ty::HEADLINE_MEDIUM),
        ("headline-small", ty::HEADLINE_SMALL),
        ("title-large", ty::TITLE_LARGE),
        ("title-medium", ty::TITLE_MEDIUM),
        ("title-small", ty::TITLE_SMALL),
        ("body-large", ty::BODY_LARGE),
        ("body-medium", ty::BODY_MEDIUM),
        ("body-small", ty::BODY_SMALL),
        ("label-large", ty::LABEL_LARGE),
        ("label-medium", ty::LABEL_MEDIUM),
        ("label-small", ty::LABEL_SMALL),
    ];
    for (name, style) in m3_type {
        s.push_str(&format!(
            "  --m3-typescale-{name}-size: {}px;\n",
            style.size_dp as u32
        ));
        s.push_str(&format!(
            "  --m3-typescale-{name}-line-height: {}px;\n",
            style.line_height_dp as u32
        ));
        s.push_str(&format!("  --m3-typescale-{name}-weight: {};\n", style.weight));
    }

    // ── Aphrody brand colors (extends gemini_brand canonical 5) ────────────
    s.push_str("\n  /* Aphrody brand palette (crates/m3-tokens/src/gemini_brand.rs) */\n");
    let brand_colors: [(&str, u32); 8] = [
        ("blue", GEMINI_BLUE),
        ("purple", GEMINI_PURPLE),
        ("pink", GEMINI_PINK),
        ("yellow", GEMINI_YELLOW),
        ("green", GEMINI_GREEN),
        ("cyan", APHRODY_CYAN),
        ("orange", APHRODY_ORANGE),
        ("red", APHRODY_RED),
    ];
    for (name, argb) in brand_colors {
        s.push_str(&format!("  --aphrody-brand-{name}: {};\n", hex_upper(argb)));
    }

    // ── Aphrody gradients (spectrum-shift, warm-tone, success-tone, shimmer) ─
    s.push_str("\n  /* Aphrody brand gradients */\n");
    s.push_str(
        "  --aphrody-spectrum-shift: linear-gradient(90deg, \
         var(--aphrody-brand-blue) 0%, \
         var(--aphrody-brand-purple) 50%, \
         var(--aphrody-brand-pink) 100%);\n",
    );
    s.push_str(
        "  --aphrody-warm-tone: linear-gradient(45deg, \
         var(--aphrody-brand-yellow) 0%, \
         var(--aphrody-brand-pink) 100%);\n",
    );
    s.push_str(&format!(
        "  --aphrody-success-tone: linear-gradient(135deg, \
         {} 0%, \
         var(--aphrody-brand-green) 100%);\n",
        hex_upper(APHRODY_GREEN_DEEP),
    ));
    s.push_str(
        "  --aphrody-shimmer: linear-gradient(90deg, \
         transparent 0%, \
         color-mix(in srgb, white 35%, transparent) 50%, \
         transparent 100%);\n",
    );

    // ── Aphrody terminal-specific tokens ───────────────────────────────────
    s.push_str("\n  /* Aphrody terminal-specific tokens */\n");
    s.push_str(
        "  --aphrody-mono-font: 'JetBrains Mono', 'Cascadia Code', 'SF Mono', \
         'Menlo', 'Consolas', 'Liberation Mono', monospace;\n",
    );
    s.push_str(&format!("  --aphrody-corner-card: {}px;\n", sh::LARGE.dp));
    s.push_str(&format!("  --aphrody-corner-pill: {}px;\n", sh::FULL.dp));
    s.push_str(&format!("  --aphrody-corner-chip: {}px;\n", sh::MEDIUM.dp));

    s.push_str("}\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brand_colors_are_opaque() {
        for argb in [GEMINI_BLUE, GEMINI_PURPLE, GEMINI_PINK, GEMINI_YELLOW, GEMINI_GREEN] {
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

    #[cfg(feature = "std")]
    #[test]
    fn aphrody_extension_colors_are_opaque() {
        for argb in [APHRODY_CYAN, APHRODY_ORANGE, APHRODY_RED, APHRODY_GREEN_DEEP] {
            assert_eq!(argb >> 24, 0xFF, "expected opaque ARGB: 0x{argb:08X}");
        }
    }

    #[cfg(feature = "std")]
    #[test]
    fn export_aphrody_brand_css_well_formed() {
        let css = export_aphrody_brand_css();

        // Structural framing
        assert!(css.starts_with(":root {"), "CSS must open with `:root {{`, got: {css:.80}");
        assert!(css.trim_end().ends_with('}'), "CSS must end with `}}` close brace");

        // No Rust formatting leftovers (escaped braces leaking into output)
        assert!(!css.contains("{{"), "unexpected `{{{{` in CSS — formatter leak");
        assert!(!css.contains("}}"), "unexpected `}}}}` in CSS — formatter leak");

        // Required token families
        let required = [
            // M3 baseline colors
            "--m3-color-primary:",
            "--m3-color-on-primary:",
            "--m3-color-secondary:",
            "--m3-color-surface:",
            // M3 corners
            "--m3-corner-xs:",
            "--m3-corner-sm:",
            "--m3-corner-md:",
            "--m3-corner-lg:",
            "--m3-corner-xl:",
            "--m3-corner-full:",
            // M3 typescale
            "--m3-typescale-display-large-size:",
            "--m3-typescale-display-large-line-height:",
            "--m3-typescale-display-large-weight:",
            "--m3-typescale-body-medium-size:",
            "--m3-typescale-label-small-weight:",
            // Aphrody brand colors (all 8)
            "--aphrody-brand-blue:",
            "--aphrody-brand-purple:",
            "--aphrody-brand-pink:",
            "--aphrody-brand-yellow:",
            "--aphrody-brand-green:",
            "--aphrody-brand-cyan:",
            "--aphrody-brand-orange:",
            "--aphrody-brand-red:",
            // Aphrody gradients
            "--aphrody-spectrum-shift:",
            "--aphrody-warm-tone:",
            "--aphrody-success-tone:",
            "--aphrody-shimmer:",
            // Terminal tokens
            "--aphrody-mono-font:",
            "--aphrody-corner-card:",
            "--aphrody-corner-pill:",
            "--aphrody-corner-chip:",
        ];
        for token in required {
            assert!(css.contains(token), "expected `{token}` in export_aphrody_brand_css output");
        }

        // Hand-mirrored values from the HTML demo must match exactly
        assert!(css.contains("--aphrody-brand-blue: #4285F4;"));
        assert!(css.contains("--aphrody-brand-purple: #9168C0;"));
        assert!(css.contains("--aphrody-brand-pink: #EC4899;"));
        assert!(css.contains("--aphrody-brand-cyan: #00BCD4;"));
        assert!(css.contains("--aphrody-brand-orange: #FF7043;"));
        assert!(css.contains("--aphrody-brand-red: #E53935;"));

        // M3 primary baseline (purple seed #6750A4)
        assert!(css.contains("--m3-color-primary: #6750A4;"));

        // M3 display-large size = 57dp per spec
        assert!(css.contains("--m3-typescale-display-large-size: 57px;"));

        // Token-line count sanity (≥ 30 declarations — not a stub)
        let decl_lines = css.lines().filter(|l| l.contains(": ") && l.trim().ends_with(';')).count();
        assert!(
            decl_lines >= 30,
            "expected at least 30 CSS token declarations, got {decl_lines}"
        );

        // Single :root opener / single closer (well-formed block)
        assert_eq!(css.matches(":root {").count(), 1, "exactly one `:root {{` block expected");
    }
}
