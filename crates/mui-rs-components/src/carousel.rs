//! Carousel component.

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Carousel {
    pub items: Vec<String>,
}

impl Carousel {
    pub fn new(items: Vec<String>) -> Self {
        Self { items }
    }

    pub const HEIGHT_DP: f64 = 120.0;
    const ITEM_W: f64 = 120.0;
    const GAP: f64 = 8.0;
}

impl mui_rs_renderer::pipeline::Widget for Carousel {
    fn draw(&self, cx: &mut mui_rs_renderer::pipeline::DrawCx, transform: mui_rs_renderer::vello::kurbo::Affine) {
        use mui_rs_renderer::vello::kurbo::{Affine, RoundedRect};
        use mui_rs_renderer::vello::peniko::{Color, Fill};
        use mui_rs_renderer::TextStyle;
        let h = Self::HEIGHT_DP;
        // Hero carousel: a row of rounded thumbnails, each labelled.
        let style = TextStyle::new("Roboto, Segoe UI, Arial, sans-serif", 14.0, 500.0, Color::WHITE);
        for (i, item) in self.items.iter().enumerate() {
            let x = i as f64 * (Self::ITEM_W + Self::GAP);
            let cell = RoundedRect::new(x, 0.0, x + Self::ITEM_W, h, 16.0);
            // surface-container-high tile.
            cx.scene.fill(Fill::NonZero, transform, Color::from_rgb8(73, 69, 79), None, &cell);
            let (_tw, th) = cx.measure_text(item, style);
            cx.draw_text(item, style, transform * Affine::translate((x + 12.0, h - f64::from(th) - 12.0)));
        }
    }
}

#[cfg(target_arch = "wasm32")]
pub mod wasm {
    use wasm_bindgen::prelude::*;
    use web_sys::{Document, Element};

    use super::*;

    pub fn create_carousel(doc: &Document, props: &Carousel) -> Result<Element, JsValue> {
        // MWC doesn't have an official md-carousel yet, so we use a custom or third-party wrapper.
        let el = doc.create_element("md-carousel")?;
        for item in &props.items {
            let child = doc.create_element("md-carousel-item")?;
            child.set_text_content(Some(item));
            el.append_child(&child)?;
        }
        Ok(el)
    }
}
