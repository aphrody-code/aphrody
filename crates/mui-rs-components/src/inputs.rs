//! Inputs: TextField, Select, Checkbox, Radio, Switch, Slider, DatePicker, TimePicker, SearchBar.

use mui_rs_renderer::{
    pipeline::Widget,
    vello::{
        Scene,
        kurbo::{Affine, Circle, RoundedRect},
        peniko::{Color, Fill},
    },
};

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

impl Widget for TextField {
    fn draw(&self, scene: &mut Scene, transform: Affine) {
        let width = 240.0;
        let height = 56.0;

        match self.variant {
            TextFieldVariant::Outlined => {
                let rect = RoundedRect::new(0.0, 0.0, width, height, 4.0);
                let border_color = if self.error {
                    Color::from_rgb8(179, 38, 30) // Error color
                } else if self.disabled {
                    Color::from_rgba8(28, 27, 31, 30)
                } else {
                    Color::from_rgb8(121, 116, 126) // Outline
                };
                
                // Draw border (stroke)
                scene.stroke(
                    &mui_rs_renderer::vello::kurbo::Stroke::new(1.0),
                    transform,
                    border_color,
                    None,
                    &rect,
                );
            }
            TextFieldVariant::Filled => {
                let rect = RoundedRect::new(0.0, 0.0, width, height, 4.0);
                let bg_color = Color::from_rgb8(231, 224, 236);
                scene.fill(Fill::NonZero, transform, bg_color, None, &rect);
                
                // Bottom line
                let line_y = height - 1.0;
                let line = mui_rs_renderer::vello::kurbo::Line::new((0.0, line_y), (width, line_y));
                scene.stroke(
                    &mui_rs_renderer::vello::kurbo::Stroke::new(1.0),
                    transform,
                    Color::from_rgb8(73, 69, 79),
                    None,
                    &line,
                );
            }
            TextFieldVariant::Standard => {
                let line_y = height - 1.0;
                let line = mui_rs_renderer::vello::kurbo::Line::new((0.0, line_y), (width, line_y));
                scene.stroke(
                    &mui_rs_renderer::vello::kurbo::Stroke::new(1.0),
                    transform,
                    Color::from_rgb8(73, 69, 79),
                    None,
                    &line,
                );
            }
        }
        
        // TODO: Render Label and Value using parley (text engine)
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
    fn draw(&self, scene: &mut Scene, transform: Affine) {
        let size = 18.0;
        let rect = RoundedRect::new(0.0, 0.0, size, size, 2.0);
        if self.checked {
            let color = if self.disabled { Color::from_rgba8(28, 27, 31, 97) } else { Color::from_rgb8(103, 80, 164) };
            scene.fill(Fill::NonZero, transform, color, None, &rect);
            // Checkmark
            let mut path = mui_rs_renderer::vello::kurbo::BezPath::new();
            path.move_to((4.0, 9.0));
            path.line_to((8.0, 13.0));
            path.line_to((14.0, 5.0));
            let check_color = if self.disabled { Color::from_rgba8(255, 255, 255, 255) } else { Color::from_rgb8(255, 255, 255) };
            scene.stroke(&mui_rs_renderer::vello::kurbo::Stroke::new(2.0), transform, check_color, None, &path);
        } else {
            let color = if self.disabled { Color::from_rgba8(28, 27, 31, 97) } else { Color::from_rgb8(73, 69, 79) };
            scene.stroke(&mui_rs_renderer::vello::kurbo::Stroke::new(2.0), transform, color, None, &rect);
        }
    }
}

#[derive(Debug, Clone)]
pub struct Radio {
    pub checked: bool,
    pub disabled: bool,
}

impl Widget for Radio {
    fn draw(&self, scene: &mut Scene, transform: Affine) {
        let center = (10.0, 10.0);
        let outer_circle = Circle::new(center, 10.0);
        let color = if self.disabled { Color::from_rgba8(28, 27, 31, 97) } else if self.checked { Color::from_rgb8(103, 80, 164) } else { Color::from_rgb8(73, 69, 79) };
        
        scene.stroke(&mui_rs_renderer::vello::kurbo::Stroke::new(2.0), transform, color, None, &outer_circle);
        
        if self.checked {
            let inner_circle = Circle::new(center, 5.0);
            scene.fill(Fill::NonZero, transform, color, None, &inner_circle);
        }
    }
}

#[derive(Debug, Clone)]
pub struct Switch {
    pub checked: bool,
    pub disabled: bool,
}

impl Widget for Switch {
    fn draw(&self, scene: &mut Scene, transform: Affine) {
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

        scene.fill(Fill::NonZero, transform, track_color, None, &track_rect);

        // Thumb specs
        let thumb_color = if self.disabled {
            if self.checked {
                Color::from_rgba8(28, 27, 31, 97)
            } else {
                Color::from_rgba8(28, 27, 31, 97)
            }
        } else if self.checked {
            Color::from_rgb8(255, 255, 255) // On primary
        } else {
            Color::from_rgb8(121, 116, 126) // Outline
        };

        // Thumb size and position
        let (thumb_radius, cx) = if self.checked {
            (12.0, track_width - 16.0) // 24dp size, right aligned
        } else {
            (8.0, 16.0) // 16dp size, left aligned
        };
        let cy = track_height / 2.0;

        let thumb_circle = Circle::new((cx, cy), thumb_radius);
        scene.fill(Fill::NonZero, transform, thumb_color, None, &thumb_circle);
    }
}

#[derive(Debug, Clone)]
pub struct Slider {
    pub value: f32,
    pub disabled: bool,
}

impl Widget for Slider {
    fn draw(&self, scene: &mut Scene, transform: Affine) {
        let width = 200.0;
        let cy = 24.0;
        
        let track_color = if self.disabled { Color::from_rgba8(28, 27, 31, 30) } else { Color::from_rgb8(231, 224, 236) };
        let active_color = if self.disabled { Color::from_rgba8(28, 27, 31, 97) } else { Color::from_rgb8(103, 80, 164) };
        
        // Inactive track
        let inactive_track = mui_rs_renderer::vello::kurbo::Line::new((0.0, cy), (width, cy));
        scene.stroke(&mui_rs_renderer::vello::kurbo::Stroke::new(4.0), transform, track_color, None, &inactive_track);
        
        // Active track
        let active_width = width * (self.value.clamp(0.0, 1.0) as f64);
        let active_track = mui_rs_renderer::vello::kurbo::Line::new((0.0, cy), (active_width, cy));
        scene.stroke(&mui_rs_renderer::vello::kurbo::Stroke::new(4.0), transform, active_color, None, &active_track);
        
        // Thumb
        let thumb = Circle::new((active_width, cy), 10.0);
        scene.fill(Fill::NonZero, transform, active_color, None, &thumb);
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
