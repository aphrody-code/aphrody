//! M3 typography type-scale (15 roles).
//!
//! Default fonts: **Google Sans Display** for Display + Headline + Title,
//! **Roboto Flex** for Body + Label. Sizes follow the M3 spec table at
//! <https://m3.material.io/styles/typography/type-scale-tokens>.

use serde::{Deserialize, Serialize};

/// Default font family for Display / Headline / Title roles.
pub const FONT_DISPLAY: &str = "Google Sans Display";
/// Default font family for Body / Label roles.
pub const FONT_BODY: &str = "Roboto Flex";

/// A typography style token.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct TypeStyle {
    pub font_family: &'static str,
    pub font_weight: u16,
    pub font_size_dp: f64,
    pub line_height_dp: f64,
    pub letter_spacing_dp: f64,
}

/// M3 type-scale variant (15 roles).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TypeRole {
    DisplayLarge,
    DisplayMedium,
    DisplaySmall,
    HeadlineLarge,
    HeadlineMedium,
    HeadlineSmall,
    TitleLarge,
    TitleMedium,
    TitleSmall,
    BodyLarge,
    BodyMedium,
    BodySmall,
    LabelLarge,
    LabelMedium,
    LabelSmall,
}

impl TypeRole {
    /// Resolve to a `TypeStyle` using the M3 canonical type-scale.
    #[must_use]
    pub const fn style(self) -> TypeStyle {
        match self {
            Self::DisplayLarge => TypeStyle {
                font_family: FONT_DISPLAY,
                font_weight: 400,
                font_size_dp: 57.0,
                line_height_dp: 64.0,
                letter_spacing_dp: -0.25,
            },
            Self::DisplayMedium => TypeStyle {
                font_family: FONT_DISPLAY,
                font_weight: 400,
                font_size_dp: 45.0,
                line_height_dp: 52.0,
                letter_spacing_dp: 0.0,
            },
            Self::DisplaySmall => TypeStyle {
                font_family: FONT_DISPLAY,
                font_weight: 400,
                font_size_dp: 36.0,
                line_height_dp: 44.0,
                letter_spacing_dp: 0.0,
            },
            Self::HeadlineLarge => TypeStyle {
                font_family: FONT_DISPLAY,
                font_weight: 400,
                font_size_dp: 32.0,
                line_height_dp: 40.0,
                letter_spacing_dp: 0.0,
            },
            Self::HeadlineMedium => TypeStyle {
                font_family: FONT_DISPLAY,
                font_weight: 400,
                font_size_dp: 28.0,
                line_height_dp: 36.0,
                letter_spacing_dp: 0.0,
            },
            Self::HeadlineSmall => TypeStyle {
                font_family: FONT_DISPLAY,
                font_weight: 400,
                font_size_dp: 24.0,
                line_height_dp: 32.0,
                letter_spacing_dp: 0.0,
            },
            Self::TitleLarge => TypeStyle {
                font_family: FONT_DISPLAY,
                font_weight: 400,
                font_size_dp: 22.0,
                line_height_dp: 28.0,
                letter_spacing_dp: 0.0,
            },
            Self::TitleMedium => TypeStyle {
                font_family: FONT_BODY,
                font_weight: 500,
                font_size_dp: 16.0,
                line_height_dp: 24.0,
                letter_spacing_dp: 0.15,
            },
            Self::TitleSmall => TypeStyle {
                font_family: FONT_BODY,
                font_weight: 500,
                font_size_dp: 14.0,
                line_height_dp: 20.0,
                letter_spacing_dp: 0.1,
            },
            Self::BodyLarge => TypeStyle {
                font_family: FONT_BODY,
                font_weight: 400,
                font_size_dp: 16.0,
                line_height_dp: 24.0,
                letter_spacing_dp: 0.5,
            },
            Self::BodyMedium => TypeStyle {
                font_family: FONT_BODY,
                font_weight: 400,
                font_size_dp: 14.0,
                line_height_dp: 20.0,
                letter_spacing_dp: 0.25,
            },
            Self::BodySmall => TypeStyle {
                font_family: FONT_BODY,
                font_weight: 400,
                font_size_dp: 12.0,
                line_height_dp: 16.0,
                letter_spacing_dp: 0.4,
            },
            Self::LabelLarge => TypeStyle {
                font_family: FONT_BODY,
                font_weight: 500,
                font_size_dp: 14.0,
                line_height_dp: 20.0,
                letter_spacing_dp: 0.1,
            },
            Self::LabelMedium => TypeStyle {
                font_family: FONT_BODY,
                font_weight: 500,
                font_size_dp: 12.0,
                line_height_dp: 16.0,
                letter_spacing_dp: 0.5,
            },
            Self::LabelSmall => TypeStyle {
                font_family: FONT_BODY,
                font_weight: 500,
                font_size_dp: 11.0,
                line_height_dp: 16.0,
                letter_spacing_dp: 0.5,
            },
        }
    }

    /// All 15 type-scale roles.
    #[must_use]
    pub const fn all() -> &'static [TypeRole] {
        &[
            Self::DisplayLarge,
            Self::DisplayMedium,
            Self::DisplaySmall,
            Self::HeadlineLarge,
            Self::HeadlineMedium,
            Self::HeadlineSmall,
            Self::TitleLarge,
            Self::TitleMedium,
            Self::TitleSmall,
            Self::BodyLarge,
            Self::BodyMedium,
            Self::BodySmall,
            Self::LabelLarge,
            Self::LabelMedium,
            Self::LabelSmall,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn type_scale_display_large_57sp() {
        let s = TypeRole::DisplayLarge.style();
        assert!((s.font_size_dp - 57.0).abs() < 1e-9);
        assert!((s.line_height_dp - 64.0).abs() < 1e-9);
        assert_eq!(s.font_family, FONT_DISPLAY);
    }

    #[test]
    fn type_scale_15_roles() {
        assert_eq!(TypeRole::all().len(), 15);
    }
}
