//! M3 typescale — 15 roles from Display Large to Label Small.

#[derive(Debug, Clone)]
pub struct TypeStyle {
    pub font_family: &'static str,
    pub weight: u16,
    pub size: f32,
    pub line_height: f32,
    pub letter_spacing: f32,
}

#[derive(Debug, Clone)]
pub struct TypeScale {
    pub display_large: TypeStyle,
    pub display_medium: TypeStyle,
    pub display_small: TypeStyle,
    pub headline_large: TypeStyle,
    pub headline_medium: TypeStyle,
    pub headline_small: TypeStyle,
    pub title_large: TypeStyle,
    pub title_medium: TypeStyle,
    pub title_small: TypeStyle,
    pub label_large: TypeStyle,
    pub label_medium: TypeStyle,
    pub label_small: TypeStyle,
    pub body_large: TypeStyle,
    pub body_medium: TypeStyle,
    pub body_small: TypeStyle,
}

impl Default for TypeScale {
    fn default() -> Self {
        let roboto = "Roboto";
        Self {
            display_large:   TypeStyle { font_family: roboto, weight: 400, size: 57.0, line_height: 64.0, letter_spacing: -0.25 },
            display_medium:  TypeStyle { font_family: roboto, weight: 400, size: 45.0, line_height: 52.0, letter_spacing: 0.0 },
            display_small:   TypeStyle { font_family: roboto, weight: 400, size: 36.0, line_height: 44.0, letter_spacing: 0.0 },
            headline_large:  TypeStyle { font_family: roboto, weight: 400, size: 32.0, line_height: 40.0, letter_spacing: 0.0 },
            headline_medium: TypeStyle { font_family: roboto, weight: 400, size: 28.0, line_height: 36.0, letter_spacing: 0.0 },
            headline_small:  TypeStyle { font_family: roboto, weight: 400, size: 24.0, line_height: 32.0, letter_spacing: 0.0 },
            title_large:     TypeStyle { font_family: roboto, weight: 400, size: 22.0, line_height: 28.0, letter_spacing: 0.0 },
            title_medium:    TypeStyle { font_family: roboto, weight: 500, size: 16.0, line_height: 24.0, letter_spacing: 0.15 },
            title_small:     TypeStyle { font_family: roboto, weight: 500, size: 14.0, line_height: 20.0, letter_spacing: 0.1 },
            label_large:     TypeStyle { font_family: roboto, weight: 500, size: 14.0, line_height: 20.0, letter_spacing: 0.1 },
            label_medium:    TypeStyle { font_family: roboto, weight: 500, size: 12.0, line_height: 16.0, letter_spacing: 0.5 },
            label_small:     TypeStyle { font_family: roboto, weight: 500, size: 11.0, line_height: 16.0, letter_spacing: 0.5 },
            body_large:      TypeStyle { font_family: roboto, weight: 400, size: 16.0, line_height: 24.0, letter_spacing: 0.5 },
            body_medium:     TypeStyle { font_family: roboto, weight: 400, size: 14.0, line_height: 20.0, letter_spacing: 0.25 },
            body_small:      TypeStyle { font_family: roboto, weight: 400, size: 12.0, line_height: 16.0, letter_spacing: 0.4 },
        }
    }
}
