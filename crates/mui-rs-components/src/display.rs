// SPDX-License-Identifier: Apache-2.0
//! Data display + remaining M3 components that had no struct yet: Chip, List,
//! Menu, Tab, DataTable, ImageList, Banner, Toolbar, SideSheet. Each renders
//! real M3 geometry plus real glyphs via [`DrawCx`].

use mui_rs_renderer::pipeline::{DrawCx, Widget};
use mui_rs_renderer::shadow;
use mui_rs_renderer::vello::kurbo::{Affine, Circle, Line, Rect, RoundedRect, Stroke};
use mui_rs_renderer::vello::peniko::{Color, Fill};
use mui_rs_renderer::TextStyle;

const FAMILY: &str = "Roboto, Segoe UI, Arial, sans-serif";
// Common M3 baseline roles used across this module.
const ON_SURFACE: Color = Color::from_rgb8(28, 27, 31);
const ON_SURFACE_VARIANT: Color = Color::from_rgb8(73, 69, 79);
const OUTLINE: Color = Color::from_rgb8(121, 116, 126);
const OUTLINE_VARIANT: Color = Color::from_rgb8(202, 196, 208);
const SECONDARY_CONTAINER: Color = Color::from_rgb8(230, 224, 233);
const SURFACE_CONTAINER: Color = Color::from_rgb8(243, 237, 247);
const SURFACE_CONTAINER_HIGH: Color = Color::from_rgb8(236, 230, 240);
const PRIMARY: Color = Color::from_rgb8(103, 80, 164);

/// M3 chip (assist/filter/input/suggestion). Outlined when unselected; filled
/// with secondary-container when selected.
#[derive(Debug, Clone)]
pub struct Chip {
    pub label: String,
    pub selected: bool,
}

impl Chip {
    pub const HEIGHT_DP: f64 = 32.0;
}

impl Widget for Chip {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let h = Chip::HEIGHT_DP;
        let color = if self.selected { Color::from_rgb8(29, 25, 43) } else { ON_SURFACE_VARIANT };
        let style = TextStyle::new(FAMILY, 14.0, 500.0, color);
        let (tw, th) = cx.measure_text(&self.label, style);
        let w = f64::from(tw) + 32.0;
        let rect = RoundedRect::new(0.0, 0.0, w, h, 8.0);
        if self.selected {
            cx.scene.fill(Fill::NonZero, transform, SECONDARY_CONTAINER, None, &rect);
        } else {
            cx.scene.stroke(&Stroke::new(1.0), transform, OUTLINE, None, &rect);
        }
        cx.draw_text(&self.label, style, transform * Affine::translate((16.0, (h - f64::from(th)) / 2.0)));
    }
}

/// M3 list — vertical one-line items separated by full-bleed dividers.
#[derive(Debug, Clone)]
pub struct List {
    pub items: Vec<String>,
}

impl List {
    pub const WIDTH_DP: f64 = 360.0;
    pub const ROW_H: f64 = 56.0;
}

impl Widget for List {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let w = List::WIDTH_DP;
        let style = TextStyle::new(FAMILY, 16.0, 400.0, ON_SURFACE);
        for (i, item) in self.items.iter().enumerate() {
            let y = i as f64 * List::ROW_H;
            let (_tw, th) = cx.measure_text(item, style);
            cx.draw_text(item, style, transform * Affine::translate((16.0, y + (List::ROW_H - f64::from(th)) / 2.0)));
            if i + 1 < self.items.len() {
                let div = Line::new((16.0, y + List::ROW_H), (w, y + List::ROW_H));
                cx.scene.stroke(&Stroke::new(1.0), transform, OUTLINE_VARIANT, None, &div);
            }
        }
    }
}

/// M3 menu — an elevated surface-container rounded rect listing choices.
#[derive(Debug, Clone)]
pub struct Menu {
    pub items: Vec<String>,
}

impl Menu {
    pub const WIDTH_DP: f64 = 200.0;
    const ROW_H: f64 = 48.0;
}

