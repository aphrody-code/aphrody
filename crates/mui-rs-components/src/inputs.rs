//! Inputs: TextField, Select, Checkbox, Radio, Switch, Slider, DatePicker, TimePicker, SearchBar.

use mui_rs_renderer::pipeline::{DrawCx, Widget};
use mui_rs_renderer::vello::kurbo::{Affine, BezPath, Circle, Line, RoundedRect, Stroke};
use mui_rs_renderer::vello::peniko::{Color, Fill};
use mui_rs_renderer::TextStyle;

const FIELD_FAMILY: &str = mui_rs_renderer::text::FONT_UI;

#[derive(Debug, Clone)]
pub struct TextField {
    pub label: String,
    pub value: String,
    pub variant: TextFieldVariant,
    pub disabled: bool,
    pub error: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextFieldVariant {
    Standard,
    Filled,
    Outlined,
}

impl TextField {
    pub const WIDTH_DP: f64 = 240.0;
    pub const HEIGHT_DP: f64 = 56.0;
}

impl Widget for TextField {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let width = TextField::WIDTH_DP;
        let height = TextField::HEIGHT_DP;

        match self.variant {
            TextFieldVariant::Outlined => {
                let rect = RoundedRect::new(0.0, 0.0, width, height, 4.0);
                let border_color = if self.error {
                    Color::from_rgb8(179, 38, 30) // Error
                } else if self.disabled {
                    Color::from_rgba8(28, 27, 31, 30)
                } else {
                    Color::from_rgb8(121, 116, 126) // Outline
                };
                cx.scene.stroke(&Stroke::new(1.0), transform, border_color, None, &rect);
            }
            TextFieldVariant::Filled => {
                let rect = RoundedRect::new(0.0, 0.0, width, height, 4.0);
                cx.scene.fill(Fill::NonZero, transform, Color::from_rgb8(231, 224, 236), None, &rect);
                let line_y = height - 1.0;
                let line = Line::new((0.0, line_y), (width, line_y));
                cx.scene.stroke(&Stroke::new(1.0), transform, Color::from_rgb8(73, 69, 79), None, &line);
            }
            TextFieldVariant::Standard => {
                let line_y = height - 1.0;
                let line = Line::new((0.0, line_y), (width, line_y));
                cx.scene.stroke(&Stroke::new(1.0), transform, Color::from_rgb8(73, 69, 79), None, &line);
            }
        }

        // Label (small, top) and value (body, baseline) — real glyphs.
        let label_color = if self.error {
            Color::from_rgb8(179, 38, 30)
        } else if self.disabled {
            Color::from_rgba8(28, 27, 31, 97)
        } else {
            Color::from_rgb8(73, 69, 79) // on-surface-variant
        };
        if !self.label.is_empty() {
            let label_style = TextStyle::new(FIELD_FAMILY, 12.0, 400.0, label_color);
            cx.draw_text(&self.label, label_style, transform * Affine::translate((16.0, 8.0)));
        }
        if !self.value.is_empty() {
            let value_color = if self.disabled {
                Color::from_rgba8(28, 27, 31, 97)
            } else {
                Color::from_rgb8(28, 27, 31) // on-surface
            };
            let value_style = TextStyle::new(FIELD_FAMILY, 16.0, 400.0, value_color);
            cx.draw_text(&self.value, value_style, transform * Affine::translate((16.0, 26.0)));
        }
    }
}

#[derive(Debug, Clone)]
pub struct Select {
    pub options: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct Checkbox {
    pub checked: bool,
    pub disabled: bool,
}

impl Widget for Checkbox {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let size = 18.0;
        let rect = RoundedRect::new(0.0, 0.0, size, size, 2.0);
        if self.checked {
            let color = if self.disabled { Color::from_rgba8(28, 27, 31, 97) } else { Color::from_rgb8(103, 80, 164) };
            cx.scene.fill(Fill::NonZero, transform, color, None, &rect);
            // Checkmark
            let mut path = BezPath::new();
            path.move_to((4.0, 9.0));
            path.line_to((8.0, 13.0));
            path.line_to((14.0, 5.0));
            cx.scene.stroke(&Stroke::new(2.0), transform, Color::WHITE, None, &path);
        } else {
            let color = if self.disabled { Color::from_rgba8(28, 27, 31, 97) } else { Color::from_rgb8(73, 69, 79) };
            cx.scene.stroke(&Stroke::new(2.0), transform, color, None, &rect);
        }
    }
}

