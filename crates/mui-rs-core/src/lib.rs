// SPDX-License-Identifier: Apache-2.0
//! `mui-rs-core` — fundamental types and traits shared across the mui-rs crates.
//!
//! This is the foundation layer: it re-exports the [`m3_tokens`] design tokens
//! and defines the [`Theme`] that components, the renderer, and motion all read
//! from. aphrody UIs are **dark-first** — [`Theme::default`] is the aphrody dark
//! rust theme.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub use m3_tokens;
pub use m3_tokens::color::{ColorRoles, APHRODY, APHRODY_DARK, BASELINE, BASELINE_DARK};

/// A resolved UI theme: the active [`ColorRoles`] plus whether it is dark.
///
/// Construct one of the presets ([`Theme::aphrody_dark`] etc.) or build from an
/// arbitrary [`ColorRoles`] with [`Theme::new`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Theme {
    /// The active color roles (all M3 roles, incl. expanded surfaces).
    pub color: ColorRoles,
    /// Whether this is a dark theme (drives elevation tint, shadows, etc.).
    pub dark: bool,
}

impl Theme {
    /// Build a theme from explicit color roles.
    #[must_use]
    pub const fn new(color: ColorRoles, dark: bool) -> Self {
        Self { color, dark }
    }

    /// aphrody brand dark theme (rust seed `#CE422B`) — the project default.
    #[must_use]
    pub const fn aphrody_dark() -> Self {
        Self { color: APHRODY_DARK, dark: true }
    }

    /// aphrody brand light theme (rust seed `#CE422B`).
    #[must_use]
    pub const fn aphrody_light() -> Self {
        Self { color: APHRODY, dark: false }
    }

    /// M3 baseline (purple seed `#6750A4`) dark theme.
    #[must_use]
    pub const fn baseline_dark() -> Self {
        Self { color: BASELINE_DARK, dark: true }
    }

    /// M3 baseline (purple seed `#6750A4`) light theme.
    #[must_use]
    pub const fn baseline_light() -> Self {
        Self { color: BASELINE, dark: false }
    }
}

impl Default for Theme {
    /// aphrody is dark-first.
    fn default() -> Self {
        Self::aphrody_dark()
    }
}

/// Common imports for downstream mui-rs crates.
pub mod prelude {
    pub use crate::{ColorRoles, Theme};
    pub use m3_tokens::shape::CornerRadius;
    pub use m3_tokens::typography::TypeStyle;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_aphrody_dark() {
        let t = Theme::default();
        assert!(t.dark);
        assert_eq!(t.color.primary, APHRODY_DARK.primary);
    }

    #[test]
    fn presets_differ() {
        assert_ne!(Theme::aphrody_dark().color.primary, Theme::baseline_dark().color.primary);
        assert!(!Theme::aphrody_light().dark);
        // expanded surface role reachable through the theme.
        assert_eq!(Theme::aphrody_dark().color.surface_container, 0xFF271D1C);
    }
}
