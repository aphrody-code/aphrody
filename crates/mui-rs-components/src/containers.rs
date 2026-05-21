//! Containers: Card, Dialog, BottomSheet, SideSheet, Carousel, TopAppBar, BottomAppBar, Tooltip.

use m3_tokens::{
    shape::{CornerRadius, MEDIUM},
    state::{DRAGGED, FOCUS, HOVER, PRESSED, StateLayer},
};

use crate::actions::InteractionState;

/// M3 Card variants
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardVariant {
    Elevated,
    Filled,
    Outlined,
}

#[derive(Debug, Clone)]
pub struct Card {
    pub variant: CardVariant,
    pub interactive: bool,
    pub disabled: bool,
}

impl Card {
    /// M3 standard cards use Medium corner radius (12dp)
    pub fn shape(&self) -> CornerRadius {
        MEDIUM
    }

    /// Resolves the elevation level (0-5) based on variant and interaction state.
    pub fn elevation_level(&self, state: InteractionState) -> usize {
        if self.disabled {
            return 0; // Disabled cards have no elevation
        }

        match self.variant {
            CardVariant::Elevated => match state {
                InteractionState::Resting => 1,
                InteractionState::Hovered => 2,
                InteractionState::Focused => 1,
                InteractionState::Pressed => 1,
                InteractionState::Dragged => 4,
            },
            CardVariant::Filled => match state {
                InteractionState::Resting => 0,
                InteractionState::Hovered => 1,
                InteractionState::Focused => 0,
                InteractionState::Pressed => 0,
                InteractionState::Dragged => 3,
            },
            CardVariant::Outlined => match state {
                InteractionState::Resting => 0,
                InteractionState::Hovered => 1,
                InteractionState::Focused => 0,
                InteractionState::Pressed => 0,
                InteractionState::Dragged => 3,
            },
        }
    }

    /// Returns the state layer overlay (opacity) applied over the card background.
    pub fn state_layer(&self, state: InteractionState) -> Option<StateLayer> {
        if !self.interactive || self.disabled {
            return None;
        }
        match state {
            InteractionState::Resting => None,
            InteractionState::Hovered => Some(HOVER),
            InteractionState::Focused => Some(FOCUS),
            InteractionState::Pressed => Some(PRESSED),
            InteractionState::Dragged => Some(DRAGGED),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BottomSheet {
    pub open: bool,
}

#[derive(Debug, Clone)]
pub struct TopAppBar {
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct BottomAppBar {
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct Dialog {
    pub open: bool,
    pub title: String,
}

use mui_rs_renderer::pipeline::{DrawCx, Widget};
use mui_rs_renderer::shadow;
use mui_rs_renderer::vello::kurbo::{Affine, Rect, RoundedRect, Stroke};
use mui_rs_renderer::vello::peniko::{Color, Fill};

#[derive(Debug, Clone)]
pub struct Tooltip {
    pub text: String,
}

impl Card {
    /// Default M3 card footprint in dp (medium component).
    pub const WIDTH_DP: f64 = 200.0;
    pub const HEIGHT_DP: f64 = 150.0;
    const RADIUS_DP: f64 = 12.0; // Medium corner radius
}

impl Widget for Card {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let (w, h, r) = (Card::WIDTH_DP, Card::HEIGHT_DP, Card::RADIUS_DP);
        let rect = RoundedRect::new(0.0, 0.0, w, h, r);

        // Real M3 elevation shadow (behind the surface), per variant resting level.
        let level = self.elevation_level(InteractionState::Resting);
        shadow::draw_elevation(cx.scene, transform, Rect::new(0.0, 0.0, w, h), r, level);

        let base_color = match self.variant {
            CardVariant::Elevated => Color::from_rgb8(243, 237, 247), // Surface container low
            CardVariant::Filled => Color::from_rgb8(232, 222, 248),   // Surface container highest
            CardVariant::Outlined => Color::from_rgb8(254, 247, 255), // Surface
        };
        cx.scene.fill(Fill::NonZero, transform, base_color, None, &rect);

        // Outlined cards carry a real 1dp outline stroke (no shadow).
        if self.variant == CardVariant::Outlined {
            let outline = if self.disabled {
                Color::from_rgba8(28, 27, 31, 30)
            } else {
                Color::from_rgb8(121, 116, 126) // Outline
            };
            cx.scene.stroke(&Stroke::new(1.0), transform, outline, None, &rect);
        }
    }
}
