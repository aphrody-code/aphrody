// SPDX-License-Identifier: Apache-2.0
//! Google Sans Flex variable-font axis tokens.
//!
//! Reference: <https://design.google/library/google-sans-flex-font>.
//!
//! Google Sans Flex is the variable-font sibling of Google Sans, shipping
//! six interpolation axes in a single ~4 MB TTF:
//!
//! | Axis tag | Description                                | Min  | Default | Max   |
//! |----------|--------------------------------------------|------|---------|-------|
//! | `wght`   | Weight (CSS `font-weight`)                 |  100 |  400    |  900  |
//! | `opsz`   | Optical size (target rendering size, pt)   |    9 |   14    |  144  |
//! | `wdth`   | Width (relative percentage)                |   75 |  100    |  125  |
//! | `GRAD`   | Grade (weight without changing line width) | -200 |    0    |  150  |
//! | `slnt`   | Slant (degrees, negative = italic lean)    |  -10 |    0    |    0  |
//! | `ROND`   | Roundness (terminal shape continuum)       |    0 |    0    |  100  |
//!
//! The crate exposes the axes as compile-time constants plus helpers to
//! produce a CSS `font-variation-settings` value from a typed
//! [`AxisSettings`] tuple.  Cross-platform, `no_std`-friendly (CSS export
//! gated behind `feature = "std"`).
//!
//! The font binary itself lives at `assets/fonts/google-sans-flex/` in the
//! aphrody workspace and is shipped under the SIL Open Font License v1.1
//! (`assets/fonts/google-sans-flex/OFL.txt`).

/// One variable-font axis descriptor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Axis {
    /// 4-char OpenType axis tag (e.g. `"wght"`, `"opsz"`, `"GRAD"`).
    pub tag: &'static str,
    /// Lower bound of the legal range.
    pub min: f32,
    /// The font's default value for this axis.
    pub default: f32,
    /// Upper bound of the legal range.
    pub max: f32,
}

impl Axis {
    /// Clamp `value` into `[self.min, self.max]`.
    #[must_use]
    pub fn clamp(&self, value: f32) -> f32 {
        if value < self.min {
            self.min
        } else if value > self.max {
            self.max
        } else {
            value
        }
    }
}

/// Weight axis (CSS `font-weight`).
pub const WGHT: Axis = Axis { tag: "wght", min: 100.0, default: 400.0, max: 900.0 };

/// Optical size axis (target render size in points).
pub const OPSZ: Axis = Axis { tag: "opsz", min: 9.0, default: 14.0, max: 144.0 };

/// Width axis (relative percentage; 100 = neutral).
pub const WDTH: Axis = Axis { tag: "wdth", min: 75.0, default: 100.0, max: 125.0 };

/// Grade axis (weight nudge that does not affect line metrics).
pub const GRAD: Axis = Axis { tag: "GRAD", min: -200.0, default: 0.0, max: 150.0 };

/// Slant axis (degrees; -10 = italic lean, 0 = upright).
pub const SLNT: Axis = Axis { tag: "slnt", min: -10.0, default: 0.0, max: 0.0 };

/// Roundness axis (0 = sharp terminals, 100 = fully rounded).
pub const ROND: Axis = Axis { tag: "ROND", min: 0.0, default: 0.0, max: 100.0 };

/// All 6 axes in canonical declaration order.
pub const ALL_AXES: [&Axis; 6] = [&WGHT, &OPSZ, &WDTH, &GRAD, &SLNT, &ROND];

// ---------------------------------------------------------------------------
// AxisSettings — a typed sextuple of current axis values.
// ---------------------------------------------------------------------------

/// Current value for each variable axis. Each field is clamped on
/// construction via [`Self::clamped`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisSettings {
    /// Weight (`wght`).
    pub wght: f32,
    /// Optical size (`opsz`).
    pub opsz: f32,
    /// Width (`wdth`).
    pub wdth: f32,
    /// Grade (`GRAD`).
    pub grad: f32,
    /// Slant (`slnt`).
    pub slnt: f32,
    /// Roundness (`ROND`).
    pub rond: f32,
}

impl AxisSettings {
    /// The font's default axis values (matches the static "Regular" cut).
    pub const DEFAULT: Self =
        Self { wght: 400.0, opsz: 14.0, wdth: 100.0, grad: 0.0, slnt: 0.0, rond: 0.0 };

    /// Construct, clamping every field into its axis range.
    #[must_use]
    pub fn clamped(self) -> Self {
        Self {
            wght: WGHT.clamp(self.wght),
            opsz: OPSZ.clamp(self.opsz),
            wdth: WDTH.clamp(self.wdth),
            grad: GRAD.clamp(self.grad),
            slnt: SLNT.clamp(self.slnt),
            rond: ROND.clamp(self.rond),
        }
    }
}

impl Default for AxisSettings {
    fn default() -> Self {
        Self::DEFAULT
    }
}

// ---------------------------------------------------------------------------
// CSS export (std-only)
// ---------------------------------------------------------------------------

