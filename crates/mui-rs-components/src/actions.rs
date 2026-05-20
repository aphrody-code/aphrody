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

use mui_rs_renderer::{
    pipeline::Widget,
    vello::{
        Scene,
        kurbo::{Affine, RoundedRect},
        peniko::{Color, Fill},
    },
};

impl Widget for Button {
    fn draw(&self, scene: &mut Scene, transform: Affine) {
        let (px_left, px_right) = self.horizontal_padding();
        let width = px_left as f64 + 60.0 + px_right as f64; // approximate width based on label length
        let height = Button::HEIGHT_DP as f64;

        let rect = RoundedRect::new(0.0, 0.0, width, height, height / 2.0); // Pill shape (FULL radius)

        // Define base color based on variant
        let base_color = if self.disabled {
            Color::from_rgba8(28, 27, 31, 30) // Surface on-variant with 0.12 opacity
        } else {
            match self.variant {
                ButtonVariant::Filled => Color::from_rgb8(103, 80, 164), // Primary
                ButtonVariant::FilledTonal => Color::from_rgb8(232, 222, 248), /* Secondary container */
                ButtonVariant::Elevated => Color::from_rgb8(243, 237, 247), /* Surface container
                                                                              * low */
                ButtonVariant::Outlined | ButtonVariant::Text => Color::TRANSPARENT,
            }
        };

        // Draw background
        scene.fill(Fill::NonZero, transform, base_color, None, &rect);

        // Outlined stroke
        if self.variant == ButtonVariant::Outlined {
            let stroke_color = if self.disabled {
                Color::from_rgba8(28, 27, 31, 30) // disabled outline
            } else {
                Color::from_rgb8(121, 116, 126) // Outline color
            };
            scene.stroke(
                &mui_rs_renderer::vello::kurbo::Stroke::new(1.0),
                transform,
                stroke_color,
                None,
                &rect,
            );
        }

        // TODO: State Layer overlay, Shadow/Elevation, Label Text (requires parley), Icon (requires SVG/font).
    }
}

#[derive(Debug, Clone)]
pub struct Fab {
    pub icon: String,
}

impl Widget for Fab {
    fn draw(&self, scene: &mut Scene, transform: Affine) {
        let size = 56.0;
        let rect = RoundedRect::new(0.0, 0.0, size, size, 16.0); // FAB has 16dp rounding in M3
        let base_color = Color::from_rgb8(232, 222, 248); // Primary Container
        
        scene.fill(Fill::NonZero, transform, base_color, None, &rect);
        // TODO: Shadow/Icon to be implemented
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
