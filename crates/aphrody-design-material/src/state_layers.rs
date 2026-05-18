//! M3 state layers — alpha overlays applied on top of a base color to
//! represent interaction state.
//!
//! M3 spec (NOT M2):
//! - Hover  : 8 %  (M2 was 8 % too)
//! - Focus  : 10 % (M2 was 12 %)
//! - Pressed: 10 % (M2 was 12 %)
//! - Dragged: 16 % (same as M2)

use serde::{Deserialize, Serialize};

/// Hover state-layer alpha (0.08).
pub const HOVER: f64 = 0.08;
/// Focus state-layer alpha (0.10).
pub const FOCUS: f64 = 0.10;
/// Pressed state-layer alpha (0.10).
pub const PRESSED: f64 = 0.10;
/// Dragged state-layer alpha (0.16).
pub const DRAGGED: f64 = 0.16;

/// Discrete interaction state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum State {
    Hover,
    Focus,
    Pressed,
    Dragged,
}

impl State {
    /// Alpha of this state layer.
    #[must_use]
    pub const fn alpha(self) -> f64 {
        match self {
            Self::Hover => HOVER,
            Self::Focus => FOCUS,
            Self::Pressed => PRESSED,
            Self::Dragged => DRAGGED,
        }
    }
}

/// Composite `layer` (with `alpha`) over `base` in straight-alpha RGB. The
/// returned ARGB has alpha 0xFF (state layers are opaque visually).
#[must_use]
pub fn composite_over(base: u32, layer: u32, alpha: f64) -> u32 {
    let a = alpha.clamp(0.0, 1.0);
    let mix = |bc: u32, lc: u32| -> u32 {
        let b = f64::from(bc as u8);
        let l = f64::from(lc as u8);
        let out = b * (1.0 - a) + l * a;
        (out.round() as u32).min(255)
    };
    let r = mix(base >> 16, layer >> 16);
    let g = mix(base >> 8, layer >> 8);
    let b = mix(base, layer);
    0xFF00_0000 | (r << 16) | (g << 8) | b
}

/// Apply a state layer of `state_color` (typically the on-XXX role) at the
/// canonical M3 alpha onto `base` (the XXX role).
#[must_use]
pub fn apply_state(base: u32, state_color: u32, state: State) -> u32 {
    composite_over(base, state_color, state.alpha())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_layer_hover_0_08_alpha() {
        assert!((State::Hover.alpha() - 0.08).abs() < 1e-9);
        assert!((State::Focus.alpha() - 0.10).abs() < 1e-9);
        assert!((State::Pressed.alpha() - 0.10).abs() < 1e-9);
        assert!((State::Dragged.alpha() - 0.16).abs() < 1e-9);
    }

    #[test]
    fn composite_white_over_black_at_50pct_is_grey() {
        let out = composite_over(0xFF00_0000, 0xFFFF_FFFF, 0.5);
        let r = (out >> 16) & 0xFF;
        assert!((126..=129).contains(&r), "got {}", r);
    }

    #[test]
    fn apply_hover_lightens_base_with_white_overlay() {
        let base = 0xFF20_2020;
        let lit = apply_state(base, 0xFFFF_FFFF, State::Hover);
        assert!((lit & 0xFF) > (base & 0xFF));
    }
}
