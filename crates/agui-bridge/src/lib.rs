// SPDX-License-Identifier: Apache-2.0
//! `agui-bridge` — Rust bindings for the AG-UI (Agent UI) protocol.
//!
//! Ports the wire format defined in
//! `packages/agui-adapter/src/types.ts` and provides:
//!
//! 1. [`AguiNode`] — a declarative UI-tree enum covering every element kind an agent can emit.
//! 2. [`parse`] — deserialise the agui wire format (`WireNode` JSON) into an `AguiNode` tree.
//! 3. [`render`] (wasm32-gated) — mount an `AguiNode` tree into a real browser DOM container by
//!    calling into `web_sys`; buttons are delegated to `mui-rs-components::button`.
//!
//! ## Mapping table (wire tag → custom element / DOM element)
//!
//! | wire `"type"`  | DOM element produced                                 |
//! |----------------|------------------------------------------------------|
//! | `"text"`       | `<span class="agui-text">`                           |
//! | `"container"`  | `<div class="agui-container">` + children            |
//! | `"button"`     | via `mui_rs_components::button` → `<md-*-button>`        |
//! | `"input"`      | `<md-outlined-text-field>` (MWC3)                    |
//! | `"toolCall"`   | `<div class="agui-tool-call">` card                  |
//! | `"markdown"`   | `<div class="agui-markdown">` (raw text; host styles)|
//! | `"image"`      | `<img class="agui-image">`                           |
//! | `"code"`       | `<pre class="agui-code"><code>` with lang class      |

#![forbid(unsafe_code)]

#[cfg(target_arch = "wasm32")] use wasm_bindgen::prelude::*;
#[cfg(target_arch = "wasm32")] use web_sys::{Document, Element, HtmlElement};

pub mod wire;

// Re-export props types so consumers only need to import from `agui_bridge`.
pub use mui_rs_components::actions::{Button as ButtonProps, ButtonVariant};
use wire::{WireCode, WireImage, WireInput, WireNode, WireToolCall};

// ---------------------------------------------------------------------------
// AguiNode — the parsed, host-native representation
// ---------------------------------------------------------------------------

/// Props for a text input element.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct InputProps {
    pub label: String,
    pub value: String,
    pub placeholder: String,
    pub disabled: bool,
    pub on_input_id: Option<String>,
}

/// Props for a tool-call progress / result card.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolCallProps {
    pub tool_name: String,
    pub args: serde_json::Value,
    pub status: Option<wire::ToolCallStatus>,
    pub result: Option<serde_json::Value>,
}

/// Props for an image element.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ImageProps {
    pub src: String,
    pub alt: String,
    pub width_px: Option<u32>,
    pub height_px: Option<u32>,
}

/// Props for a syntax-highlighted code block.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct CodeProps {
    pub code: String,
    pub lang: Option<String>,
}

