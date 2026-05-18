//! M3 motion: 8 easing curves + canonical durations.

use serde::{Deserialize, Serialize};

/// M3 easing class. `bezier()` returns the four control values for the
/// CSS `cubic-bezier(x1, y1, x2, y2)` function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Easing {
    Linear,
    Standard,
    StandardDecelerate,
    StandardAccelerate,
    Emphasized,
    EmphasizedDecelerate,
    EmphasizedAccelerate,
    /// Material 2 fallback ("Legacy") — `cubic-bezier(0.4, 0.0, 0.2, 1.0)`.
    Legacy,
}

impl Easing {
    /// Canonical M3 bezier control points (x1, y1, x2, y2).
    #[must_use]
    pub const fn bezier(self) -> (f64, f64, f64, f64) {
        match self {
            Self::Linear => (0.0, 0.0, 1.0, 1.0),
            Self::Standard => (0.2, 0.0, 0.0, 1.0),
            Self::StandardDecelerate => (0.0, 0.0, 0.0, 1.0),
            Self::StandardAccelerate => (0.3, 0.0, 1.0, 1.0),
            Self::Emphasized => (0.2, 0.0, 0.0, 1.0),
            Self::EmphasizedDecelerate => (0.05, 0.7, 0.1, 1.0),
            Self::EmphasizedAccelerate => (0.3, 0.0, 0.8, 0.15),
            Self::Legacy => (0.4, 0.0, 0.2, 1.0),
        }
    }

    /// CSS `cubic-bezier(...)` string.
    #[must_use]
    pub fn css(self) -> String {
        let (a, b, c, d) = self.bezier();
        format!("cubic-bezier({a}, {b}, {c}, {d})")
    }

    /// All 8 easing classes.
    #[must_use]
    pub const fn all() -> &'static [Easing] {
        &[
            Self::Linear,
            Self::Standard,
            Self::StandardDecelerate,
            Self::StandardAccelerate,
            Self::Emphasized,
            Self::EmphasizedDecelerate,
            Self::EmphasizedAccelerate,
            Self::Legacy,
        ]
    }
}

/// Canonical M3 durations (ms): short1..long4.
pub const DURATIONS_MS: [u32; 15] = [
    50, 100, 150, 200, 250, 300, 350, 400, 450, 500, 600, 700, 800, 900, 1000,
];

/// Named M3 duration tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DurationToken {
    Short1,
    Short2,
    Short3,
    Short4,
    Medium1,
    Medium2,
    Medium3,
    Medium4,
    Long1,
    Long2,
    Long3,
    Long4,
    ExtraLong1,
    ExtraLong2,
    ExtraLong3,
    ExtraLong4,
}

impl DurationToken {
    /// Duration in milliseconds.
    #[must_use]
    pub const fn ms(self) -> u32 {
        match self {
            Self::Short1 => 50,
            Self::Short2 => 100,
            Self::Short3 => 150,
            Self::Short4 => 200,
            Self::Medium1 => 250,
            Self::Medium2 => 300,
            Self::Medium3 => 350,
            Self::Medium4 => 400,
            Self::Long1 => 450,
            Self::Long2 => 500,
            Self::Long3 => 550,
            Self::Long4 => 600,
            Self::ExtraLong1 => 700,
            Self::ExtraLong2 => 800,
            Self::ExtraLong3 => 900,
            Self::ExtraLong4 => 1000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn motion_emphasized_easing_bezier() {
        let (a, b, c, d) = Easing::Emphasized.bezier();
        assert!((a - 0.2).abs() < 1e-9);
        assert!(b.abs() < 1e-9);
        assert!(c.abs() < 1e-9);
        assert!((d - 1.0).abs() < 1e-9);
    }

    #[test]
    fn motion_legacy_is_m2_fallback() {
        let (a, b, c, d) = Easing::Legacy.bezier();
        assert!((a - 0.4).abs() < 1e-9);
        assert!(b.abs() < 1e-9);
        assert!((c - 0.2).abs() < 1e-9);
        assert!((d - 1.0).abs() < 1e-9);
    }

    #[test]
    fn motion_durations_contain_canonical_values() {
        assert_eq!(DurationToken::Short1.ms(), 50);
        assert_eq!(DurationToken::Medium2.ms(), 300);
        assert_eq!(DurationToken::ExtraLong4.ms(), 1000);
        assert_eq!(DURATIONS_MS.len(), 15);
    }
}
