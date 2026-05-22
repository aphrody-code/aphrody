// SPDX-License-Identifier: Apache-2.0
//! Material Design 3 shape — corner radius token scale.
//!
//! M3 defines a corner radius scale that controls how component edges are
//! rounded.  The M3 Expressive scale has ten named tokens ranging from `none`
//! (0 dp, perfectly square) through `full` (9999 dp, a fully rounded pill).
//!
//! Components are assigned a shape role by the M3 specification; the tokens
//! here are the authoritative values for those roles.
//!
//! Reference: <https://m3.material.io/styles/shape/corner-radius-scale>
//!
//! ## Token table
//!
//! | Token                  | dp   | CSS variable                                  |
//! |------------------------|------|-----------------------------------------------|
//! | none                   | 0    | `--md-sys-shape-corner-none`                  |
//! | extra-small            | 4    | `--md-sys-shape-corner-extra-small`           |
//! | small                  | 8    | `--md-sys-shape-corner-small`                 |
//! | medium                 | 12   | `--md-sys-shape-corner-medium`                |
//! | large                  | 16   | `--md-sys-shape-corner-large`                 |
//! | large-increased        | 20   | `--md-sys-shape-corner-large-increased`       |
//! | extra-large            | 28   | `--md-sys-shape-corner-extra-large`           |
//! | extra-large-increased  | 32   | `--md-sys-shape-corner-extra-large-increased` |
//! | extra-extra-large      | 48   | `--md-sys-shape-corner-extra-extra-large`     |
//! | full                   | 9999 | `--md-sys-shape-corner-full`                  |

/// A corner radius token value expressed in density-independent pixels (dp).
///
/// Construct values through the named [`pub const`] tokens in this module
/// (`NONE`, `EXTRA_SMALL`, … `FULL`) rather than constructing this struct
/// directly.  The `dp` field is intentionally `pub` to allow read access and
/// future helper methods without adding indirection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct CornerRadius {
    /// Radius in density-independent pixels.
    pub dp: u16,
}

/// No corner rounding — perfectly square (0 dp).
pub const NONE: CornerRadius = CornerRadius { dp: 0 };

/// Extra-small corner rounding (4 dp).
pub const EXTRA_SMALL: CornerRadius = CornerRadius { dp: 4 };

/// Small corner rounding (8 dp).
pub const SMALL: CornerRadius = CornerRadius { dp: 8 };

/// Medium corner rounding (12 dp).
pub const MEDIUM: CornerRadius = CornerRadius { dp: 12 };

/// Large corner rounding (16 dp).
pub const LARGE: CornerRadius = CornerRadius { dp: 16 };

/// Large-increased corner rounding (20 dp) — M3 Expressive.
pub const LARGE_INCREASED: CornerRadius = CornerRadius { dp: 20 };

/// Extra-large corner rounding (28 dp).
pub const EXTRA_LARGE: CornerRadius = CornerRadius { dp: 28 };

/// Extra-large-increased corner rounding (32 dp) — M3 Expressive.
pub const EXTRA_LARGE_INCREASED: CornerRadius = CornerRadius { dp: 32 };

/// Extra-extra-large corner rounding (48 dp) — M3 Expressive.
pub const EXTRA_EXTRA_LARGE: CornerRadius = CornerRadius { dp: 48 };

/// Full corner rounding — pill shape (9999 dp).
///
/// 9999 dp is the M3 sentinel value indicating a fully rounded (pill) shape.
/// Any radius greater than half the component's shortest side produces the
/// same visual result.
pub const FULL: CornerRadius = CornerRadius { dp: 9999 };

/// All ten corner-radius tokens in canonical M3 (Expressive) order.
pub const ALL: [CornerRadius; 10] = [
    NONE,
    EXTRA_SMALL,
    SMALL,
    MEDIUM,
    LARGE,
    LARGE_INCREASED,
    EXTRA_LARGE,
    EXTRA_LARGE_INCREASED,
    EXTRA_EXTRA_LARGE,
    FULL,
];

