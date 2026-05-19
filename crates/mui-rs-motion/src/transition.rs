//! M3 transition types — Forward/Backward, Enter/Exit, Shared element.

use mui_rs_tokens::motion::{Easing, duration};

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
        Self { pattern: TransitionPattern::ContainerTransform, duration_ms: duration::MEDIUM4, easing: Easing::EMPHASIZED }
    }

    pub fn shared_axis_x() -> Self {
        Self { pattern: TransitionPattern::SharedAxisX, duration_ms: duration::MEDIUM3, easing: Easing::EMPHASIZED }
    }

    pub fn fade_through() -> Self {
        Self { pattern: TransitionPattern::FadeThrough, duration_ms: duration::MEDIUM2, easing: Easing::STANDARD }
    }
}
