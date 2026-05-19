//! M3 theme — groups ColorScheme + TypeScale + ShapeScale + ElevationLevel.

use mui_rs_tokens::{ColorScheme, TypeScale, ShapeScale};

pub struct Theme {
    pub color: ColorScheme,
    pub typography: TypeScale,
    pub shape: ShapeScale,
    pub dark_mode: bool,
}