/// Declarative UI tree that the agui renderer materialises into the DOM.
///
/// Each variant corresponds to one `<a-gui-*>` custom element kind in the
/// agui-adapter spec.  The `Container` variant carries owned children so the
/// entire tree can be walked without extra allocations.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum AguiNode {
    /// Plain text rendered in a `<span class="agui-text">`.
    Text(String),
    /// A layout wrapper; children are appended in order.
    Container(Vec<AguiNode>),
    /// A button; delegates to `mui-rs-components::button` → MWC3.
    Button(ButtonProps),
    /// A single-line text input field.
    Input(InputProps),
    /// A tool-call progress / result card.
    ToolCall(ToolCallProps),
    /// Agent-authored markdown; host applies its own markdown renderer.
    Markdown(String),
    /// An image element.
    Image(ImageProps),
    /// A syntax-highlighted code block.
    Code(CodeProps),
}

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// Errors returned by [`parse`].
#[derive(Debug, thiserror::Error)]
pub enum AguiError {
    /// The input string is not valid JSON or does not match the expected shape.
    #[error("agui wire-format parse error: {0}")]
    Json(#[from] serde_json::Error),
}

// ---------------------------------------------------------------------------
// parse — WireNode JSON → AguiNode tree
// ---------------------------------------------------------------------------

/// Deserialise a JSON-encoded `WireNode` into an [`AguiNode`] tree.
///
/// The `json` string must contain a single JSON object whose `"type"` field
/// selects the variant.  Nested containers are converted recursively.
///
/// # Errors
/// Returns [`AguiError::Json`] if the input is not valid JSON or the shape
/// does not match any recognised `WireNode` variant.
pub fn parse(json: &str) -> Result<AguiNode, AguiError> {
    let wire: WireNode = serde_json::from_str(json)?;
    Ok(convert_wire(wire))
}

/// Recursively convert a [`WireNode`] into an [`AguiNode`].
fn convert_wire(w: WireNode) -> AguiNode {
    match w {
        WireNode::Text { text } => AguiNode::Text(text),
        WireNode::Container { children } => {
            AguiNode::Container(children.into_iter().map(convert_wire).collect())
        },
        WireNode::Button { props } => AguiNode::Button(ButtonProps {
            label: props.label,
            variant: match props.variant.as_str() {
                "filled" => ButtonVariant::Filled,
                "filledTonal" => ButtonVariant::FilledTonal,
                "outlined" => ButtonVariant::Outlined,
                "elevated" => ButtonVariant::Elevated,
                _ => ButtonVariant::Text,
            },
            disabled: props.disabled,
            on_click_id: props.on_click_id,
            icon: None, // Wire format doesn't currently specify an icon
        }),
        WireNode::Input { props } => AguiNode::Input(wire_input_to_props(props)),
        WireNode::ToolCall { props } => AguiNode::ToolCall(wire_tool_call_to_props(props)),
        WireNode::Markdown { text } => AguiNode::Markdown(text),
        WireNode::Image { props } => AguiNode::Image(wire_image_to_props(props)),
        WireNode::Code { props } => AguiNode::Code(wire_code_to_props(props)),
    }
}

fn wire_input_to_props(w: WireInput) -> InputProps {
    InputProps {
        label: w.label,
        value: w.value,
        placeholder: w.placeholder,
        disabled: w.disabled,
        on_input_id: w.on_input_id,
    }
}

fn wire_tool_call_to_props(w: WireToolCall) -> ToolCallProps {
    ToolCallProps { tool_name: w.tool_name, args: w.args, status: w.status, result: w.result }
}

fn wire_image_to_props(w: WireImage) -> ImageProps {
    ImageProps { src: w.src, alt: w.alt, width_px: w.width_px, height_px: w.height_px }
}

fn wire_code_to_props(w: WireCode) -> CodeProps {
    CodeProps { code: w.code, lang: w.lang }
}

// ---------------------------------------------------------------------------
// render — AguiNode → DOM (wasm32 only)
// ---------------------------------------------------------------------------

/// Mount an [`AguiNode`] tree into a DOM element identified by `container_id`.
///
/// The container element is cleared before the new tree is appended. Each
/// `AguiNode` variant maps to one or more DOM nodes as described in the crate
/// doc mapping table.  Button nodes are built via `mui_rs_components::button` so
/// they emit MWC3 `<md-*-button>` elements and share the same event-routing
/// contract (`data-on-click-id`).
///
/// # Errors
/// Returns a [`JsValue`] describing the DOM error if:
/// - `container_id` is not found in the document.
/// - Any DOM creation call fails (e.g. missing document in a Worker).
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = render)]
pub fn render(node: &AguiNode, container_id: &str) -> Result<(), JsValue> {
    let doc = document()?;
    let container = doc
        .get_element_by_id(container_id)
        .ok_or_else(|| JsValue::from_str(&format!("container #{container_id} not found")))?;
    // Clear the container before mounting the new tree.
    container.set_inner_html("");
    let child = build_element(&doc, node)?;
    container.append_child(&child)?;
    Ok(())
}

