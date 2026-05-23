// SPDX-License-Identifier: Apache-2.0
//! Material Design 3 adaptive layout — native Rust implementation.
//!
//! Window size classes (now called *breakpoints*), recommended pane counts,
//! and the navigation / action / dialog component swaps M3 prescribes per
//! breakpoint.
//!
//! Reference: <https://m3.material.io/foundations/layout/applying-layout/window-size-classes>
//!
//! | Breakpoint  | Width (dp)  | Panes (rec) | Navigation             |
//! |-------------|-------------|-------------|------------------------|
//! | Compact     | `< 600`     | 1           | Navigation bar         |
//! | Medium      | `600–839`   | 1 (or 2)    | Collapsed nav rail     |
//! | Expanded    | `840–1199`  | 2           | Collapsed nav rail     |
//! | Large       | `1200–1599` | 2           | Expanded nav rail      |
//! | Extra-large | `>= 1600`   | 3           | Expanded nav rail      |

#![allow(clippy::missing_const_for_fn)]

/// An M3 breakpoint (formerly *window size class*), classifying a window
/// **width** in density-independent pixels (dp).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Breakpoint {
    /// `< 600dp` — phone in portrait.
    Compact,
    /// `600–839dp` — tablet/foldable in portrait.
    Medium,
    /// `840–1199dp` — phone/tablet/foldable in landscape, desktop.
    Expanded,
    /// `1200–1599dp` — desktop.
    Large,
    /// `>= 1600dp` — desktop, ultra-wide monitors.
    ExtraLarge,
}

/// Recommended navigation component for a breakpoint (M3 swap guidance).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Navigation {
    /// Bottom navigation bar (compact).
    Bar,
    /// Collapsed navigation rail (medium / expanded).
    RailCollapsed,
    /// Standard expanded navigation rail (large / extra-large).
    RailExpanded,
}

/// Recommended supplemental-selection / action surface for a breakpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionSurface {
    /// Bottom sheet (compact).
    BottomSheet,
    /// Menu (medium and up).
    Menu,
}

/// Recommended dialog type for a breakpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dialog {
    /// Full-screen or basic dialog (compact).
    FullScreenOrBasic,
    /// Basic dialog (medium and up).
    Basic,
}

impl Breakpoint {
    /// Every breakpoint, ascending.
    pub const ALL: [Breakpoint; 5] = [
        Breakpoint::Compact,
        Breakpoint::Medium,
        Breakpoint::Expanded,
        Breakpoint::Large,
        Breakpoint::ExtraLarge,
    ];

    /// Classify a window width (dp) into its M3 breakpoint.
    ///
    /// ```rust
    /// use m3_tokens::adaptive::Breakpoint;
    /// assert_eq!(Breakpoint::from_width_dp(599), Breakpoint::Compact);
    /// assert_eq!(Breakpoint::from_width_dp(600), Breakpoint::Medium);
    /// assert_eq!(Breakpoint::from_width_dp(840), Breakpoint::Expanded);
    /// assert_eq!(Breakpoint::from_width_dp(1200), Breakpoint::Large);
    /// assert_eq!(Breakpoint::from_width_dp(1600), Breakpoint::ExtraLarge);
    /// ```
    #[must_use]
    pub const fn from_width_dp(width_dp: u32) -> Self {
        if width_dp < 600 {
            Self::Compact
        } else if width_dp < 840 {
            Self::Medium
        } else if width_dp < 1200 {
            Self::Expanded
        } else if width_dp < 1600 {
            Self::Large
        } else {
            Self::ExtraLarge
        }
    }

    /// Inclusive lower bound of this breakpoint's width range, in dp.
    #[must_use]
    pub const fn min_width_dp(self) -> u32 {
        match self {
            Self::Compact => 0,
            Self::Medium => 600,
            Self::Expanded => 840,
            Self::Large => 1200,
            Self::ExtraLarge => 1600,
        }
    }

    /// Exclusive upper bound of this breakpoint's width range, in dp.
    /// `None` for [`Breakpoint::ExtraLarge`] (unbounded).
    #[must_use]
    pub const fn max_width_dp(self) -> Option<u32> {
        match self {
            Self::Compact => Some(600),
            Self::Medium => Some(840),
            Self::Expanded => Some(1200),
            Self::Large => Some(1600),
            Self::ExtraLarge => None,
        }
    }

    /// Recommended number of layout panes.
    #[must_use]
    pub const fn recommended_panes(self) -> u8 {
        match self {
            Self::Compact | Self::Medium => 1,
            Self::Expanded | Self::Large => 2,
            Self::ExtraLarge => 3,
        }
    }

