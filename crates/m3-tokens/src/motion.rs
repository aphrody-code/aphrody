// SPDX-License-Identifier: Apache-2.0
//! Material Design 3 motion tokens.
//!
//! Provides standard easing curves and duration values that define the M3
//! motion system.  Use these tokens instead of ad-hoc `cubic-bezier` or
//! hand-picked millisecond values.
//!
//! Reference: <https://m3.material.io/styles/motion/easing-and-duration/tokens-specs>

// ── Easing curves ────────────────────────────────────────────────────────────

/// A named M3 easing curve expressed as a CSS `cubic-bezier(...)` string.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Easing {
    /// Human-readable name matching the M3 specification token name.
    pub name: &'static str,
    /// CSS `cubic-bezier(x1, y1, x2, y2)` string, ready for insertion into
    /// a `transition-timing-function` or `animation-timing-function` property.
    pub cubic_bezier: &'static str,
}

/// Emphasized — the default easing for most M3 transitions.
///
/// Produces a deceleration curve that starts fast and eases out, appropriate
/// for elements entering the screen or responding to user input.
pub const EASING_EMPHASIZED: Easing =
    Easing { name: "emphasized", cubic_bezier: "cubic-bezier(0.2, 0.0, 0, 1.0)" };

/// Emphasized decelerate — for elements entering the screen.
pub const EASING_EMPHASIZED_DECELERATE: Easing =
    Easing { name: "emphasized-decelerate", cubic_bezier: "cubic-bezier(0.05, 0.7, 0.1, 1.0)" };

/// Emphasized accelerate — for elements leaving the screen.
pub const EASING_EMPHASIZED_ACCELERATE: Easing =
    Easing { name: "emphasized-accelerate", cubic_bezier: "cubic-bezier(0.3, 0.0, 0.8, 0.15)" };

/// Standard — general-purpose easing for components that stay on screen.
pub const EASING_STANDARD: Easing =
    Easing { name: "standard", cubic_bezier: "cubic-bezier(0.2, 0.0, 0, 1.0)" };

/// Standard decelerate — entering or expanding element with standard curve.
pub const EASING_STANDARD_DECELERATE: Easing =
    Easing { name: "standard-decelerate", cubic_bezier: "cubic-bezier(0, 0, 0, 1)" };

/// Standard accelerate — exiting or collapsing element with standard curve.
pub const EASING_STANDARD_ACCELERATE: Easing =
    Easing { name: "standard-accelerate", cubic_bezier: "cubic-bezier(0.3, 0, 1, 1)" };

/// Linear — constant rate; for continuous indicators (progress, scrubbing).
pub const EASING_LINEAR: Easing =
    Easing { name: "linear", cubic_bezier: "cubic-bezier(0, 0, 1, 1)" };

/// All seven M3 easing curves in spec order.
pub const ALL_EASINGS: [Easing; 7] = [
    EASING_EMPHASIZED,
    EASING_EMPHASIZED_DECELERATE,
    EASING_EMPHASIZED_ACCELERATE,
    EASING_STANDARD,
    EASING_STANDARD_DECELERATE,
    EASING_STANDARD_ACCELERATE,
    EASING_LINEAR,
];

// ── Durations ────────────────────────────────────────────────────────────────

/// Duration token pairing a name with a millisecond value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Duration {
    /// Specification token name (e.g. `"short1"`).
    pub name: &'static str,
    /// Duration in milliseconds.
    pub ms: u16,
}

/// Short1 — 50 ms.  Micro-interactions, icon state changes.
pub const DURATION_SHORT1: Duration = Duration { name: "short1", ms: 50 };

/// Short2 — 100 ms.  Small component responses (checkbox, radio).
pub const DURATION_SHORT2: Duration = Duration { name: "short2", ms: 100 };

/// Short3 — 150 ms.  Default for simple component transitions (FAB collapse).
pub const DURATION_SHORT3: Duration = Duration { name: "short3", ms: 150 };

/// Short4 — 200 ms.  Tooltip, menu appear.
pub const DURATION_SHORT4: Duration = Duration { name: "short4", ms: 200 };