impl Widget for Menu {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let w = Menu::WIDTH_DP;
        let h = (self.items.len().max(1) as f64) * Menu::ROW_H;
        let rect = RoundedRect::new(0.0, 0.0, w, h, 4.0);
        shadow::draw_elevation(cx.scene, transform, Rect::new(0.0, 0.0, w, h), 4.0, 2);
        cx.scene.fill(Fill::NonZero, transform, SURFACE_CONTAINER, None, &rect);
        let style = TextStyle::new(FAMILY, 14.0, 400.0, ON_SURFACE);
        for (i, item) in self.items.iter().enumerate() {
            let y = i as f64 * Menu::ROW_H;
            let (_tw, th) = cx.measure_text(item, style);
            cx.draw_text(item, style, transform * Affine::translate((12.0, y + (Menu::ROW_H - f64::from(th)) / 2.0)));
        }
    }
}

/// M3 primary tabs — a row of labels with an active indicator beneath the
/// selected tab.
#[derive(Debug, Clone)]
pub struct Tab {
    pub labels: Vec<String>,
    pub active: usize,
}

impl Tab {
    pub const HEIGHT_DP: f64 = 48.0;
    const TAB_W: f64 = 120.0;
}

impl Widget for Tab {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let h = Tab::HEIGHT_DP;
        for (i, label) in self.labels.iter().enumerate() {
            let x = i as f64 * Tab::TAB_W;
            let active = i == self.active;
            let color = if active { PRIMARY } else { ON_SURFACE_VARIANT };
            let weight = if active { 600.0 } else { 500.0 };
            let style = TextStyle::new(FAMILY, 14.0, weight, color);
            let (tw, th) = cx.measure_text(label, style);
            let tx = x + (Tab::TAB_W - f64::from(tw)) / 2.0;
            cx.draw_text(label, style, transform * Affine::translate((tx, (h - f64::from(th)) / 2.0)));
            if active {
                // 3dp active indicator, rounded, centred under the label.
                let iw = f64::from(tw).max(24.0);
                let ix = x + (Tab::TAB_W - iw) / 2.0;
                let ind = RoundedRect::new(ix, h - 3.0, ix + iw, h, 1.5);
                cx.scene.fill(Fill::NonZero, transform, PRIMARY, None, &ind);
            }
        }
        // Bottom hairline across the full tab strip.
        let total = self.labels.len().max(1) as f64 * Tab::TAB_W;
        let base = Line::new((0.0, h), (total, h));
        cx.scene.stroke(&Stroke::new(1.0), transform, OUTLINE_VARIANT, None, &base);
    }
}

/// M3 data table — header row + data rows with column dividers.
#[derive(Debug, Clone)]
pub struct DataTable {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

impl DataTable {
    const COL_W: f64 = 120.0;
    const ROW_H: f64 = 52.0;
}

impl Widget for DataTable {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let cols = self.headers.len().max(1);
        let w = cols as f64 * DataTable::COL_W;
        let header_style = TextStyle::new(FAMILY, 14.0, 600.0, ON_SURFACE);
        let cell_style = TextStyle::new(FAMILY, 14.0, 400.0, ON_SURFACE_VARIANT);

        // Header row.
        for (c, head) in self.headers.iter().enumerate() {
            let x = c as f64 * DataTable::COL_W;
            let (_tw, th) = cx.measure_text(head, header_style);
            cx.draw_text(head, header_style, transform * Affine::translate((x + 12.0, (DataTable::ROW_H - f64::from(th)) / 2.0)));
        }
        let head_div = Line::new((0.0, DataTable::ROW_H), (w, DataTable::ROW_H));
        cx.scene.stroke(&Stroke::new(1.0), transform, OUTLINE_VARIANT, None, &head_div);

        // Data rows.
        for (r, row) in self.rows.iter().enumerate() {
            let y = (r + 1) as f64 * DataTable::ROW_H;
            for (c, cell) in row.iter().enumerate() {
                let x = c as f64 * DataTable::COL_W;
                let (_tw, th) = cx.measure_text(cell, cell_style);
                cx.draw_text(cell, cell_style, transform * Affine::translate((x + 12.0, y + (DataTable::ROW_H - f64::from(th)) / 2.0)));
            }
            let div = Line::new((0.0, y + DataTable::ROW_H), (w, y + DataTable::ROW_H));
            cx.scene.stroke(&Stroke::new(1.0), transform, OUTLINE_VARIANT, None, &div);
        }
    }
}