/// Render an [`AguiNode`] from its JSON representation.  Convenience wrapper
/// for JS callers that already have the node as a JSON string.
///
/// # Errors
/// Returns a [`JsValue`] if `node_json` cannot be parsed or DOM ops fail.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen(js_name = renderJson)]
pub fn render_json(node_json: &str, container_id: &str) -> Result<(), JsValue> {
    let node = parse(node_json).map_err(|e| JsValue::from_str(&format!("parse error: {e}")))?;
    render(&node, container_id)
}

// ---------------------------------------------------------------------------
// DOM helpers (wasm32 only)
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
fn document() -> Result<Document, JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("no window"))?;
    window.document().ok_or_else(|| JsValue::from_str("no document"))
}

/// Recursively build a DOM subtree for `node` in `doc`.
#[cfg(target_arch = "wasm32")]
fn build_element(doc: &Document, node: &AguiNode) -> Result<Element, JsValue> {
    match node {
        AguiNode::Text(text) => {
            let el = create_el(doc, "span")?;
            el.class_list().add_1("agui-text")?;
            el.set_text_content(Some(text));
            Ok(el)
        },
        AguiNode::Container(children) => {
            let el = create_el(doc, "div")?;
            el.class_list().add_1("agui-container")?;
            for child in children {
                let child_el = build_element(doc, child)?;
                el.append_child(&child_el)?;
            }
            Ok(el)
        },
        AguiNode::Button(props) => {
            // Delegate to mui-rs-components — produces <md-*-button>.
            let btn = mui_rs_components::button::create_button(props)?;
            btn.class_list().add_1("agui-button")?;
            // dyn_into from HtmlElement to Element via From impl
            Ok(btn.into())
        },
        AguiNode::Input(props) => build_input(doc, props),
        AguiNode::ToolCall(props) => build_tool_call(doc, props),
        AguiNode::Markdown(text) => {
            let el = create_el(doc, "div")?;
            el.class_list().add_1("agui-markdown")?;
            el.set_text_content(Some(text));
            Ok(el)
        },
        AguiNode::Image(props) => build_image(doc, props),
        AguiNode::Code(props) => build_code(doc, props),
    }
}

#[cfg(target_arch = "wasm32")]
fn create_el(doc: &Document, tag: &str) -> Result<Element, JsValue> {
    doc.create_element(tag)
}

#[cfg(target_arch = "wasm32")]
fn as_html(el: Element) -> Result<HtmlElement, JsValue> {
    use wasm_bindgen::JsCast as _;
    el.dyn_into::<HtmlElement>().map_err(|_| JsValue::from_str("element is not HtmlElement"))
}

#[cfg(target_arch = "wasm32")]
fn build_input(doc: &Document, props: &InputProps) -> Result<Element, JsValue> {
    // Use MWC3 md-outlined-text-field when available; fall back to native
    // <input> — the element tag is the same MWC3 custom element the
    // mui-rs-components uses for its Input module.
    let el = create_el(doc, "md-outlined-text-field")?;
    el.class_list().add_1("agui-input")?;
    if !props.label.is_empty() {
        el.set_attribute("label", &props.label)?;
    }
    if !props.value.is_empty() {
        el.set_attribute("value", &props.value)?;
    }
    if !props.placeholder.is_empty() {
        el.set_attribute("placeholder", &props.placeholder)?;
    }
    if props.disabled {
        el.set_attribute("disabled", "")?;
    }
    if let Some(id) = &props.on_input_id {
        el.set_attribute("data-on-input-id", id)?;
    }
    Ok(el)
}

