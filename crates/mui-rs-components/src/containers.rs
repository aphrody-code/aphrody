//! Containers: Card, Dialog, BottomSheet, SideSheet, Carousel, TopAppBar, BottomAppBar, Tooltip.

use m3_tokens::{
    shape::{CornerRadius, MEDIUM},
    state::{DRAGGED, FOCUS, HOVER, PRESSED, StateLayer},
};

use crate::actions::InteractionState;

/// M3 Card variants
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CardVariant {
    Elevated,
    Filled,
    Outlined,
}

#[derive(Debug, Clone)]
pub struct Card {
    pub variant: CardVariant,
    pub interactive: bool,
    pub disabled: bool,
}

impl Card {
    /// M3 standard cards use Medium corner radius (12dp)
    pub fn shape(&self) -> CornerRadius {
        MEDIUM
    }

    /// Resolves the elevation level (0-5) based on variant and interaction state.
    pub fn elevation_level(&self, state: InteractionState) -> usize {
        if self.disabled {
            return 0; // Disabled cards have no elevation
        }

        match self.variant {
            CardVariant::Elevated => match state {
                InteractionState::Resting => 1,
                InteractionState::Hovered => 2,
                InteractionState::Focused => 1,
                InteractionState::Pressed => 1,
                InteractionState::Dragged => 4,
            },
            CardVariant::Filled => match state {
                InteractionState::Resting => 0,
                InteractionState::Hovered => 1,
                InteractionState::Focused => 0,
                InteractionState::Pressed => 0,
                InteractionState::Dragged => 3,
            },
            CardVariant::Outlined => match state {
                InteractionState::Resting => 0,
                InteractionState::Hovered => 1,
                InteractionState::Focused => 0,
                InteractionState::Pressed => 0,
                InteractionState::Dragged => 3,
            },
        }
    }