    /// Maximum supported layout panes.
    #[must_use]
    pub const fn max_panes(self) -> u8 {
        match self {
            Self::Compact => 1,
            Self::Medium | Self::Expanded | Self::Large => 2,
            Self::ExtraLarge => 3,
        }
    }

    /// Recommended navigation component (M3 component-swap guidance).
    #[must_use]
    pub const fn navigation(self) -> Navigation {
        match self {
            Self::Compact => Navigation::Bar,
            Self::Medium | Self::Expanded => Navigation::RailCollapsed,
            Self::Large | Self::ExtraLarge => Navigation::RailExpanded,
        }
    }

    /// Recommended supplemental action surface.
    #[must_use]
    pub const fn action_surface(self) -> ActionSurface {
        match self {
            Self::Compact => ActionSurface::BottomSheet,
            _ => ActionSurface::Menu,
        }
    }

    /// Recommended dialog type.
    #[must_use]
    pub const fn dialog(self) -> Dialog {
        match self {
            Self::Compact => Dialog::FullScreenOrBasic,
            _ => Dialog::Basic,
        }
    }

    /// Lowercase token slug (e.g. `"compact"`, `"extra-large"`). Used for CSS
    /// custom-property names and data attributes.
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            Self::Compact => "compact",
            Self::Medium => "medium",
            Self::Expanded => "expanded",
            Self::Large => "large",
            Self::ExtraLarge => "extra-large",
        }
    }
}

/// Emits the M3 breakpoint minimums as CSS custom properties **and** Tailwind
/// v4 `--breakpoint-*` tokens (so `@media` / responsive utilities resolve to
/// the canonical M3 widths). Pairs with `m3_tokens::color::export_*` in the UI
/// fusion (see `docs/design/FUSION-PLAN.md`).
///
/// Only available with the `std` feature.
///
/// # Example
///
/// ```rust
/// use m3_tokens::adaptive::export_breakpoints_css;
/// let css = export_breakpoints_css();
/// assert!(css.contains("--md-sys-breakpoint-medium: 600px;"));
/// assert!(css.contains("--breakpoint-expanded: 840px;"));
/// ```
#[cfg(feature = "std")]
pub fn export_breakpoints_css() -> std::string::String {
    let mut sys = std::string::String::from(":root {\n");
    let mut theme = std::string::String::from("@theme {\n");
    for bp in Breakpoint::ALL {
        let (slug, min) = (bp.token(), bp.min_width_dp());
        sys.push_str(&std::format!("  --md-sys-breakpoint-{slug}: {min}px;\n"));
        theme.push_str(&std::format!("  --breakpoint-{slug}: {min}px;\n"));
    }
    sys.push_str("}\n\n");
    theme.push('}');
    sys.push_str(&theme);
    sys
}

// ============================================================================
//  Parts of layout (the layout scaffold).
//  Reference: https://m3.material.io/foundations/layout/understanding-layout/parts-of-layout
// ============================================================================

/// A level of the M3 layout scaffold. Ordered outermost → innermost: bars frame
/// the content, rails fill the perimeter around panes, panes hold all content.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScaffoldRegion {
    /// Top frame: app bar / navigation bar (spans one or multiple panes).
    Bars,
    /// Perimeter around panes: navigation rail, toolbar, FAB, primary controls.
    Rails,
    /// The content area. **All content must live in a pane** (1–3 panes).
    Panes,
}

/// Sizing behaviour of a pane within a multi-pane layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaneKind {
    /// Fixed-width pane (e.g. a list); can be collapsed/expanded via drag handle.
    Fixed,
    /// Flexible pane that absorbs remaining width (e.g. a detail view).
    Flexible,
}

/// How adjacent panes are visually separated.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Containment {
    /// Panes blend with the background (implicit grouping via spacing).
    Implicit,
    /// Distinct colors/outlines delineate panes (used in XR / for emphasis).
    Explicit,
}

/// Spacer between panes, in dp (M3 standard pane gutter).
pub const PANE_SPACER_DP: u32 = 24;

impl Breakpoint {
    /// M3 responsive-grid column count for this breakpoint.
    #[must_use]
    pub const fn grid_columns(self) -> u8 {
        match self {
            Self::Compact => 4,
            Self::Medium => 12,
            Self::Expanded | Self::Large | Self::ExtraLarge => 12,
        }
    }

    /// Window margin (dp) for this breakpoint (also used as the grid gutter).
    #[must_use]
    pub const fn margin_dp(self) -> u32 {
        match self {
            Self::Compact => 16,
            _ => 24,
        }
    }

