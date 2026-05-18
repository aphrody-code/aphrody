//! M3 elevation: 6 levels (0..=5) with box-shadow + surface tint overlay.
//!
//! Surface tint alphas follow the canonical M3 table:
//! - L0: 0.00 — L1: 0.05 — L2: 0.08 — L3: 0.11 — L4: 0.12 — L5: 0.14.

use serde::{Deserialize, Serialize};

/// M3 elevation level (0..=5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Elevation {
    Level0,
    Level1,
    Level2,
    Level3,
    Level4,
    Level5,
}

impl Elevation {
    /// dp height of this elevation.
    #[must_use]
    pub const fn dp(self) -> u32 {
        match self {
            Self::Level0 => 0,
            Self::Level1 => 1,
            Self::Level2 => 3,
            Self::Level3 => 6,
            Self::Level4 => 8,
            Self::Level5 => 12,
        }
    }

    /// Surface tint overlay alpha applied on top of the surface color
    /// (`surfaceTint` role mixed at this alpha).
    #[must_use]
    pub const fn surface_tint_alpha(self) -> f64 {
        match self {
            Self::Level0 => 0.00,
            Self::Level1 => 0.05,
            Self::Level2 => 0.08,
            Self::Level3 => 0.11,
            Self::Level4 => 0.12,
            Self::Level5 => 0.14,
        }
    }

    /// CSS `box-shadow` for this elevation (two-layer M3 shadow:
    /// key + ambient). Colors are black with M3-canonical alphas.
    #[must_use]
    pub fn box_shadow_css(self) -> String {
        let dp = self.dp();
        if dp == 0 {
            return "none".to_owned();
        }
        // Two-layer M3 shadow: key (sharper, vertical offset) + ambient
        // (softer, larger blur). Numbers cribbed from Material spec table.
        let key_y = dp.max(1);
        let key_blur = (dp * 2).max(2);
        let amb_y = (dp / 2).max(1);
        let amb_blur = (dp * 3).max(3);
        format!(
            "0px {key_y}px {key_blur}px 0px rgba(0,0,0,0.30), \
             0px {amb_y}px {amb_blur}px 0px rgba(0,0,0,0.15)"
        )
    }

    /// All levels in order.
    #[must_use]
    pub const fn all() -> &'static [Elevation] {
        &[
            Self::Level0,
            Self::Level1,
            Self::Level2,
            Self::Level3,
            Self::Level4,
            Self::Level5,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elevation_level3_shadow_format() {
        let s = Elevation::Level3.box_shadow_css();
        assert!(s.contains("rgba(0,0,0,0.30)"), "missing key layer: {s}");
        assert!(s.contains("rgba(0,0,0,0.15)"), "missing ambient layer: {s}");
        assert!(s.contains("px"));
    }

    #[test]
    fn elevation_level0_is_none() {
        assert_eq!(Elevation::Level0.box_shadow_css(), "none");
        assert!((Elevation::Level0.surface_tint_alpha() - 0.0).abs() < 1e-9);
    }

    #[test]
    fn elevation_tint_monotonic() {
        let alphas: Vec<f64> = Elevation::all()
            .iter()
            .map(|e| e.surface_tint_alpha())
            .collect();
        for w in alphas.windows(2) {
            assert!(w[0] <= w[1], "tint alpha should be non-decreasing");
        }
        assert!((alphas[5] - 0.14).abs() < 1e-9);
    }
}
