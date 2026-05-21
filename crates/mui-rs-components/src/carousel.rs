//! Carousel component.

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Carousel {
    pub items: Vec<String>,
}

impl Carousel {
    pub fn new(items: Vec<String>) -> Self {
        Self { items }
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