#[cfg(target_arch = "wasm32")]
fn build_tool_call(doc: &Document, props: &ToolCallProps) -> Result<Element, JsValue> {
    let card = create_el(doc, "div")?;
    card.class_list().add_1("agui-tool-call")?;

    // Tool name header
    let header = create_el(doc, "div")?;
    header.class_list().add_1("agui-tool-call__name")?;
    header.set_text_content(Some(&props.tool_name));
    card.append_child(&header)?;

    // Status badge
    if let Some(status) = &props.status {
        let badge = create_el(doc, "span")?;
        badge.class_list().add_1("agui-tool-call__status")?;
        let status_str = match status {
            wire::ToolCallStatus::Started => "started",
            wire::ToolCallStatus::Completed => "completed",
            wire::ToolCallStatus::Failed => "failed",
        };
        badge.set_attribute("data-status", status_str)?;
        badge.set_text_content(Some(status_str));
        card.append_child(&badge)?;
    }

    // Args (compact JSON)
    let args_el = create_el(doc, "pre")?;
    args_el.class_list().add_1("agui-tool-call__args")?;
    let args_json = serde_json::to_string(&props.args).unwrap_or_else(|_| "{}".to_owned());
    args_el.set_text_content(Some(&args_json));
    card.append_child(&args_el)?;

    // Result (if present)
    if let Some(result) = &props.result {
        let result_el = create_el(doc, "pre")?;
        result_el.class_list().add_1("agui-tool-call__result")?;
        let result_json = serde_json::to_string(result).unwrap_or_else(|_| "null".to_owned());
        result_el.set_text_content(Some(&result_json));
        card.append_child(&result_el)?;
    }

    Ok(card)
}

#[cfg(target_arch = "wasm32")]
fn build_image(doc: &Document, props: &ImageProps) -> Result<Element, JsValue> {
    use wasm_bindgen::JsCast as _;
    let el = doc.create_element("img")?;
    let img = el
        .dyn_into::<web_sys::HtmlImageElement>()
        .map_err(|_| JsValue::from_str("img is not HtmlImageElement"))?;
    img.class_list().add_1("agui-image")?;
    img.set_src(&props.src);
    img.set_alt(&props.alt);
    if let Some(w) = props.width_px {
        img.set_attribute("width", &w.to_string())?;
    }
    if let Some(h) = props.height_px {
        img.set_attribute("height", &h.to_string())?;
    }
    Ok(img.into())
}

#[cfg(target_arch = "wasm32")]
fn build_code(doc: &Document, props: &CodeProps) -> Result<Element, JsValue> {
    let pre = create_el(doc, "pre")?;
    pre.class_list().add_1("agui-code")?;
    let code_el = create_el(doc, "code")?;
    if let Some(lang) = &props.lang {
        // Convention: class="language-<lang>" aligns with highlight.js / Prism.
        code_el.class_list().add_1(&format!("language-{lang}"))?;
    }
    code_el.set_text_content(Some(&props.code));
    pre.append_child(&code_el)?;
    Ok(pre)
}

