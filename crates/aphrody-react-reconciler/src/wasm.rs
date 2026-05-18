// SPDX-License-Identifier: Apache-2.0
//
// wasm-bindgen bindings — expose a thin subset of the reconciler surface
// to a JS host (browser, Claude Code MCP runtime). We only forward the
// primitives needed to mount, patch, and snapshot a tree; the React-fiber
// scheduler is still TS-side (re-uses the existing `packages/aphrody-jsx`
// thin wrapper around `react-reconciler` until that crate ships).

//! WASM exports — only compiled on `wasm32-unknown-unknown`.

#![cfg(target_arch = "wasm32")]

use alloc::string::{String, ToString};
use serde_json::Map;
use wasm_bindgen::prelude::*;

use crate::element::ElementKind;
use crate::osc::{encode_frame, OscFrame, VecSink};
use crate::reconciler::Reconciler as RustReconciler;

/// Thin JS-friendly wrapper around `Reconciler`. JSON props are accepted as
/// `String` (the JS caller passes `JSON.stringify(props)`).
#[wasm_bindgen]
pub struct AphrodyReconciler {
    inner: RustReconciler,
}

#[wasm_bindgen]
impl AphrodyReconciler {
    /// Constructs a new reconciler with a fresh root.
    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: RustReconciler::new(),
        }
    }

    /// Returns the root id.
    #[wasm_bindgen(getter, js_name = rootId)]
    #[must_use]
    pub fn root_id(&self) -> String {
        self.inner.root_id().to_string()
    }

    /// Appends an element by kind name + JSON-serialized props.
    /// Returns the new child node id.
    #[wasm_bindgen(js_name = appendElement)]
    pub fn append_element(&self, kind: &str, props_json: &str) -> Result<String, JsValue> {
        let kind = ElementKind::from_str(kind)
            .map_err(|e| JsValue::from_str(&e))?;
        let props: Map<String, serde_json::Value> = if props_json.is_empty() {
            Map::new()
        } else {
            serde_json::from_str(props_json)
                .map_err(|e| JsValue::from_str(&e.to_string()))?
        };
        let node = self.inner.append_element(kind, props);
        Ok(node.id().to_string())
    }

    /// Appends a text literal. Returns the new child node id.
    #[wasm_bindgen(js_name = appendText)]
    pub fn append_text(&self, text: &str) -> String {
        let node = self.inner.append_text(text);
        node.id().to_string()
    }

    /// Drives a commit and returns the emitted frames (joined as a single
    /// OSC byte stream).
    #[wasm_bindgen]
    pub fn commit(&self) -> String {
        let mut sink = VecSink::new();
        self.inner.commit(&mut sink);
        sink.joined()
    }

    /// Returns a JSON snapshot of the current root tree.
    #[wasm_bindgen]
    pub fn snapshot(&self) -> String {
        let json = self.inner.snapshot();
        serde_json::to_string(&json).unwrap_or_default()
    }
}

impl Default for AphrodyReconciler {
    fn default() -> Self {
        Self::new()
    }
}

/// Stateless helper: encode an `unmount` frame so the JS host can ship it
/// without round-tripping through a full reconciler instance.
#[wasm_bindgen(js_name = encodeUnmountFrame)]
#[must_use]
pub fn encode_unmount_frame(root_id: &str) -> String {
    encode_frame(&OscFrame::Unmount(crate::osc::UnmountFields {
        id: root_id.to_string(),
    }))
}
