//! M3 shape scale — corner radii for all component sizes.

#[derive(Debug, Clone, Copy)]
pub enum CornerStyle {
    Rounded(f32),
    Cut(f32),
    Full,
    None,
}

#[derive(Debug, Clone)]
pub struct ShapeScale {
    pub extra_small: CornerStyle,
    pub small: CornerStyle,
    pub medium: CornerStyle,
    pub large: CornerStyle,
    pub extra_large: CornerStyle,
    pub full: CornerStyle,
}

impl Default for ShapeScale {
    fn default() -> Self {
        Self {
            extra_small: CornerStyle::Rounded(4.0),
            small:       CornerStyle::Rounded(8.0),
            medium:      CornerStyle::Rounded(12.0),
            large:       CornerStyle::Rounded(16.0),
            extra_large: CornerStyle::Rounded(28.0),
            full:        CornerStyle::Full,
        }
    }
}
