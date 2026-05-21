//! Actions: Button, IconButton, FAB, ButtonGroup.

use m3_tokens::{
    shape::{CornerRadius, FULL},
    state::{FOCUS, HOVER, PRESSED, StateLayer},
};

/// M3 Button variants
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ButtonVariant {
    Filled,
    FilledTonal,
    Outlined,
    Elevated,
    Text,
}

/// User interaction state for styling
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum InteractionState {
    Resting,
    Hovered,
    Focused,
    Pressed,
    Dragged,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Button {
    pub variant: ButtonVariant,
    pub label: String,
    pub disabled: bool,
    pub icon: Option<String>,
    pub on_click_id: Option<String>,
}

impl Button {
    /// M3 Standard height for buttons
    pub const HEIGHT_DP: u16 = 40;

    /// All standard buttons use FULL corner radius (pill shape)
    pub fn shape(&self) -> CornerRadius {
        FULL
    }

    /// Horizontal padding (left, right) in dp.
    /// Changes if there's an icon.
    pub fn horizontal_padding(&self) -> (u16, u16) {
        if self.icon.is_some() {
            (16, 24) // Leading icon spacing
        } else {
            (24, 24)
        }
    }

    /// Internal spacing between icon and label in dp.
    pub fn icon_spacing(&self) -> u16 {
        8
    }

    /// Resolves the elevation level (0-5) based on variant and interaction state.
    pub fn elevation_level(&self, state: InteractionState) -> usize {
        if self.disabled {
            return 0; // Disabled buttons have no elevation
        }

        match self.variant {
            ButtonVariant::Elevated => match state {
                InteractionState::Resting => 1,
                InteractionState::Hovered => 2,
                InteractionState::Focused => 1,
                InteractionState::Pressed => 1,
                InteractionState::Dragged => 3,
            },
            ButtonVariant::Filled => match state {
                InteractionState::Resting => 0,
                InteractionState::Hovered => 1,
                InteractionState::Focused => 0,
                InteractionState::Pressed => 0,
                InteractionState::Dragged => 3,
            },
            // Tonal, Outlined, and Text buttons remain flat (level 0) across states
            _ => 0,
        }
    }

    pub fn state_layer(&self, state: InteractionState) -> Option<StateLayer> {
        if self.disabled {
            return None; // Disabled content uses specific container/content opacity, not overlay
        }
        match state {
            InteractionState::Resting => None,
            InteractionState::Hovered => Some(HOVER),
            InteractionState::Focused => Some(FOCUS),
            InteractionState::Pressed => Some(PRESSED),
            InteractionState::Dragged => Some(m3_tokens::state::DRAGGED),
        }
    }
}

use mui_rs_renderer::pipeline::{DrawCx, Widget};
use mui_rs_renderer::shadow;
use mui_rs_renderer::vello::kurbo::{Affine, Rect, RoundedRect, Stroke};
use mui_rs_renderer::vello::peniko::{Color, Fill};
use mui_rs_renderer::TextStyle;

/// M3 label-large type face for button text.
const LABEL_FAMILY: &str = "Roboto, Segoe UI, Arial, sans-serif";
const LABEL_SIZE: f32 = 14.0;
const LABEL_WEIGHT: f32 = 500.0;

impl Button {
    /// Content (label) colour for the variant + disabled state.
    fn label_color(&self) -> Color {
        if self.disabled {
            return Color::from_rgba8(28, 27, 31, 97); // on-surface 38%
        }
        match self.variant {
            ButtonVariant::Filled => Color::WHITE,                       // on-primary
            ButtonVariant::FilledTonal => Color::from_rgb8(29, 25, 43),  // on-secondary-container
            ButtonVariant::Outlined | ButtonVariant::Elevated | ButtonVariant::Text => {
                Color::from_rgb8(103, 80, 164) // primary
            }
        }
    }
}

impl Widget for Button {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let height = f64::from(Button::HEIGHT_DP);
        let style = TextStyle::new(LABEL_FAMILY, LABEL_SIZE, LABEL_WEIGHT, self.label_color());

        // Real text metrics drive the pill width (replaces the old guess).
        let (label_w, label_h) = cx.measure_text(&self.label, style);
        let (px_left, px_right) = (24.0_f64, 24.0_f64);
        let width = (px_left + f64::from(label_w) + px_right).max(height);
        let rect = RoundedRect::new(0.0, 0.0, width, height, height / 2.0);

        // Elevated/Filled buttons cast a real M3 shadow at rest.
        let level = self.elevation_level(InteractionState::Resting);
        shadow::draw_elevation(cx.scene, transform, Rect::new(0.0, 0.0, width, height), height / 2.0, level);

        let base_color = if self.disabled {
            Color::from_rgba8(28, 27, 31, 30)
        } else {
            match self.variant {
                ButtonVariant::Filled => Color::from_rgb8(103, 80, 164), // Primary
                ButtonVariant::FilledTonal => Color::from_rgb8(232, 222, 248), // Secondary container
                ButtonVariant::Elevated => Color::from_rgb8(243, 237, 247), // Surface container low
                ButtonVariant::Outlined | ButtonVariant::Text => Color::TRANSPARENT,
            }
        };
        cx.scene.fill(Fill::NonZero, transform, base_color, None, &rect);

        if self.variant == ButtonVariant::Outlined {
            let stroke_color = if self.disabled {
                Color::from_rgba8(28, 27, 31, 30)
            } else {
                Color::from_rgb8(121, 116, 126)
            };
            cx.scene.stroke(&Stroke::new(1.0), transform, stroke_color, None, &rect);
        }

        // Centred label — real glyphs.
        let tx = (width - f64::from(label_w)) / 2.0;
        let ty = (height - f64::from(label_h)) / 2.0;
        cx.draw_text(&self.label, style, transform * Affine::translate((tx, ty)));
    }
}

#[derive(Debug, Clone)]
pub struct Fab {
    pub icon: String,
}

impl Fab {
    pub const SIZE_DP: f64 = 56.0;
    const RADIUS_DP: f64 = 16.0;
}

impl Widget for Fab {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let (size, r) = (Fab::SIZE_DP, Fab::RADIUS_DP);
        let rect = RoundedRect::new(0.0, 0.0, size, size, r);

        // FABs sit at elevation level 3 in M3.
        shadow::draw_elevation(cx.scene, transform, Rect::new(0.0, 0.0, size, size), r, 3);

        let base_color = Color::from_rgb8(232, 222, 248); // Primary container
        cx.scene.fill(Fill::NonZero, transform, base_color, None, &rect);

        // Render the icon glyph centred when it is a single pictographic char
        // (emoji / symbol). Material Symbol *names* (e.g. "add") need the symbol
        // font, which is loaded separately — those are skipped rather than drawn
        // as literal words.
        if self.icon.chars().count() == 1 && self.icon.chars().all(|c| !c.is_ascii_alphabetic()) {
            let style = TextStyle::new("Segoe UI Emoji, sans-serif", 24.0, 400.0, Color::from_rgb8(29, 25, 43));
            let (gw, gh) = cx.measure_text(&self.icon, style);
            let tx = (size - f64::from(gw)) / 2.0;
            let ty = (size - f64::from(gh)) / 2.0;
            cx.draw_text(&self.icon, style, transform * Affine::translate((tx, ty)));
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExtendedFab {
    pub label: String,
    pub icon: String,
}

#[derive(Debug, Clone)]
pub struct SegmentedButton {
    pub options: Vec<String>,
}