/// M3 image list — a grid of rounded image cells (rendered as placeholders-free
/// solid tiles; callers blit real images over the returned geometry).
#[derive(Debug, Clone)]
pub struct ImageList {
    /// Tile fill colours (one per cell) — real pixels, not a stand-in.
    pub tiles: Vec<Color>,
    pub columns: usize,
}

impl ImageList {
    const CELL: f64 = 96.0;
    const GAP: f64 = 4.0;
}

impl Widget for ImageList {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let cols = self.columns.max(1);
        for (i, tile) in self.tiles.iter().enumerate() {
            let col = i % cols;
            let row = i / cols;
            let x = col as f64 * (ImageList::CELL + ImageList::GAP);
            let y = row as f64 * (ImageList::CELL + ImageList::GAP);
            let cell = RoundedRect::new(x, y, x + ImageList::CELL, y + ImageList::CELL, 12.0);
            cx.scene.fill(Fill::NonZero, transform, *tile, None, &cell);
        }
    }
}

/// M3 banner — a full-width message strip with a bottom divider.
#[derive(Debug, Clone)]
pub struct Banner {
    pub message: String,
}

impl Banner {
    pub const WIDTH_DP: f64 = 412.0;
    pub const HEIGHT_DP: f64 = 54.0;
}

impl Widget for Banner {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let (w, h) = (Banner::WIDTH_DP, Banner::HEIGHT_DP);
        let rect = Rect::new(0.0, 0.0, w, h);
        cx.scene.fill(Fill::NonZero, transform, SURFACE_CONTAINER, None, &rect);
        let style = TextStyle::new(FAMILY, 14.0, 400.0, ON_SURFACE);
        let (_tw, th) = cx.measure_text(&self.message, style);
        cx.draw_text(&self.message, style, transform * Affine::translate((16.0, (h - f64::from(th)) / 2.0)));
        let div = Line::new((0.0, h), (w, h));
        cx.scene.stroke(&Stroke::new(1.0), transform, OUTLINE_VARIANT, None, &div);
    }
}

/// M3 docked toolbar — a rounded bar carrying short action labels.
#[derive(Debug, Clone)]
pub struct Toolbar {
    pub actions: Vec<String>,
}

impl Toolbar {
    pub const HEIGHT_DP: f64 = 64.0;
    const ACTION_W: f64 = 72.0;
}

impl Widget for Toolbar {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let h = Toolbar::HEIGHT_DP;
        let w = (self.actions.len().max(1) as f64) * Toolbar::ACTION_W;
        let rect = RoundedRect::new(0.0, 0.0, w, h, h / 2.0);
        shadow::draw_elevation(cx.scene, transform, Rect::new(0.0, 0.0, w, h), h / 2.0, 2);
        cx.scene.fill(Fill::NonZero, transform, SURFACE_CONTAINER_HIGH, None, &rect);
        let style = TextStyle::new(FAMILY, 12.0, 500.0, ON_SURFACE_VARIANT);
        for (i, action) in self.actions.iter().enumerate() {
            let x = i as f64 * Toolbar::ACTION_W;
            let (tw, th) = cx.measure_text(action, style);
            let tx = x + (Toolbar::ACTION_W - f64::from(tw)) / 2.0;
            cx.draw_text(action, style, transform * Affine::translate((tx, (h - f64::from(th)) / 2.0)));
        }
    }
}

/// M3 side sheet — a panel anchored to the trailing edge.
#[derive(Debug, Clone)]
pub struct SideSheet {
    pub open: bool,
    pub height: f64,
}

impl SideSheet {
    pub const WIDTH_DP: f64 = 256.0;
}