/// Medium1 — 250 ms.  Card expansion, dialog reveal.
pub const DURATION_MEDIUM1: Duration = Duration { name: "medium1", ms: 250 };

/// Medium2 — 300 ms.  Navigation transition, sheet.
pub const DURATION_MEDIUM2: Duration = Duration { name: "medium2", ms: 300 };

/// Medium3 — 350 ms.  Complex component transitions.
pub const DURATION_MEDIUM3: Duration = Duration { name: "medium3", ms: 350 };

/// Medium4 — 400 ms.  Larger component transitions.
pub const DURATION_MEDIUM4: Duration = Duration { name: "medium4", ms: 400 };

/// Long1 — 450 ms.  Complex layout transitions.
pub const DURATION_LONG1: Duration = Duration { name: "long1", ms: 450 };

/// Long2 — 500 ms.  Full-screen transitions.
pub const DURATION_LONG2: Duration = Duration { name: "long2", ms: 500 };

/// Long3 — 550 ms.
pub const DURATION_LONG3: Duration = Duration { name: "long3", ms: 550 };

/// Long4 — 600 ms.
pub const DURATION_LONG4: Duration = Duration { name: "long4", ms: 600 };

/// Extra-long1 — 700 ms.  Large expressive transitions.
pub const DURATION_EXTRA_LONG1: Duration = Duration { name: "extra-long1", ms: 700 };

/// Extra-long2 — 800 ms.
pub const DURATION_EXTRA_LONG2: Duration = Duration { name: "extra-long2", ms: 800 };

/// Extra-long3 — 900 ms.
pub const DURATION_EXTRA_LONG3: Duration = Duration { name: "extra-long3", ms: 900 };

/// Extra-long4 — 1000 ms.  Largest expressive transitions.
pub const DURATION_EXTRA_LONG4: Duration = Duration { name: "extra-long4", ms: 1000 };

/// All 16 M3 duration tokens in ascending order (short1 → extra-long4).
pub const ALL_DURATIONS: [Duration; 16] = [
    DURATION_SHORT1,
    DURATION_SHORT2,
    DURATION_SHORT3,
    DURATION_SHORT4,
    DURATION_MEDIUM1,
    DURATION_MEDIUM2,
    DURATION_MEDIUM3,
    DURATION_MEDIUM4,
    DURATION_LONG1,
    DURATION_LONG2,
    DURATION_LONG3,
    DURATION_LONG4,
    DURATION_EXTRA_LONG1,
    DURATION_EXTRA_LONG2,
    DURATION_EXTRA_LONG3,
    DURATION_EXTRA_LONG4,
];

/// The six primary M3 duration values in milliseconds (50/150/200/250/300/400).
///
/// These are the durations most frequently referenced in the M3 motion guide
/// and correspond to the six archetypal use-cases.
pub const PRIMARY_DURATIONS_MS: [u16; 6] = [50, 150, 200, 250, 300, 400];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seven_standard_easings() {
        assert_eq!(ALL_EASINGS.len(), 7);
        assert_eq!(EASING_LINEAR.cubic_bezier, "cubic-bezier(0, 0, 1, 1)");
    }

    #[test]
    fn all_easings_have_cubic_bezier() {
        for e in &ALL_EASINGS {
            assert!(
                e.cubic_bezier.starts_with("cubic-bezier("),
                "easing '{}' must use cubic-bezier notation",
                e.name
            );
        }
    }

    #[test]
    fn primary_durations_are_in_spec_order() {
        assert_eq!(PRIMARY_DURATIONS_MS, [50, 150, 200, 250, 300, 400]);
    }

    #[test]
    fn all_durations_ascending() {
        for window in ALL_DURATIONS.windows(2) {
            assert!(window[0].ms <= window[1].ms, "durations must be non-decreasing");
        }
    }

    #[test]
    fn long_and_extra_long_match_spec() {
        assert_eq!(DURATION_LONG1.ms, 450);
        assert_eq!(DURATION_LONG2.ms, 500);
        assert_eq!(DURATION_EXTRA_LONG4.ms, 1000);
        assert_eq!(ALL_DURATIONS.len(), 16);
    }
}