/// Produce a CSS `font-variation-settings` value from an [`AxisSettings`].
///
/// Output format follows the CSS spec: `"tag" value, "tag" value, ...`
/// — for example `"wght" 500, "opsz" 24, "ROND" 60`.
#[cfg(feature = "std")]
#[must_use]
pub fn font_variation_settings(s: &AxisSettings) -> String {
    let s = s.clamped();
    format!(
        "\"wght\" {wght:.0}, \"opsz\" {opsz:.0}, \"wdth\" {wdth:.0}, \"GRAD\" {grad:.0}, \"slnt\" \
         {slnt:.0}, \"ROND\" {rond:.0}",
        wght = s.wght,
        opsz = s.opsz,
        wdth = s.wdth,
        grad = s.grad,
        slnt = s.slnt,
        rond = s.rond,
    )
}

/// Emit a `@font-face` declaration block pointing at the variable font
/// shipped at
/// `assets/fonts/google-sans-flex/GoogleSansFlex-VariableFont_GRAD,ROND,opsz,slnt,wdth,wght.ttf`.
///
/// `relative_url` is the URL prefix used in the consumer page (e.g.
/// `"../../../assets/fonts/google-sans-flex/"` for the
/// `crates/aphrody-wasm/examples/` HTML demos).
#[cfg(feature = "std")]
#[must_use]
pub fn export_font_face(relative_url: &str) -> String {
    let mut s = String::with_capacity(512);
    s.push_str("@font-face {\n");
    s.push_str("  font-family: 'Google Sans Flex';\n");
    s.push_str(&format!(
        "  src: url('{relative_url}GoogleSansFlex-VariableFont_GRAD,ROND,opsz,slnt,wdth,wght.ttf'\
         ) format('truetype-variations');\n"
    ));
    s.push_str("  font-weight: 100 900;\n");
    s.push_str("  font-stretch: 75% 125%;\n");
    s.push_str("  font-style: oblique -10deg 0deg;\n");
    s.push_str("  font-display: swap;\n");
    s.push_str("}\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn axis_count_is_six() {
        assert_eq!(ALL_AXES.len(), 6, "Google Sans Flex ships 6 axes");
    }

    #[test]
    fn axis_tags_match_filename_order() {
        // The variable-font filename lists axes in this order:
        // GRAD,ROND,opsz,slnt,wdth,wght
        // ALL_AXES uses a different (more user-facing) order, but every
        // tag from the filename must be present.
        let tags: [&str; 6] = [
            ALL_AXES[0].tag,
            ALL_AXES[1].tag,
            ALL_AXES[2].tag,
            ALL_AXES[3].tag,
            ALL_AXES[4].tag,
            ALL_AXES[5].tag,
        ];
        for required in ["wght", "opsz", "wdth", "GRAD", "slnt", "ROND"] {
            assert!(
                tags.contains(&required),
                "axis `{required}` must be present in ALL_AXES"
            );
        }
    }

    #[test]
    fn axis_clamp_respects_bounds() {
        assert_eq!(WGHT.clamp(50.0), 100.0);
        assert_eq!(WGHT.clamp(1000.0), 900.0);
        assert_eq!(WGHT.clamp(500.0), 500.0);
        assert_eq!(SLNT.clamp(-20.0), -10.0);
        assert_eq!(SLNT.clamp(5.0), 0.0);
    }

    #[test]
    fn axis_settings_default_matches_regular_cut() {
        let s = AxisSettings::default();
        assert_eq!(s.wght, 400.0);
        assert_eq!(s.opsz, 14.0);
        assert_eq!(s.wdth, 100.0);
    }

    #[test]
    fn axis_settings_clamps_out_of_range() {
        let s = AxisSettings {
            wght: 1500.0,
            opsz: 0.0,
            wdth: 200.0,
            grad: -500.0,
            slnt: 10.0,
            rond: -10.0,
        }
        .clamped();
        assert_eq!(s.wght, 900.0);
        assert_eq!(s.opsz, 9.0);
        assert_eq!(s.wdth, 125.0);
        assert_eq!(s.grad, -200.0);
        assert_eq!(s.slnt, 0.0);
        assert_eq!(s.rond, 0.0);
    }

    #[cfg(feature = "std")]
    #[test]
    fn font_variation_settings_format() {
        let s = AxisSettings {
            wght: 500.0,
            opsz: 24.0,
            wdth: 110.0,
            grad: 50.0,
            slnt: -2.0,
            rond: 60.0,
        };
        let css = font_variation_settings(&s);
        assert!(css.contains("\"wght\" 500"), "css: {css}");
        assert!(css.contains("\"opsz\" 24"), "css: {css}");
        assert!(css.contains("\"GRAD\" 50"), "css: {css}");
        assert!(css.contains("\"ROND\" 60"), "css: {css}");
    }

    #[cfg(feature = "std")]
    #[test]
    fn export_font_face_contains_required_fields() {
        let face = export_font_face("../../../assets/fonts/google-sans-flex/");
        assert!(face.contains("font-family: 'Google Sans Flex'"));
        assert!(face.contains("font-weight: 100 900"));
        assert!(face.contains("font-stretch: 75% 125%"));
        assert!(face.contains("truetype-variations"));
        assert!(face.contains("GoogleSansFlex-VariableFont"));
    }
}