    /// Recommended pane composition for this breakpoint. For right-to-left
    /// languages the navigation region mirrors to the right; the pane order
    /// here is logical (leading → trailing), not physical.
    ///
    /// ```rust
    /// use m3_tokens::adaptive::{Breakpoint, PaneKind};
    /// assert_eq!(Breakpoint::Compact.pane_layout(), &[PaneKind::Flexible]);
    /// assert_eq!(
    ///     Breakpoint::Expanded.pane_layout(),
    ///     &[PaneKind::Fixed, PaneKind::Flexible],
    /// );
    /// assert_eq!(Breakpoint::ExtraLarge.pane_layout().len(), 3);
    /// ```
    #[must_use]
    pub const fn pane_layout(self) -> &'static [PaneKind] {
        match self {
            Self::Compact | Self::Medium => &[PaneKind::Flexible],
            Self::Expanded | Self::Large => &[PaneKind::Fixed, PaneKind::Flexible],
            Self::ExtraLarge => {
                &[PaneKind::Fixed, PaneKind::Flexible, PaneKind::Fixed]
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classification_boundaries() {
        assert_eq!(Breakpoint::from_width_dp(0), Breakpoint::Compact);
        assert_eq!(Breakpoint::from_width_dp(599), Breakpoint::Compact);
        assert_eq!(Breakpoint::from_width_dp(600), Breakpoint::Medium);
        assert_eq!(Breakpoint::from_width_dp(839), Breakpoint::Medium);
        assert_eq!(Breakpoint::from_width_dp(840), Breakpoint::Expanded);
        assert_eq!(Breakpoint::from_width_dp(1199), Breakpoint::Expanded);
        assert_eq!(Breakpoint::from_width_dp(1200), Breakpoint::Large);
        assert_eq!(Breakpoint::from_width_dp(1599), Breakpoint::Large);
        assert_eq!(Breakpoint::from_width_dp(1600), Breakpoint::ExtraLarge);
        assert_eq!(Breakpoint::from_width_dp(4096), Breakpoint::ExtraLarge);
    }

    #[test]
    fn ranges_are_contiguous() {
        // Each breakpoint's max equals the next one's min.
        for pair in Breakpoint::ALL.windows(2) {
            assert_eq!(pair[0].max_width_dp(), Some(pair[1].min_width_dp()));
        }
        assert_eq!(Breakpoint::ExtraLarge.max_width_dp(), None);
    }

    #[test]
    fn navigation_swaps_match_spec() {
        assert_eq!(Breakpoint::Compact.navigation(), Navigation::Bar);
        assert_eq!(Breakpoint::Medium.navigation(), Navigation::RailCollapsed);
        assert_eq!(Breakpoint::Expanded.navigation(), Navigation::RailCollapsed);
        assert_eq!(Breakpoint::Large.navigation(), Navigation::RailExpanded);
        assert_eq!(Breakpoint::ExtraLarge.navigation(), Navigation::RailExpanded);
    }

    #[test]
    fn panes_match_spec() {
        assert_eq!(Breakpoint::Compact.recommended_panes(), 1);
        assert_eq!(Breakpoint::Expanded.recommended_panes(), 2);
        assert_eq!(Breakpoint::ExtraLarge.recommended_panes(), 3);
        assert_eq!(Breakpoint::Compact.max_panes(), 1);
        assert_eq!(Breakpoint::ExtraLarge.max_panes(), 3);
    }

    #[test]
    fn action_and_dialog_swaps() {
        assert_eq!(Breakpoint::Compact.action_surface(), ActionSurface::BottomSheet);
        assert_eq!(Breakpoint::Medium.action_surface(), ActionSurface::Menu);
        assert_eq!(Breakpoint::Compact.dialog(), Dialog::FullScreenOrBasic);
        assert_eq!(Breakpoint::Large.dialog(), Dialog::Basic);
    }

    #[test]
    fn layout_parts_match_spec() {
        assert_eq!(Breakpoint::Compact.grid_columns(), 4);
        assert_eq!(Breakpoint::Medium.grid_columns(), 12);
        assert_eq!(Breakpoint::Compact.margin_dp(), 16);
        assert_eq!(Breakpoint::Expanded.margin_dp(), 24);
        assert_eq!(PANE_SPACER_DP, 24);
        // pane_layout length agrees with recommended_panes.
        for bp in Breakpoint::ALL {
            assert_eq!(bp.pane_layout().len() as u8, bp.recommended_panes());
        }
        assert!(ScaffoldRegion::Bars < ScaffoldRegion::Panes);
        assert_ne!(Containment::Implicit, Containment::Explicit);
    }

    #[cfg(feature = "std")]
    #[test]
    fn css_export_has_both_namespaces() {
        let css = export_breakpoints_css();
        assert!(css.contains("--md-sys-breakpoint-compact: 0px;"));
        assert!(css.contains("--md-sys-breakpoint-extra-large: 1600px;"));
        assert!(css.contains("--breakpoint-large: 1200px;"));
        assert!(css.contains("@theme {"));
    }
}
