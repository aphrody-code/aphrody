//! M3 transition types — Forward/Backward, Enter/Exit, Shared element.

use m3_tokens::motion::{Easing, DURATION_LONG2, DURATION_LONG1, DURATION_MEDIUM2, EASING_EMPHASIZED, EASING_STANDARD};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitionPattern {
    /// Container transform — shared element expansion
    ContainerTransform,
    /// Shared axis — horizontal/vertical/z navigation
    SharedAxisX,
    SharedAxisY,
    SharedAxisZ,
    /// Fade through — content swap
    FadeThrough,
    /// Fade — overlay patterns
    Fade,
}

#[derive(Debug, Clone)]
pub struct Transition {
    pub pattern: TransitionPattern,
    pub duration_ms: u32,
    pub easing: Easing,
}

impl Transition {
    pub fn container_transform() -> Self {
        Self { pattern: TransitionPattern::ContainerTransform, duration_ms: DURATION_LONG2.ms as u32, easing: EASING_EMPHASIZED }
    }

    pub fn shared_axis_x() -> Self {
        Self { pattern: TransitionPattern::SharedAxisX, duration_ms: DURATION_LONG1.ms as u32, easing: EASING_EMPHASIZED }
    }

    pub fn fade_through() -> Self {
        Self { pattern: TransitionPattern::FadeThrough, duration_ms: DURATION_MEDIUM2.ms as u32, easing: EASING_STANDARD }
    }
}