/// Kebab-case names for each token in [`ALL`] order.
///
/// Matches the CSS custom property suffix defined by the M3 specification:
/// `--md-sys-shape-corner-<name>`.
pub const NAMES: [&str; 10] = [
    "none",
    "extra-small",
    "small",
    "medium",
    "large",
    "large-increased",
    "extra-large",
    "extra-large-increased",
    "extra-extra-large",
    "full",
];

/// Emits a CSS `:root` block declaring all ten M3 shape corner-radius
/// custom properties.
///
/// Each property value is expressed as `<dp>dp` per M3 convention (not `px`),
/// matching the `--md-sys-shape-corner-*` naming scheme.
///
/// # Example
///
/// ```
/// # #[cfg(feature = "std")]
/// # {
/// let css = m3_tokens::shape::export_css();
/// assert!(css.contains("--md-sys-shape-corner-medium: 12dp;"));
/// # }
/// ```
#[cfg(feature = "std")]
#[must_use]
pub fn export_css() -> String {
    let mut out = String::from(":root {\n");
    for (token, name) in ALL.iter().zip(NAMES.iter()) {
        out.push_str(&format!("  --md-sys-shape-corner-{}: {}dp;\n", name, token.dp));
    }
    out.push('}');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_has_ten_entries() {
        assert_eq!(ALL.len(), 10);
    }

    #[test]
    fn names_has_ten_entries() {
        assert_eq!(NAMES.len(), 10);
    }

    #[test]
    fn canonical_ordering() {
        // Each token must be strictly larger than the previous one.
        assert!(FULL.dp > EXTRA_EXTRA_LARGE.dp);
        assert!(EXTRA_EXTRA_LARGE.dp > EXTRA_LARGE_INCREASED.dp);
        assert!(EXTRA_LARGE_INCREASED.dp > EXTRA_LARGE.dp);
        assert!(EXTRA_LARGE.dp > LARGE_INCREASED.dp);
        assert!(LARGE_INCREASED.dp > LARGE.dp);
        assert!(LARGE.dp > MEDIUM.dp);
        assert!(MEDIUM.dp > SMALL.dp);
        assert!(SMALL.dp > EXTRA_SMALL.dp);
        assert!(EXTRA_SMALL.dp > NONE.dp);
    }

    #[test]
    fn all_array_is_ordered() {
        for window in ALL.windows(2) {
            assert!(
                window[0].dp < window[1].dp,
                "ALL must be strictly ascending; {} >= {}",
                window[0].dp,
                window[1].dp,
            );
        }
    }

    #[test]
    fn names_match_all_length() {
        assert_eq!(ALL.len(), NAMES.len());
    }

    #[test]
    fn canonical_dp_values() {
        assert_eq!(NONE.dp, 0);
        assert_eq!(EXTRA_SMALL.dp, 4);
        assert_eq!(SMALL.dp, 8);
        assert_eq!(MEDIUM.dp, 12);
        assert_eq!(LARGE.dp, 16);
        assert_eq!(LARGE_INCREASED.dp, 20);
        assert_eq!(EXTRA_LARGE.dp, 28);
        assert_eq!(EXTRA_LARGE_INCREASED.dp, 32);
        assert_eq!(EXTRA_EXTRA_LARGE.dp, 48);
        assert_eq!(FULL.dp, 9999);
    }

    #[cfg(feature = "std")]
    #[test]
    fn export_css_contains_medium() {
        let css = export_css();
        assert!(
            css.contains("--md-sys-shape-corner-medium"),
            "export_css must emit --md-sys-shape-corner-medium"
        );
    }

    #[cfg(feature = "std")]
    #[test]
    fn export_css_has_root_block() {
        let css = export_css();
        assert!(css.starts_with(":root {"), "CSS must open with :root {{");
        assert!(css.ends_with('}'), "CSS must close with }}");
    }

    #[cfg(feature = "std")]
    #[test]
    fn export_css_contains_all_names() {
        let css = export_css();
        for name in NAMES {
            assert!(css.contains(name), "export_css must contain token name '{name}'");
        }
    }
}
