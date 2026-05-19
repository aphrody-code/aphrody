// SPDX-License-Identifier: Apache-2.0
//! Layout DSL — thin wrappers over `ratatui::layout` that keep call sites
//! concise without re-implementing the underlying solver.
//!
//! All helpers operate on the same [`ratatui::layout::Rect`] / [`Constraint`]
//! types as ratatui itself, so they compose with stock widgets.

use ratatui::layout::{Constraint, Direction, Layout as RtLayout, Rect};

// Re-export ratatui primitives so callers can do `use aphrody_tui::layout::*`
// and never need to add `ratatui` to their own Cargo.toml.
pub use ratatui::layout::{Constraint as RatatuiConstraint, Direction as RatatuiDirection};

/// Typed layout DSL — a thin facade over [`ratatui::layout::Layout`] matching
/// the API spec in CLAUDE.md §0.5 T-10 (`Layout::split(rect, &[Constraint])`).
///
/// ```
/// use aphrody_tui::layout::{Layout, RatatuiConstraint, RatatuiDirection};
/// use ratatui::layout::Rect;
///
/// let area = Rect::new(0, 0, 100, 30);
/// let rows = Layout::new(RatatuiDirection::Vertical)
///     .split(area, &[RatatuiConstraint::Length(3), RatatuiConstraint::Min(0)]);
/// assert_eq!(rows.len(), 2);
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Layout {
    direction: Direction,
}

impl Layout {
    /// New layout with the supplied direction.
    #[must_use]
    pub const fn new(direction: Direction) -> Self {
        Self { direction }
    }

    /// Vertical convenience constructor.
    #[must_use]
    pub const fn vertical() -> Self {
        Self::new(Direction::Vertical)
    }

    /// Horizontal convenience constructor.
    #[must_use]
    pub const fn horizontal() -> Self {
        Self::new(Direction::Horizontal)
    }

    /// Split `area` into one rect per `constraint`.
    #[must_use]
    pub fn split(self, area: Rect, constraints: &[Constraint]) -> Vec<Rect> {
        RtLayout::default()
            .direction(self.direction)
            .constraints(constraints.to_vec())
            .split(area)
            .to_vec()
    }
}

/// Split `area` vertically using the supplied constraints.
///
/// ```
/// use aphrody_tui::layout::vsplit;
/// use ratatui::layout::{Constraint, Rect};
///
/// let area = Rect::new(0, 0, 80, 24);
/// let rows = vsplit(area, &[Constraint::Length(3), Constraint::Min(0), Constraint::Length(1)]);
/// assert_eq!(rows.len(), 3);
/// assert_eq!(rows[0].height, 3);
/// assert_eq!(rows[2].height, 1);
/// ```
#[must_use]
pub fn vsplit(area: Rect, constraints: &[Constraint]) -> Vec<Rect> {
    RtLayout::default()
        .direction(Direction::Vertical)
        .constraints(constraints.to_vec())
        .split(area)
        .to_vec()
}

/// Split `area` horizontally using the supplied constraints.
///
/// ```
/// use aphrody_tui::layout::hsplit;
/// use ratatui::layout::{Constraint, Rect};
///
/// let area = Rect::new(0, 0, 80, 24);
/// let cols = hsplit(area, &[Constraint::Percentage(30), Constraint::Percentage(70)]);
/// assert_eq!(cols.len(), 2);
/// ```
#[must_use]
pub fn hsplit(area: Rect, constraints: &[Constraint]) -> Vec<Rect> {
    RtLayout::default()
        .direction(Direction::Horizontal)
        .constraints(constraints.to_vec())
        .split(area)
        .to_vec()
}

/// Centre a `width` x `height` rect inside `area`, clamping if the requested
/// size exceeds the available space.  Useful for modal dialogs.
#[must_use]
pub fn center(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(h)) / 2;
    Rect::new(x, y, w, h)
}

/// Convenience constructor mirroring the `vsplit![...]` macro pattern used in
/// some community ratatui crates but expressed as a function so we don't
/// have to ship a proc-macro.  Returns the constraint vector ready to be
/// passed to [`vsplit`].
#[must_use]
pub fn lengths(values: &[u16]) -> Vec<Constraint> {
    values.iter().copied().map(Constraint::Length).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vsplit_three_constraints_yields_three_rects() {
        let area = Rect::new(0, 0, 40, 20);
        let rows =
            vsplit(area, &[Constraint::Length(3), Constraint::Min(0), Constraint::Length(1)]);
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].y, 0);
        assert_eq!(rows[0].height, 3);
        // Middle row absorbs the remainder.
        assert_eq!(rows[1].y, 3);
        assert_eq!(rows[1].height, 20 - 3 - 1);
        assert_eq!(rows[2].y, 19);
        assert_eq!(rows[2].height, 1);
    }

    #[test]
    fn hsplit_percentages_sum_to_full_width() {
        let area = Rect::new(0, 0, 100, 10);
        let cols = hsplit(area, &[Constraint::Percentage(30), Constraint::Percentage(70)]);
        assert_eq!(cols.len(), 2);
        assert_eq!(cols[0].width + cols[1].width, 100);
    }

    #[test]
    fn center_clamps_to_area() {
        let area = Rect::new(0, 0, 10, 4);
        let modal = center(area, 100, 100);
        assert_eq!(modal.width, 10);
        assert_eq!(modal.height, 4);
    }

    #[test]
    fn lengths_helper_round_trips() {
        let cs = lengths(&[5, 1, 1]);
        assert_eq!(cs.len(), 3);
        assert!(matches!(cs[0], Constraint::Length(5)));
    }
}
