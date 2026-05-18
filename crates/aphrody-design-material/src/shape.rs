//! M3 shape system: 7 corner radius families.

use serde::{Deserialize, Serialize};

/// M3 shape family. `radius_dp()` returns the canonical corner radius.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShapeFamily {
    None,
    ExtraSmall,
    Small,
    Medium,
    Large,
    ExtraLarge,
    /// Fully rounded (pill / circle).
    Full,
}

impl ShapeFamily {
    /// Canonical M3 corner radius in dp.
    /// Mapping: None=0, XS=4, S=8, M=12, L=16, XL=28, Full=9999.
    #[must_use]
    pub const fn radius_dp(self) -> u32 {
        match self {
            Self::None => 0,
            Self::ExtraSmall => 4,
            Self::Small => 8,
            Self::Medium => 12,
            Self::Large => 16,
            Self::ExtraLarge => 28,
            Self::Full => 9999,
        }
    }

    /// CSS `border-radius` value (px, 1dp ≈ 1px on 1×).
    #[must_use]
    pub fn css_border_radius(self) -> String {
        match self {
            Self::Full => "9999px".to_owned(),
            other => format!("{}px", other.radius_dp()),
        }
    }

    /// All M3 shape families (ordered).
    #[must_use]
    pub const fn all() -> &'static [ShapeFamily] {
        &[
            Self::None,
            Self::ExtraSmall,
            Self::Small,
            Self::Medium,
            Self::Large,
            Self::ExtraLarge,
            Self::Full,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_radius_extra_small_4dp() {
        assert_eq!(ShapeFamily::ExtraSmall.radius_dp(), 4);
        assert_eq!(ShapeFamily::Small.radius_dp(), 8);
        assert_eq!(ShapeFamily::Medium.radius_dp(), 12);
        assert_eq!(ShapeFamily::Large.radius_dp(), 16);
        assert_eq!(ShapeFamily::ExtraLarge.radius_dp(), 28);
        assert_eq!(ShapeFamily::Full.radius_dp(), 9999);
    }

    #[test]
    fn shape_css_values() {
        assert_eq!(ShapeFamily::Medium.css_border_radius(), "12px");
        assert_eq!(ShapeFamily::Full.css_border_radius(), "9999px");
    }
}
