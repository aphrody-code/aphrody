//! Divider component.

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Divider {
    pub inset: bool,
}

impl Divider {
    pub fn new(inset: bool) -> Self {
        Self { inset }
    }

    /// Default full width in dp.
    pub const WIDTH_DP: f64 = 360.0;
}

impl mui_rs_renderer::pipeline::Widget for Divider {
    fn draw(&self, cx: &mut mui_rs_renderer::pipeline::DrawCx, transform: mui_rs_renderer::vello::kurbo::Affine) {
        use mui_rs_renderer::vello::kurbo::{Line, Stroke};
        use mui_rs_renderer::vello::peniko::Color;
        let x0 = if self.inset { 16.0 } else { 0.0 };
        let line = Line::new((x0, 0.0), (Self::WIDTH_DP, 0.0));
        // outline-variant, 1dp.
        cx.scene.stroke(&Stroke::new(1.0), transform, Color::from_rgb8(202, 196, 208), None, &line);
    }
}

#[cfg(target_arch = "wasm32")]
pub mod wasm {
    use wasm_bindgen::prelude::*;
    use web_sys::{Document, Element};

    use super::*;

    pub fn create_divider(doc: &Document, props: &Divider) -> Result<Element, JsValue> {
        let el = doc.create_element("md-divider")?;
        if props.inset {
            el.set_attribute("inset", "")?;
        }
        Ok(el)
    }
}
