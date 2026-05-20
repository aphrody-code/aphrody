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

use mui_rs_renderer::pipeline::Widget;
use mui_rs_renderer::vello::{Scene, kurbo::{Affine, RoundedRect}, peniko::{Color, Fill}};

#[derive(Debug, Clone)]
pub struct Tooltip {
    pub text: String,
}

impl Widget for Card {
    fn draw(&self, scene: &mut Scene, transform: Affine) {
        let width = 200.0;
        let height = 150.0;
        
        let rect = RoundedRect::new(0.0, 0.0, width, height, 12.0); // Medium radius
        
        let base_color = match self.variant {
            CardVariant::Elevated => Color::from_rgb8(243, 237, 247), // Surface container low
            CardVariant::Filled => Color::from_rgb8(232, 222, 248), // Surface container highest
            CardVariant::Outlined => Color::from_rgb8(254, 247, 255), // Surface
        };

        scene.fill(
            Fill::NonZero,
            transform,
            base_color,
            None,
            &rect,
        );

        // If outlined, add stroke (approximated for now with inner fill)
        if self.variant == CardVariant::Outlined {
            let inner_rect = RoundedRect::new(1.0, 1.0, width - 1.0, height - 1.0, 11.0);
            scene.fill(
                Fill::NonZero,
                transform,
                Color::from_rgb8(254, 247, 255), // Outline inner
                None,
                &inner_rect,
            );
        }
    }
}