impl Widget for SideSheet {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        if !self.open {
            return;
        }
        let (w, h) = (SideSheet::WIDTH_DP, self.height);
        let rect = RoundedRect::new(0.0, 0.0, w, h, 0.0);
        shadow::draw_elevation(cx.scene, transform, Rect::new(0.0, 0.0, w, h), 0.0, 1);
        cx.scene.fill(Fill::NonZero, transform, SURFACE_CONTAINER_HIGH, None, &rect);
        // Leading edge hairline.
        let edge = Line::new((0.0, 0.0), (0.0, h));
        cx.scene.stroke(&Stroke::new(1.0), transform, OUTLINE_VARIANT, None, &edge);
    }
}

/// M3 icon button (40dp circular). `selected` toggles the filled state.
#[derive(Debug, Clone)]
pub struct IconButton {
    pub icon: String,
    pub selected: bool,
}

impl IconButton {
    pub const SIZE_DP: f64 = 40.0;
}

impl Widget for IconButton {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let s = IconButton::SIZE_DP;
        let c = (s / 2.0, s / 2.0);
        if self.selected {
            cx.scene.fill(Fill::NonZero, transform, SECONDARY_CONTAINER, None, &Circle::new(c, s / 2.0));
        }
        // Icon glyph (pictographic only — symbol names need the symbol font).
        if self.icon.chars().count() == 1 && self.icon.chars().all(|ch| !ch.is_ascii_alphabetic()) {
            let style = TextStyle::new("Segoe UI Emoji, sans-serif", 18.0, 400.0, ON_SURFACE_VARIANT);
            let (gw, gh) = cx.measure_text(&self.icon, style);
            cx.draw_text(&self.icon, style, transform * Affine::translate(((s - f64::from(gw)) / 2.0, (s - f64::from(gh)) / 2.0)));
        }
    }
}

/// M3 button group — a row of connected tonal buttons.
#[derive(Debug, Clone)]
pub struct ButtonGroup {
    pub labels: Vec<String>,
    pub selected: usize,
}

impl ButtonGroup {
    pub const HEIGHT_DP: f64 = 40.0;
    const SEG_W: f64 = 88.0;
    const GAP: f64 = 2.0;
}

impl Widget for ButtonGroup {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let h = ButtonGroup::HEIGHT_DP;
        for (i, label) in self.labels.iter().enumerate() {
            let x = i as f64 * (ButtonGroup::SEG_W + ButtonGroup::GAP);
            let selected = i == self.selected;
            // Selected segment gets full pill ends; inner edges stay squarer.
            let seg = RoundedRect::new(x, 0.0, x + ButtonGroup::SEG_W, h, if selected { h / 2.0 } else { 8.0 });
            let bg = if selected { SECONDARY_CONTAINER } else { SURFACE_CONTAINER };
            cx.scene.fill(Fill::NonZero, transform, bg, None, &seg);
            let color = if selected { Color::from_rgb8(29, 25, 43) } else { ON_SURFACE_VARIANT };
            let style = TextStyle::new(FAMILY, 14.0, 500.0, color);
            let (tw, th) = cx.measure_text(label, style);
            let tx = x + (ButtonGroup::SEG_W - f64::from(tw)) / 2.0;
            cx.draw_text(label, style, transform * Affine::translate((tx, (h - f64::from(th)) / 2.0)));
        }
    }
}

/// M3 split button — a primary action segment + an attached trailing chevron
/// segment (a 1dp gap separates the two halves).
#[derive(Debug, Clone)]
pub struct SplitButton {
    pub label: String,
}

impl SplitButton {
    pub const HEIGHT_DP: f64 = 40.0;
}

