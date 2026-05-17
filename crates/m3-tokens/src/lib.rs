// SPDX-License-Identifier: Apache-2.0
//! Material Design 3 baseline design tokens.
//!
//! Provides compile-time constants for color roles, typography scale,
//! elevation levels, and motion parameters drawn from the canonical M3
//! specification.  The crate is `no_std` by default; the `std` feature
//! (enabled by default) gates helpers that allocate (e.g. [`export_css`]).
//!
//! # Feature flags
//!
//! | Flag  | Default | Effect |
//! |-------|---------|--------|
//! | `std` | yes     | Enables [`export_css`] and the [`alloc`] prelude. |
//!
//! # Quick start
//!
//! ```rust
//! use m3_tokens::color::BASELINE;
//! // primary color role — 0xFF6750A4
//! assert_eq!(BASELINE.primary, 0xFF6750A4);
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

#[cfg(feature = "std")] extern crate std;

pub mod color;
#[cfg(test)]
pub mod dynamic;
pub mod elevation;
pub mod gemini_brand;
pub mod google_sans_flex;
pub mod motion;
pub mod shape;
pub mod state;
pub mod tonal;
pub mod typography;

// Re-export the CSS exporter at crate root when std is available.
#[cfg(feature = "std")] pub use color::export_css;

// ── Top-level integration tests ──────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use crate::typography::ALL as TYPE_SCALE;

    /// Enforce that the canonical type scale has exactly 15 entries.
    #[test]
    fn type_scale_count_is_15() {
        assert_eq!(TYPE_SCALE.len(), 15, "M3 type scale must contain exactly 15 type styles");
    }

    /// Enforce that the CSS export declares the primary color variable.
    #[cfg(feature = "std")]
    #[test]
    fn css_export_contains_primary() {
        use crate::color::{BASELINE, export_css};
        let css = export_css(&BASELINE);
        assert!(
            css.contains("--md-sys-color-primary:"),
            "export_css must emit --md-sys-color-primary"
        );
    }
}
