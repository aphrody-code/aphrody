//! Navigation: TopAppBar, NavigationBar, NavigationRail, NavigationDrawer.

use mui_rs_renderer::pipeline::{DrawCx, Widget};
use mui_rs_renderer::vello::Scene;
use mui_rs_renderer::vello::kurbo::{Affine, BezPath, Circle, RoundedRect, Stroke};
use mui_rs_renderer::vello::peniko::{Color, Fill};
use mui_rs_renderer::TextStyle;

#[derive(Debug, Clone)]
pub struct TopAppBar {
    pub title: String,
    pub logo_id: String,
    pub width: f64,
}

impl TopAppBar {
    pub const HEIGHT_DP: f64 = 64.0;
}

impl Widget for TopAppBar {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let width = self.width;
        let height = Self::HEIGHT_DP;

        let rect = RoundedRect::new(0.0, 0.0, width, height, 0.0);
        let bg_color = Color::from_rgb8(28, 27, 31);
        cx.scene.fill(Fill::NonZero, transform, bg_color, None, &rect);

        let logo_transform = transform * Affine::translate((16.0, 16.0));
        let logo_circle = Circle::new((16.0, 16.0), 14.0);
        cx.scene.fill(Fill::NonZero, logo_transform, Color::from_rgb8(66, 133, 244), None, &logo_circle);

        // Title — real glyphs, vertically centred, left of the window controls.
        if !self.title.is_empty() {
            let style = TextStyle::new(
                mui_rs_renderer::text::FONT_UI,
                22.0,
                400.0,
                Color::from_rgb8(230, 225, 229), // on-surface
            );
            let (_tw, th) = cx.measure_text(&self.title, style);
            let ty = (height - f64::from(th)) / 2.0;
            cx.draw_text(&self.title, style, transform * Affine::translate((56.0, ty)));
        }

        let control_padding = 8.0;
        let button_size = 40.0;

        let close_transform = transform * Affine::translate((width - button_size - control_padding, 12.0));
        draw_close_icon(cx.scene, close_transform);

        let max_transform = transform * Affine::translate((width - (button_size * 2.0) - control_padding, 12.0));
        draw_maximize_icon(cx.scene, max_transform);

        let min_transform = transform * Affine::translate((width - (button_size * 3.0) - control_padding, 12.0));
        draw_minimize_icon(cx.scene, min_transform);
    }
}

fn draw_close_icon(scene: &mut Scene, transform: Affine) {
    let mut path = BezPath::new();
    path.move_to((12.0, 12.0));
    path.line_to((28.0, 28.0));
    path.move_to((28.0, 12.0));
    path.line_to((12.0, 28.0));
    scene.stroke(&Stroke::new(2.0), transform, Color::WHITE, None, &path);
}

fn draw_maximize_icon(scene: &mut Scene, transform: Affine) {
    let rect = RoundedRect::new(12.0, 12.0, 28.0, 28.0, 2.0);
    scene.stroke(&Stroke::new(2.0), transform, Color::WHITE, None, &rect);
}

fn draw_minimize_icon(scene: &mut Scene, transform: Affine) {
    let mut path = BezPath::new();
    path.move_to((12.0, 24.0));
    path.line_to((28.0, 24.0));
    scene.stroke(&Stroke::new(2.0), transform, Color::WHITE, None, &path);
}

#[derive(Debug, Clone)]
pub struct NavigationBar {
    pub active_item: usize,
    pub width: f64,
}

impl NavigationBar {
    pub const HEIGHT_DP: f64 = 80.0;
}

impl Widget for NavigationBar {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let width = self.width;
        let height = Self::HEIGHT_DP;
        let rect = RoundedRect::new(0.0, 0.0, width, height, 0.0);
        let bg_color = Color::from_rgb8(33, 31, 38);
        cx.scene.fill(Fill::NonZero, transform, bg_color, None, &rect);

        let item_width = width / 4.0;
        for i in 0..4 {
            let item_cx = (f64::from(i) * item_width) + (item_width / 2.0);
            let item_cy = height / 2.0;
            let color = if i as usize == self.active_item {
                Color::from_rgb8(232, 222, 248)
            } else {
                Color::from_rgb8(147, 143, 153)
            };
            let circle = Circle::new((item_cx, item_cy), 16.0);
            cx.scene.fill(Fill::NonZero, transform, color, None, &circle);
        }
    }
}

#[derive(Debug, Clone)]
pub struct NavigationRail {
    pub active_item: usize,
    pub height: f64,
}

impl NavigationRail {
    pub const WIDTH_DP: f64 = 80.0;
}

impl Widget for NavigationRail {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let width = Self::WIDTH_DP;
        let height = self.height;
        let rect = RoundedRect::new(0.0, 0.0, width, height, 0.0);
        let bg_color = Color::from_rgb8(28, 27, 31);
        cx.scene.fill(Fill::NonZero, transform, bg_color, None, &rect);

        for i in 0..4 {
            let item_cx = width / 2.0;
            let item_cy = 100.0 + (f64::from(i) * 80.0);
            let color = if i as usize == self.active_item {
                Color::from_rgb8(232, 222, 248)
            } else {
                Color::from_rgb8(147, 143, 153)
            };
            let circle = Circle::new((item_cx, item_cy), 16.0);
            cx.scene.fill(Fill::NonZero, transform, color, None, &circle);
        }
    }
}

#[derive(Debug, Clone)]
pub struct NavigationDrawer {
    pub open: bool,
    pub height: f64,
}

impl NavigationDrawer {
    pub const WIDTH_DP: f64 = 360.0;
}

impl Widget for NavigationDrawer {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let width = Self::WIDTH_DP;
        let height = self.height;
        let rect = RoundedRect::new(0.0, 0.0, width, height, 0.0);
        let bg_color = Color::from_rgb8(28, 27, 31);
        cx.scene.fill(Fill::NonZero, transform, bg_color, None, &rect);
    }
}
