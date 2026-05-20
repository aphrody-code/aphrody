//! Navigation: TopAppBar, NavigationBar, NavigationRail, NavigationDrawer.

use mui_rs_renderer::{
    pipeline::Widget,
    vello::{
        Scene,
        kurbo::{Affine, RoundedRect, Circle, BezPath},
        peniko::{Color, Fill},
    },
};

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
    fn draw(&self, scene: &mut Scene, transform: Affine) {
        let width = self.width;
        let height = Self::HEIGHT_DP;
        
        let rect = RoundedRect::new(0.0, 0.0, width, height, 0.0);
        let bg_color = Color::from_rgb8(28, 27, 31);
        scene.fill(Fill::NonZero, transform, bg_color, None, &rect);

        let logo_transform = transform * Affine::translate((16.0, 16.0));
        let logo_circle = Circle::new((16.0, 16.0), 14.0);
        scene.fill(Fill::NonZero, logo_transform, Color::from_rgb8(66, 133, 244), None, &logo_circle);
        
        let control_padding = 8.0;
        let button_size = 40.0;
        
        let close_transform = transform * Affine::translate((width - button_size - control_padding, 12.0));
        draw_close_icon(scene, close_transform);

        let max_transform = transform * Affine::translate((width - (button_size * 2.0) - control_padding, 12.0));
        draw_maximize_icon(scene, max_transform);

        let min_transform = transform * Affine::translate((width - (button_size * 3.0) - control_padding, 12.0));
        draw_minimize_icon(scene, min_transform);
    }
}

fn draw_close_icon(scene: &mut Scene, transform: Affine) {
    let mut path = BezPath::new();
    path.move_to((12.0, 12.0));
    path.line_to((28.0, 28.0));
    path.move_to((28.0, 12.0));
    path.line_to((12.0, 28.0));
    scene.stroke(&mui_rs_renderer::vello::kurbo::Stroke::new(2.0), transform, Color::WHITE, None, &path);
}

fn draw_maximize_icon(scene: &mut Scene, transform: Affine) {
    let rect = RoundedRect::new(12.0, 12.0, 28.0, 28.0, 2.0);
    scene.stroke(&mui_rs_renderer::vello::kurbo::Stroke::new(2.0), transform, Color::WHITE, None, &rect);
}

fn draw_minimize_icon(scene: &mut Scene, transform: Affine) {
    let mut path = BezPath::new();
    path.move_to((12.0, 24.0));
    path.line_to((28.0, 24.0));
    scene.stroke(&mui_rs_renderer::vello::kurbo::Stroke::new(2.0), transform, Color::WHITE, None, &path);
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
    fn draw(&self, scene: &mut Scene, transform: Affine) {
        let width = self.width;
        let height = Self::HEIGHT_DP;
        let rect = RoundedRect::new(0.0, 0.0, width, height, 0.0);
        let bg_color = Color::from_rgb8(33, 31, 38);
        scene.fill(Fill::NonZero, transform, bg_color, None, &rect);

        let item_width = width / 4.0;
        for i in 0..4 {
            let cx = (i as f64 * item_width) + (item_width / 2.0);
            let cy = height / 2.0;
            let color = if i == self.active_item {
                Color::from_rgb8(232, 222, 248)
            } else {
                Color::from_rgb8(147, 143, 153)
            };
            let circle = Circle::new((cx, cy), 16.0);
            scene.fill(Fill::NonZero, transform, color, None, &circle);
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
    fn draw(&self, scene: &mut Scene, transform: Affine) {
        let width = Self::WIDTH_DP;
        let height = self.height;
        let rect = RoundedRect::new(0.0, 0.0, width, height, 0.0);
        let bg_color = Color::from_rgb8(28, 27, 31);
        scene.fill(Fill::NonZero, transform, bg_color, None, &rect);

        for i in 0..4 {
            let cx = width / 2.0;
            let cy = 100.0 + (i as f64 * 80.0);
            let color = if i == self.active_item {
                Color::from_rgb8(232, 222, 248)
            } else {
                Color::from_rgb8(147, 143, 153)
            };
            let circle = Circle::new((cx, cy), 16.0);
            scene.fill(Fill::NonZero, transform, color, None, &circle);
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
    fn draw(&self, scene: &mut Scene, transform: Affine) {
        let width = Self::WIDTH_DP;
        let height = self.height;
        let rect = RoundedRect::new(0.0, 0.0, width, height, 0.0);
        let bg_color = Color::from_rgb8(28, 27, 31);
        scene.fill(Fill::NonZero, transform, bg_color, None, &rect);
    }
}
