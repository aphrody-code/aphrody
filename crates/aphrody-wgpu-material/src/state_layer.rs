// SPDX-License-Identifier: Apache-2.0
//! M3 state-layer model.
//!
//! Interactive surfaces (Button, FAB, IconButton, NavBar destinations,
//! Switch thumb, etc.) overlay a translucent layer of the *foreground*
//! color whose alpha encodes the current interaction state per the
//! canonical M3 specification:
//!
//! | State    | Alpha |
//! |----------|-------|
//! | Hover    | 0.08  |
//! | Focus    | 0.12  |
//! | Press    | 0.12  |
//! | Drag     | 0.16  |
//!
//! Reference: <https://m3.material.io/foundations/interaction/states/applying-states>

use crate::canvas::Color;

/// Discrete interaction state for a component.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum State {
    /// Idle — no overlay.
    #[default]
    Enabled,
    /// Hovered by a pointer.
    Hover,
    /// Keyboard focused.
    Focus,
    /// Pressed (mouse/touch down).
    Pressed,
    /// Being dragged.
    Dragged,
    /// Disabled — caller should apply a 0.38 opacity scrim separately.
    Disabled,
}

/// Canonical alpha for each interaction state.
#[must_use]
pub const fn state_alpha(state: State) -> f32 {
    match state {
        State::Enabled | State::Disabled => 0.0,
        State::Hover => 0.08,
        State::Focus | State::Pressed => 0.12,
        State::Dragged => 0.16,
    }
}

/// Compute the final composited color resulting from applying an M3
/// state-layer of `overlay` over a `base` fill.
///
/// The state layer is the same color as the on-* foreground role, mixed
/// over the base via simple straight alpha compositing.  This mirrors
/// the Compose / Web M3 reference implementations.
#[must_use]
pub fn state_layer_color(base: Color, overlay: Color, state: State) -> Color {
    let a = state_alpha(state);
    if a == 0.0 {
        return base;
    }
    Color {
        r: base.r * (1.0 - a) + overlay.r * a,
        g: base.g * (1.0 - a) + overlay.g * a,
        b: base.b * (1.0 - a) + overlay.b * a,
        a: base.a,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_layer_hover_alpha() {
        assert!((state_alpha(State::Hover) - 0.08).abs() < f32::EPSILON);
    }

    #[test]
    fn state_layer_press_alpha() {
        assert!((state_alpha(State::Pressed) - 0.12).abs() < f32::EPSILON);
    }

    #[test]
    fn state_layer_dragged_alpha() {
        assert!((state_alpha(State::Dragged) - 0.16).abs() < f32::EPSILON);
    }

    #[test]
    fn state_layer_enabled_is_passthrough() {
        let base = Color { r: 0.4, g: 0.3, b: 0.2, a: 1.0 };
        let overlay = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
        let out = state_layer_color(base, overlay, State::Enabled);
        assert_eq!(base, out);
    }

    #[test]
    fn state_layer_hover_mixes_colors() {
        let base = Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };
        let overlay = Color { r: 1.0, g: 1.0, b: 1.0, a: 1.0 };
        let out = state_layer_color(base, overlay, State::Hover);
        assert!((out.r - 0.08).abs() < 1e-6);
        assert!((out.g - 0.08).abs() < 1e-6);
        assert!((out.b - 0.08).abs() < 1e-6);
    }
}