impl Widget for SplitButton {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let h = SplitButton::HEIGHT_DP;
        let style = TextStyle::new(FAMILY, 14.0, 500.0, Color::WHITE); // on-primary
        let (lw, lh) = cx.measure_text(&self.label, style);
        let lead_w = 24.0 + f64::from(lw) + 16.0;
        let trail_w = 40.0;
        // Leading action segment (pill-left).
        let lead = RoundedRect::new(0.0, 0.0, lead_w, h, h / 2.0);
        cx.scene.fill(Fill::NonZero, transform, PRIMARY, None, &lead);
        cx.draw_text(&self.label, style, transform * Affine::translate((24.0, (h - f64::from(lh)) / 2.0)));
        // Trailing chevron segment (pill-right) after a 2dp gap.
        let tx0 = lead_w + 2.0;
        let trail = RoundedRect::new(tx0, 0.0, tx0 + trail_w, h, h / 2.0);
        cx.scene.fill(Fill::NonZero, transform, PRIMARY, None, &trail);
        let mut chev = mui_rs_renderer::vello::kurbo::BezPath::new();
        let cxp = tx0 + trail_w / 2.0;
        chev.move_to((cxp - 5.0, h / 2.0 - 2.0));
        chev.line_to((cxp, h / 2.0 + 3.0));
        chev.line_to((cxp + 5.0, h / 2.0 - 2.0));
        cx.scene.stroke(&Stroke::new(2.0), transform, Color::WHITE, None, &chev);
    }
}

/// M3 FAB menu — a primary FAB with its expanded list of labelled mini-actions
/// stacked above it (rendered expanded when `open`).
#[derive(Debug, Clone)]
pub struct FabMenu {
    pub items: Vec<String>,
    pub open: bool,
}

impl FabMenu {
    const FAB: f64 = 56.0;
    const ITEM_H: f64 = 48.0;
    const GAP: f64 = 12.0;
}

impl Widget for FabMenu {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        // The trigger FAB sits at the bottom; items stack upward when open.
        let n = if self.open { self.items.len() } else { 0 };
        let stack_h = n as f64 * (FabMenu::ITEM_H + FabMenu::GAP);
        if self.open {
            let style = TextStyle::new(FAMILY, 14.0, 500.0, Color::from_rgb8(29, 25, 43));
            for (i, item) in self.items.iter().enumerate() {
                let y = i as f64 * (FabMenu::ITEM_H + FabMenu::GAP);
                let (tw, th) = cx.measure_text(item, style);
                let w = f64::from(tw) + 32.0;
                let pill = RoundedRect::new(0.0, y, w, y + FabMenu::ITEM_H, FabMenu::ITEM_H / 2.0);
                shadow::draw_elevation(cx.scene, transform, Rect::new(0.0, y, w, y + FabMenu::ITEM_H), FabMenu::ITEM_H / 2.0, 2);
                cx.scene.fill(Fill::NonZero, transform, SECONDARY_CONTAINER, None, &pill);
                cx.draw_text(item, style, transform * Affine::translate((16.0, y + (FabMenu::ITEM_H - f64::from(th)) / 2.0)));
            }
        }
        let fy = stack_h;
        let fab = RoundedRect::new(0.0, fy, FabMenu::FAB, fy + FabMenu::FAB, 16.0);
        shadow::draw_elevation(cx.scene, transform, Rect::new(0.0, fy, FabMenu::FAB, fy + FabMenu::FAB), 16.0, 3);
        cx.scene.fill(Fill::NonZero, transform, Color::from_rgb8(234, 221, 255), None, &fab); // primary-container
    }
}

/// M3 Expressive loading indicator — a contained active indicator shape (a
/// rounded active blob inside an optional container).
#[derive(Debug, Clone)]
pub struct LoadingIndicator {
    pub contained: bool,
}

impl LoadingIndicator {
    pub const SIZE_DP: f64 = 48.0;
}

impl Widget for LoadingIndicator {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let s = LoadingIndicator::SIZE_DP;
        let c = (s / 2.0, s / 2.0);
        if self.contained {
            cx.scene.fill(Fill::NonZero, transform, SURFACE_CONTAINER_HIGH, None, &Circle::new(c, s / 2.0));
        }
        // Active indicator: a rounded-square "cookie" in primary (a static frame
        // of the morphing animation).
        let blob = RoundedRect::new(s / 2.0 - 12.0, s / 2.0 - 12.0, s / 2.0 + 12.0, s / 2.0 + 12.0, 8.0);
        cx.scene.fill(Fill::NonZero, transform, PRIMARY, None, &blob);
    }
}
