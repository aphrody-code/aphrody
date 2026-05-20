//! M3 theme — groups ColorScheme + TypeScale + ShapeScale + ElevationLevel.

use m3_tokens::{color::ColorRoles, shape::CornerRadius, typography::TypeStyle};

pub struct Theme {
    pub color: ColorRoles,
    pub typography: [TypeStyle; 15],
    pub shape: [CornerRadius; 7],
    pub dark_mode: bool,
}