#[derive(Debug, Clone)]
pub struct Radio {
    pub checked: bool,
    pub disabled: bool,
}

impl Widget for Radio {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let center = (10.0, 10.0);
        let outer_circle = Circle::new(center, 10.0);
        let color = if self.disabled { Color::from_rgba8(28, 27, 31, 97) } else if self.checked { Color::from_rgb8(103, 80, 164) } else { Color::from_rgb8(73, 69, 79) };
        
        cx.scene.stroke(&Stroke::new(2.0), transform, color, None, &outer_circle);

        if self.checked {
            let inner_circle = Circle::new(center, 5.0);
            cx.scene.fill(Fill::NonZero, transform, color, None, &inner_circle);
        }
    }
}

#[derive(Debug, Clone)]
pub struct Switch {
    pub checked: bool,
    pub disabled: bool,
}

impl Widget for Switch {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        // Material 3 Switch Dimensions
        let track_width = 52.0;
        let track_height = 32.0;
        let track_radius = 16.0;

        // Draw track
        let track_rect = RoundedRect::new(0.0, 0.0, track_width, track_height, track_radius);
        
        let track_color = if self.disabled {
            Color::from_rgba8(28, 27, 31, 30) // disabled surface
        } else if self.checked {
            Color::from_rgb8(103, 80, 164) // Primary
        } else {
            Color::from_rgb8(231, 224, 236) // Surface container highest
        };

        cx.scene.fill(Fill::NonZero, transform, track_color, None, &track_rect);

        // Thumb specs
        let thumb_color = if self.disabled {
            Color::from_rgba8(28, 27, 31, 97)
        } else if self.checked {
            Color::from_rgb8(255, 255, 255) // On primary
        } else {
            Color::from_rgb8(121, 116, 126) // Outline
        };

        // Thumb size and position
        let (thumb_radius, thumb_cx) = if self.checked {
            (12.0, track_width - 16.0) // 24dp size, right aligned
        } else {
            (8.0, 16.0) // 16dp size, left aligned
        };
        let thumb_cy = track_height / 2.0;

        let thumb_circle = Circle::new((thumb_cx, thumb_cy), thumb_radius);
        cx.scene.fill(Fill::NonZero, transform, thumb_color, None, &thumb_circle);
    }
}

#[derive(Debug, Clone)]
pub struct Slider {
    pub value: f32,
    pub disabled: bool,
}

impl Widget for Slider {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let width = 200.0;
        let cy = 24.0;
        
        let track_color = if self.disabled { Color::from_rgba8(28, 27, 31, 30) } else { Color::from_rgb8(231, 224, 236) };
        let active_color = if self.disabled { Color::from_rgba8(28, 27, 31, 97) } else { Color::from_rgb8(103, 80, 164) };
        
        // Inactive track
        let inactive_track = Line::new((0.0, cy), (width, cy));
        cx.scene.stroke(&Stroke::new(4.0), transform, track_color, None, &inactive_track);

        // Active track
        let active_width = width * f64::from(self.value.clamp(0.0, 1.0));
        let active_track = Line::new((0.0, cy), (active_width, cy));
        cx.scene.stroke(&Stroke::new(4.0), transform, active_color, None, &active_track);

        // Thumb
        let thumb = Circle::new((active_width, cy), 10.0);
        cx.scene.fill(Fill::NonZero, transform, active_color, None, &thumb);
    }
}

#[derive(Debug, Clone)]
pub struct DatePicker {
    pub date: String,
}

#[derive(Debug, Clone)]
pub struct TimePicker {
    pub time: String,
}

#[derive(Debug, Clone)]
pub struct SearchBar {
    pub query: String,
}

/// Shared outlined-field footprint (Select / DatePicker / TimePicker).
const FIELD_W: f64 = 240.0;
const FIELD_H: f64 = 56.0;

