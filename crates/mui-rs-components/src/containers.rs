//! Containers: Card, Dialog, BottomSheet, SideSheet, Carousel.

use m3_tokens::shape::{CornerRadius, MEDIUM};
use m3_tokens::state::{StateLayer, HOVER, FOCUS, PRESSED, DRAGGED};
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
