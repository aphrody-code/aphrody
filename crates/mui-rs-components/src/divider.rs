//! Divider component.

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Divider {
    pub inset: bool,
}

impl Divider {
    pub fn new(inset: bool) -> Self {
        Self { inset }
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