/// Draws an outlined field box + a value/label and returns nothing.
fn outlined_field(cx: &mut DrawCx, transform: Affine, text: &str, placeholder_only: bool) {
    let rect = RoundedRect::new(0.0, 0.0, FIELD_W, FIELD_H, 4.0);
    cx.scene.stroke(&Stroke::new(1.0), transform, Color::from_rgb8(121, 116, 126), None, &rect);
    let color = if placeholder_only {
        Color::from_rgb8(73, 69, 79) // on-surface-variant
    } else {
        Color::from_rgb8(28, 27, 31) // on-surface
    };
    let style = TextStyle::new(FIELD_FAMILY, 16.0, 400.0, color);
    let (_tw, th) = cx.measure_text(text, style);
    cx.draw_text(text, style, transform * Affine::translate((16.0, (FIELD_H - f64::from(th)) / 2.0)));
}

impl Widget for Select {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let label = self.options.first().map_or("Select", |s| s.as_str());
        outlined_field(cx, transform, label, self.options.is_empty());
        // Dropdown chevron at the trailing edge.
        let mut chevron = BezPath::new();
        let x = FIELD_W - 28.0;
        let y = FIELD_H / 2.0 - 3.0;
        chevron.move_to((x, y));
        chevron.line_to((x + 6.0, y + 6.0));
        chevron.line_to((x + 12.0, y));
        cx.scene.stroke(&Stroke::new(2.0), transform, Color::from_rgb8(73, 69, 79), None, &chevron);
    }
}

impl Widget for DatePicker {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let txt = if self.date.is_empty() { "Date" } else { &self.date };
        outlined_field(cx, transform, txt, self.date.is_empty());
        // Calendar glyph (square + top ticks) at the trailing edge.
        let bx = FIELD_W - 32.0;
        let by = FIELD_H / 2.0 - 8.0;
        let cal = RoundedRect::new(bx, by, bx + 18.0, by + 18.0, 2.0);
        cx.scene.stroke(&Stroke::new(1.5), transform, Color::from_rgb8(73, 69, 79), None, &cal);
        let bar = Line::new((bx, by + 5.0), (bx + 18.0, by + 5.0));
        cx.scene.stroke(&Stroke::new(1.5), transform, Color::from_rgb8(73, 69, 79), None, &bar);
    }
}

impl Widget for TimePicker {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let txt = if self.time.is_empty() { "Time" } else { &self.time };
        outlined_field(cx, transform, txt, self.time.is_empty());
        // Clock glyph (circle + hands) at the trailing edge.
        let cxp = FIELD_W - 23.0;
        let cyp = FIELD_H / 2.0;
        cx.scene.stroke(&Stroke::new(1.5), transform, Color::from_rgb8(73, 69, 79), None, &Circle::new((cxp, cyp), 9.0));
        let hand = Line::new((cxp, cyp), (cxp, cyp - 5.0));
        cx.scene.stroke(&Stroke::new(1.5), transform, Color::from_rgb8(73, 69, 79), None, &hand);
    }
}

impl SearchBar {
    pub const WIDTH_DP: f64 = 360.0;
    pub const HEIGHT_DP: f64 = 56.0;
}

impl Widget for SearchBar {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let (w, h) = (SearchBar::WIDTH_DP, SearchBar::HEIGHT_DP);
        let pill = RoundedRect::new(0.0, 0.0, w, h, h / 2.0);
        cx.scene.fill(Fill::NonZero, transform, Color::from_rgb8(236, 230, 240), None, &pill); // surface-container-high
        // Leading search glyph (circle + diagonal handle).
        let (gx, gy) = (28.0, h / 2.0);
        cx.scene.stroke(&Stroke::new(2.0), transform, Color::from_rgb8(73, 69, 79), None, &Circle::new((gx, gy - 1.0), 6.0));
        let handle = Line::new((gx + 4.0, gy + 3.0), (gx + 9.0, gy + 8.0));
        cx.scene.stroke(&Stroke::new(2.0), transform, Color::from_rgb8(73, 69, 79), None, &handle);
        // Query text (or placeholder).
        let placeholder = self.query.is_empty();
        let text = if placeholder { "Search" } else { &self.query };
        let color = if placeholder { Color::from_rgb8(73, 69, 79) } else { Color::from_rgb8(28, 27, 31) };
        let style = TextStyle::new(FIELD_FAMILY, 16.0, 400.0, color);
        let (_tw, th) = cx.measure_text(text, style);
        cx.draw_text(text, style, transform * Affine::translate((52.0, (h - f64::from(th)) / 2.0)));
    }
}
