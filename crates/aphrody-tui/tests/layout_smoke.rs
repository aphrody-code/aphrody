// SPDX-License-Identifier: Apache-2.0
//! Smoke tests for the [`aphrody_tui::layout`] DSL — both the free
//! functions ([`vsplit`], [`hsplit`], [`center`], [`lengths`]) and the typed
//! [`Layout`] facade required by CLAUDE.md §0.5 T-10
//! (`Layout::split(rect, &[Constraint]) -> Vec<Rect>`).
//!
//! Layout invariants exercised:
//!   - children cover the full parent area (sum width / height)
//!   - children never overlap (start of N == end of N-1)
//!   - Length / Min / Percentage / Ratio all yield the expected geometry

use aphrody_tui::layout::{Layout, RatatuiConstraint, RatatuiDirection, center, hsplit, lengths, vsplit};
use ratatui::layout::{Constraint, Rect};

#[test]
fn layout_split_vertical_matches_vsplit_helper() {
    let area = Rect::new(0, 0, 100, 30);
    let constraints = [Constraint::Length(3), Constraint::Min(0), Constraint::Length(1)];

    let typed = Layout::vertical().split(area, &constraints);
    let free = vsplit(area, &constraints);

    assert_eq!(typed.len(), free.len());
    for (a, b) in typed.iter().zip(free.iter()) {
        assert_eq!(a, b);
    }
}

#[test]
fn layout_split_horizontal_matches_hsplit_helper() {
    let area = Rect::new(0, 0, 100, 30);
    let constraints = [Constraint::Percentage(40), Constraint::Percentage(60)];

    let typed = Layout::horizontal().split(area, &constraints);
    let free = hsplit(area, &constraints);

    assert_eq!(typed, free);
}

#[test]
fn vsplit_children_cover_parent() {
    let area = Rect::new(0, 0, 80, 24);
    let rows = vsplit(
        area,
        &[
            Constraint::Length(2),
            Constraint::Min(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ],
    );
    assert_eq!(rows.len(), 4);
    let total: u16 = rows.iter().map(|r| r.height).sum();
    assert_eq!(total, area.height, "rows must cover the full parent height");
    for win in rows.windows(2) {
        assert_eq!(win[0].y + win[0].height, win[1].y, "no gap between adjacent rows");
    }
}

#[test]
fn hsplit_children_cover_parent_at_arbitrary_widths() {
    let area = Rect::new(10, 5, 77, 13);
    let cols = hsplit(area, &[Constraint::Ratio(1, 3), Constraint::Ratio(2, 3)]);
    assert_eq!(cols.len(), 2);
    let total: u16 = cols.iter().map(|c| c.width).sum();
    assert_eq!(total, area.width, "cols must cover the full parent width");
    assert_eq!(cols[0].x, area.x);
    assert_eq!(cols[1].x, area.x + cols[0].width);
}

#[test]
fn center_clamps_oversized_rect() {
    let area = Rect::new(0, 0, 10, 4);
    let modal = center(area, 100, 100);
    assert_eq!(modal.width, 10);
    assert_eq!(modal.height, 4);
    assert_eq!(modal.x, 0);
    assert_eq!(modal.y, 0);
}

#[test]
fn center_places_smaller_rect_in_middle() {
    let area = Rect::new(0, 0, 20, 10);
    let modal = center(area, 6, 4);
    assert_eq!(modal.width, 6);
    assert_eq!(modal.height, 4);
    assert_eq!(modal.x, (20 - 6) / 2);
    assert_eq!(modal.y, (10 - 4) / 2);
}

#[test]
fn lengths_helper_round_trips() {
    let cs = lengths(&[5, 1, 1]);
    assert_eq!(cs.len(), 3);
    assert!(matches!(cs[0], Constraint::Length(5)));
    assert!(matches!(cs[1], Constraint::Length(1)));
    assert!(matches!(cs[2], Constraint::Length(1)));
}

#[test]
fn typed_layout_2x2_grid() {
    // The dashboard example uses a 2x2 grid; check it builds the way we expect.
    let area = Rect::new(0, 0, 80, 24);
    let rows = Layout::new(RatatuiDirection::Vertical)
        .split(area, &[RatatuiConstraint::Percentage(50), RatatuiConstraint::Percentage(50)]);
    assert_eq!(rows.len(), 2);
    let top_cells = Layout::new(RatatuiDirection::Horizontal)
        .split(rows[0], &[RatatuiConstraint::Percentage(50), RatatuiConstraint::Percentage(50)]);
    let bot_cells = Layout::new(RatatuiDirection::Horizontal)
        .split(rows[1], &[RatatuiConstraint::Percentage(50), RatatuiConstraint::Percentage(50)]);
    assert_eq!(top_cells.len(), 2);
    assert_eq!(bot_cells.len(), 2);
    // Total cells cover the parent.
    let total_w: u16 = top_cells.iter().map(|c| c.width).sum();
    assert_eq!(total_w, area.width);
    let total_h: u16 = rows.iter().map(|r| r.height).sum();
    assert_eq!(total_h, area.height);
}