// ---------------------------------------------------------------------------
// Unused-import suppression: `as_html` is intentionally available for callers
// that need to set individual DOM properties on HtmlElement, but the current
// render paths above don't always use it (they stay at Element level). The
// function is kept for completeness of the public internal API.
// ---------------------------------------------------------------------------
#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
fn _use_as_html_marker(el: Element) -> Result<HtmlElement, JsValue> {
    as_html(el)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------
#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::wire::{
        EventBase, RunLifecyclePayload, RunStatus, ToolCallStatus as WireStatus, WireEvent,
        WireNode,
    };

    // -----------------------------------------------------------------------
    // Helper: round-trip a WireNode through JSON and back.
    // -----------------------------------------------------------------------
    fn round_trip(node: &WireNode) -> WireNode {
        let json = serde_json::to_string(node).expect("serialize");
        serde_json::from_str(&json).expect("deserialize")
    }

    // -----------------------------------------------------------------------
    // WireNode JSON round-trip — one per variant
    // -----------------------------------------------------------------------

    #[test]
    fn wire_round_trip_text() {
        let n = WireNode::Text { text: "hello agui".to_owned() };
        assert_eq!(round_trip(&n), n);
    }

    #[test]
    fn wire_round_trip_container_nested() {
        let n = WireNode::Container {
            children: vec![WireNode::Text { text: "child".to_owned() }, WireNode::Container {
                children: vec![WireNode::Text { text: "deep".to_owned() }],
            }],
        };
        assert_eq!(round_trip(&n), n);
    }

    #[test]
    fn wire_round_trip_button() {
        let n = WireNode::Button {
            props: wire::WireButton {
                label: "Click me".to_owned(),
                variant: "filled".to_owned(),
                disabled: false,
                on_click_id: Some("btn-1".to_owned()),
            },
        };
        assert_eq!(round_trip(&n), n);
    }

    #[test]
    fn wire_round_trip_input() {
        let n = WireNode::Input {
            props: wire::WireInput {
                label: "Name".to_owned(),
                value: "Alice".to_owned(),
                placeholder: "Enter name".to_owned(),
                disabled: false,
                on_input_id: Some("inp-name".to_owned()),
            },
        };
        assert_eq!(round_trip(&n), n);
    }

    #[test]
    fn wire_round_trip_tool_call() {
        let n = WireNode::ToolCall {
            props: wire::WireToolCall {
                tool_name: "live-artifacts.create".to_owned(),
                args: serde_json::json!({"name": "index.html"}),
                status: Some(WireStatus::Completed),
                result: Some(serde_json::json!({"ok": true})),
            },
        };
        assert_eq!(round_trip(&n), n);
    }

    #[test]
    fn wire_round_trip_markdown() {
        let n = WireNode::Markdown { text: "## Hello\n\nWorld".to_owned() };
        assert_eq!(round_trip(&n), n);
    }

    #[test]
    fn wire_round_trip_image() {
        let n = WireNode::Image {
            props: wire::WireImage {
                src: "https://example.com/img.png".to_owned(),
                alt: "example".to_owned(),
                width_px: Some(640),
                height_px: Some(480),
            },
        };
        assert_eq!(round_trip(&n), n);
    }

    #[test]
    fn wire_round_trip_code() {
        let n = WireNode::Code {
            props: wire::WireCode {
                code: r#"fn main() { println!("hi"); }"#.to_owned(),
                lang: Some("rust".to_owned()),
            },
        };
        assert_eq!(round_trip(&n), n);
    }

    // -----------------------------------------------------------------------
    // parse() — happy path: every AguiNode variant
    // -----------------------------------------------------------------------

    #[test]
    fn parse_text_node() {
        let json = r#"{"type":"text","text":"hello"}"#;
        let node = parse(json).expect("parse text");
        assert!(matches!(node, AguiNode::Text(t) if t == "hello"));
    }

    #[test]
    fn parse_container_with_children() {
        let json = r#"{"type":"container","children":[{"type":"text","text":"a"},{"type":"text","text":"b"}]}"#;
        let node = parse(json).expect("parse container");
        match node {
            AguiNode::Container(children) => assert_eq!(children.len(), 2),
            _ => panic!("expected Container"),
        }
    }

    #[test]
    fn parse_button_node() {
        let json = r#"{"type":"button","label":"Go","variant":"filled","disabled":false}"#;
        let node = parse(json).expect("parse button");
        match node {
            AguiNode::Button(p) => {
                assert_eq!(p.label, "Go");
                assert_eq!(p.variant, "filled");
                assert!(!p.disabled);
            },
            _ => panic!("expected Button"),
        }
    }

    #[test]
    fn parse_input_node() {
        let json = r#"{"type":"input","label":"Email","value":"","placeholder":"you@example.com","disabled":false}"#;
        let node = parse(json).expect("parse input");
        assert!(matches!(node, AguiNode::Input(_)));
    }

    #[test]
    fn parse_tool_call_node() {
        let json = r#"{"type":"toolCall","toolName":"search","args":{"q":"rust"},"status":"completed","result":{"hits":3}}"#;
        let node = parse(json).expect("parse toolCall");
        match node {
            AguiNode::ToolCall(p) => {
                assert_eq!(p.tool_name, "search");
                assert!(matches!(p.status, Some(wire::ToolCallStatus::Completed)));
            },
            _ => panic!("expected ToolCall"),
        }
    }

    #[test]
    fn parse_markdown_node() {
        let json = r##"{"type":"markdown","text":"# Title"}"##;
        let node = parse(json).expect("parse markdown");
        assert!(matches!(node, AguiNode::Markdown(t) if t == "# Title"));
    }

    #[test]
    fn parse_image_node() {
        let json = r#"{"type":"image","src":"/img.png","alt":"pic"}"#;
        let node = parse(json).expect("parse image");
        match node {
            AguiNode::Image(p) => assert_eq!(p.src, "/img.png"),
            _ => panic!("expected Image"),
        }
    }

    #[test]
    fn parse_code_node() {
        let json = r#"{"type":"code","code":"let x = 1;","lang":"rust"}"#;
        let node = parse(json).expect("parse code");
        match node {
            AguiNode::Code(p) => {
                assert_eq!(p.code, "let x = 1;");
                assert_eq!(p.lang.as_deref(), Some("rust"));
            },
            _ => panic!("expected Code"),
        }
    }

    // -----------------------------------------------------------------------
    // parse() — error paths
    // -----------------------------------------------------------------------

    #[test]
    fn parse_error_on_malformed_json() {
        let err = parse("{not json}").unwrap_err();
        assert!(matches!(err, AguiError::Json(_)), "expected Json error, got {err:?}");
    }

    #[test]
    fn parse_error_on_unknown_type() {
        let err = parse(r#"{"type":"mystery","foo":"bar"}"#).unwrap_err();
        assert!(matches!(err, AguiError::Json(_)));
    }

    #[test]
    fn parse_error_on_missing_required_field() {
        // Button requires "label"; omitting it should fail.
        let err = parse(r#"{"type":"button","variant":"filled"}"#).unwrap_err();
        assert!(matches!(err, AguiError::Json(_)));
    }

    #[test]
    fn parse_error_on_empty_string() {
        assert!(parse("").is_err());
    }

    // -----------------------------------------------------------------------
    // WireEvent round-trip (the stream-level envelope, not the UI tree)
    // -----------------------------------------------------------------------

    #[test]
    fn wire_event_agent_message_round_trip() {
        let ev = WireEvent::AgentMessage {
            base: EventBase { run_id: "run-1".to_owned(), seq: Some(3), ts: 1_700_000_000_000 },
            payload: wire::AgentMessagePayload { text: "hello".to_owned(), done: Some(true) },
        };
        let json = serde_json::to_string(&ev).expect("serialize");
        let back: WireEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(ev, back);
        assert_eq!(back.run_id(), "run-1");
        assert_eq!(back.ts(), 1_700_000_000_000);
    }

    #[test]
    fn wire_event_run_lifecycle_round_trip() {
        let ev = WireEvent::RunLifecycle {
            base: EventBase { run_id: "run-2".to_owned(), seq: None, ts: 42 },
            payload: RunLifecyclePayload {
                status: RunStatus::PipelineStageStarted,
                stage_id: Some("discovery".to_owned()),
                iteration: Some(0),
                message: None,
            },
        };
        let json = serde_json::to_string(&ev).expect("serialize");
        let back: WireEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(ev, back);
    }

    #[test]
    fn wire_event_state_update_round_trip() {
        let ev = WireEvent::StateUpdate {
            base: EventBase { run_id: "run-3".to_owned(), seq: None, ts: 0 },
            payload: wire::StateUpdatePayload {
                path: "genui.figma-oauth".to_owned(),
                value: serde_json::json!({"persistTier": "project"}),
            },
        };
        let json = serde_json::to_string(&ev).expect("serialize");
        let back: WireEvent = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(ev, back);
    }
}