    /// Returns the state layer overlay (opacity) applied over the card background.
    pub fn state_layer(&self, state: InteractionState) -> Option<StateLayer> {
        if !self.interactive || self.disabled {
            return None;
        }
        match state {
            InteractionState::Resting => None,
            InteractionState::Hovered => Some(HOVER),
            InteractionState::Focused => Some(FOCUS),
            InteractionState::Pressed => Some(PRESSED),
            InteractionState::Dragged => Some(DRAGGED),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BottomSheet {
    pub open: bool,
}

#[derive(Debug, Clone)]
pub struct TopAppBar {
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct BottomAppBar {
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct Dialog {
    pub open: bool,
    pub title: String,
}

use mui_rs_renderer::pipeline::{DrawCx, Widget};
use mui_rs_renderer::shadow;
use mui_rs_renderer::vello::kurbo::{Affine, Rect, RoundedRect, Stroke};
use mui_rs_renderer::vello::peniko::{Color, Fill};
use mui_rs_renderer::TextStyle;

const FAMILY: &str = mui_rs_renderer::text::FONT_UI;

#[derive(Debug, Clone)]
pub struct Tooltip {
    pub text: String,
}

impl Card {
    /// Default M3 card footprint in dp (medium component).
    pub const WIDTH_DP: f64 = 200.0;
    pub const HEIGHT_DP: f64 = 150.0;
    const RADIUS_DP: f64 = 12.0; // Medium corner radius
}

impl Widget for Card {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let (w, h, r) = (Card::WIDTH_DP, Card::HEIGHT_DP, Card::RADIUS_DP);
        let rect = RoundedRect::new(0.0, 0.0, w, h, r);

        // Real M3 elevation shadow (behind the surface), per variant resting level.
        let level = self.elevation_level(InteractionState::Resting);
        shadow::draw_elevation(cx.scene, transform, Rect::new(0.0, 0.0, w, h), r, level);

        let base_color = match self.variant {
            CardVariant::Elevated => Color::from_rgb8(243, 237, 247), // Surface container low
            CardVariant::Filled => Color::from_rgb8(232, 222, 248),   // Surface container highest
            CardVariant::Outlined => Color::from_rgb8(254, 247, 255), // Surface
        };
        cx.scene.fill(Fill::NonZero, transform, base_color, None, &rect);

        // Outlined cards carry a real 1dp outline stroke (no shadow).
        if self.variant == CardVariant::Outlined {
            let outline = if self.disabled {
                Color::from_rgba8(28, 27, 31, 30)
            } else {
                Color::from_rgb8(121, 116, 126) // Outline
            };
            cx.scene.stroke(&Stroke::new(1.0), transform, outline, None, &rect);
        }
    }
}

impl BottomSheet {
    pub const WIDTH_DP: f64 = 360.0;
    pub const HEIGHT_DP: f64 = 220.0;
}

impl Widget for BottomSheet {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        if !self.open {
            return;
        }
        let (w, h) = (BottomSheet::WIDTH_DP, BottomSheet::HEIGHT_DP);
        // Top-rounded surface-container-low sheet with a drag handle.
        let rect = RoundedRect::new(0.0, 0.0, w, h, 28.0);
        shadow::draw_elevation(cx.scene, transform, Rect::new(0.0, 0.0, w, h), 28.0, 1);
        cx.scene.fill(Fill::NonZero, transform, Color::from_rgb8(247, 242, 250), None, &rect);
        let handle = RoundedRect::new(w / 2.0 - 16.0, 12.0, w / 2.0 + 16.0, 16.0, 2.0);
        cx.scene.fill(Fill::NonZero, transform, Color::from_rgba8(73, 69, 79, 102), None, &handle); // on-surface-variant 40%
    }
}

impl BottomAppBar {
    pub const WIDTH_DP: f64 = 412.0;
    pub const HEIGHT_DP: f64 = 80.0;
}

impl Widget for BottomAppBar {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let (w, h) = (BottomAppBar::WIDTH_DP, BottomAppBar::HEIGHT_DP);
        let rect = RoundedRect::new(0.0, 0.0, w, h, 0.0);
        cx.scene.fill(Fill::NonZero, transform, Color::from_rgb8(247, 242, 250), None, &rect); // surface-container
        let style = TextStyle::new(FAMILY, 14.0, 400.0, Color::from_rgb8(73, 69, 79));
        let (_tw, th) = cx.measure_text(&self.content, style);
        cx.draw_text(&self.content, style, transform * Affine::translate((16.0, (h - f64::from(th)) / 2.0)));
    }
}

impl Dialog {
    pub const WIDTH_DP: f64 = 312.0;
    pub const HEIGHT_DP: f64 = 180.0;
}

impl Widget for Dialog {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        if !self.open {
            return;
        }
        let (w, h) = (Dialog::WIDTH_DP, Dialog::HEIGHT_DP);
        let rect = RoundedRect::new(0.0, 0.0, w, h, 28.0);
        shadow::draw_elevation(cx.scene, transform, Rect::new(0.0, 0.0, w, h), 28.0, 3);
        cx.scene.fill(Fill::NonZero, transform, Color::from_rgb8(236, 230, 240), None, &rect); // surface-container-high
        let title_style = TextStyle::new(FAMILY, 24.0, 400.0, Color::from_rgb8(28, 27, 31)); // headline-small / on-surface
        cx.draw_text(&self.title, title_style, transform * Affine::translate((24.0, 24.0)));
    }
}

impl Tooltip {
    const PAD_X: f64 = 8.0;
    pub const HEIGHT_DP: f64 = 24.0;
}

impl Widget for Tooltip {
    fn draw(&self, cx: &mut DrawCx, transform: Affine) {
        let h = Tooltip::HEIGHT_DP;
        let style = TextStyle::new(FAMILY, 12.0, 400.0, Color::from_rgb8(245, 239, 247)); // inverse-on-surface
        let (tw, th) = cx.measure_text(&self.text, style);
        let w = f64::from(tw) + Tooltip::PAD_X * 2.0;
        let rect = RoundedRect::new(0.0, 0.0, w, h, 4.0);
        cx.scene.fill(Fill::NonZero, transform, Color::from_rgb8(50, 47, 53), None, &rect); // inverse-surface
        cx.draw_text(&self.text, style, transform * Affine::translate((Tooltip::PAD_X, (h - f64::from(th)) / 2.0)));
    }
}
