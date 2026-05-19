//! Actions: Button, IconButton, FAB, ButtonGroup.

use m3_tokens::shape::{CornerRadius, FULL};
use m3_tokens::elevation::{dp as elevation_dp, LEVEL_COUNT};
use m3_tokens::state::{StateLayer, HOVER, FOCUS, PRESSED};

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

    /// Returns the state layer overlay (opacity) applied over the button background.
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
