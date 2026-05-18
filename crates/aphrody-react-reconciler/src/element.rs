// SPDX-License-Identifier: Apache-2.0
//
// Port of `src/layout.ts` + `src/types.ts` + the element-creation paths
// owned by `src/reconciler.ts::createInstance/createTextInstance`.
//
// We use `RefCell<Rc<...>>` for parent links so the tree matches the TS
// mutable-graph shape exactly. This is single-threaded by design — the React
// reconciler model is inherently single-threaded; concurrency is the caller's
// problem and gets handled at the OSC-sink level.

//! Element tree, prop normalization, diff, and JSON serializer.

use alloc::borrow::ToOwned;
use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cell::RefCell;
use core::sync::atomic::{AtomicU64, Ordering};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::m3_tokens;

/// The 7 element kinds the reconciler recognizes.
///
/// Direct port of `JsonElement["type"]` from `types.ts`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize, Deserialize)]
pub enum ElementKind {
    /// Layout container.
    Box,
    /// Text wrapper.
    Text,
    /// `\n` shim element.
    Newline,
    /// Static (Ink-style append-only) list.
    Static,
    /// Output transform.
    Transform,
    /// Flexbox spacer.
    Spacer,
    /// Synthetic root injected by `Reconciler::new`.
    Root,
}

impl ElementKind {
    /// Returns the wire-format name (matches `JsonElement.type` in TS).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Box => "Box",
            Self::Text => "Text",
            Self::Newline => "Newline",
            Self::Static => "Static",
            Self::Transform => "Transform",
            Self::Spacer => "Spacer",
            Self::Root => "Root",
        }
    }

    /// Parses an `ElementKind` from a wire string. Mirrors
    /// `asElementType(value)` from the TS reconciler.
    pub fn from_str(value: &str) -> Result<Self, String> {
        match value {
            "Box" => Ok(Self::Box),
            "Text" => Ok(Self::Text),
            "Newline" => Ok(Self::Newline),
            "Static" => Ok(Self::Static),
            "Transform" => Ok(Self::Transform),
            "Spacer" => Ok(Self::Spacer),
            "Root" => Ok(Self::Root),
            other => Err(alloc::format!(
                "aphrody-react-reconciler: unknown element type \"{other}\""
            )),
        }
    }
}

/// Typed prop value. Covers the union of scalar shapes the TS source
/// accepts (numbers, strings, booleans, plus null). Arbitrary JSON values
/// are also supported through `Json` for callers that need it.
#[derive(Clone, Debug, PartialEq)]
pub enum PropValue {
    /// Suppressed during normalization (mirrors `value === null`).
    Null,
    /// Boolean prop. `false` is dropped during normalization.
    Bool(bool),
    /// Numeric prop (matches JS `number`).
    Number(f64),
    /// String prop (matches JS `string`).
    Str(String),
    /// Arbitrary `serde_json::Value` escape hatch.
    Json(Value),
}

impl PropValue {
    fn to_json(&self) -> Value {
        match self {
            Self::Null => Value::Null,
            Self::Bool(b) => Value::Bool(*b),
            Self::Number(n) => {
                if let Some(v) = serde_json::Number::from_f64(*n) {
                    Value::Number(v)
                } else {
                    Value::Null
                }
            }
            Self::Str(s) => Value::String(s.clone()),
            Self::Json(v) => v.clone(),
        }
    }
}

impl From<bool> for PropValue {
    fn from(b: bool) -> Self {
        Self::Bool(b)
    }
}

impl From<i64> for PropValue {
    #[allow(clippy::cast_precision_loss)]
    fn from(n: i64) -> Self {
        Self::Number(n as f64)
    }
}

impl From<u64> for PropValue {
    #[allow(clippy::cast_precision_loss)]
    fn from(n: u64) -> Self {
        Self::Number(n as f64)
    }
}

impl From<f64> for PropValue {
    fn from(n: f64) -> Self {
        Self::Number(n)
    }
}

impl From<&str> for PropValue {
    fn from(s: &str) -> Self {
        Self::Str(s.to_owned())
    }
}

impl From<String> for PropValue {
    fn from(s: String) -> Self {
        Self::Str(s)
    }
}

/// Wire-shape JSON tree node — matches `JsonNode` in `types.ts`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum JsonNode {
    /// Element node.
    Element {
        /// Element kind tag.
        #[serde(rename = "type")]
        kind: ElementKind,
        /// Stable node id.
        id: String,
        /// Normalized prop bag.
        props: Map<String, Value>,
        /// Ordered children.
        children: Vec<JsonNode>,
    },
    /// Text node (literal string content).
    TextNode {
        /// Always `"TEXT_NODE"` on the wire.
        #[serde(rename = "type")]
        kind: TextNodeMarker,
        /// Stable node id.
        id: String,
        /// Literal text content.
        value: String,
    },
}

/// Sentinel enum that serializes to the literal `"TEXT_NODE"` string.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TextNodeMarker {
    /// Sole variant.
    #[serde(rename = "TEXT_NODE")]
    TextNode,
}

/// Patch operations sent inside `aphrody-jsx-update` frames.
/// Direct port of `JsonPatchOp` from `types.ts`.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(tag = "op", rename_all = "kebab-case")]
pub enum JsonPatchOp {
    /// Replace a single prop.
    #[serde(rename = "replace-prop")]
    ReplaceProp {
        /// Target node id.
        id: String,
        /// Prop key.
        key: String,
        /// New prop value (post-normalization).
        value: Value,
    },
    /// Remove a single prop.
    #[serde(rename = "remove-prop")]
    RemoveProp {
        /// Target node id.
        id: String,
        /// Prop key to remove.
        key: String,
    },
    /// Replace text-node content.
    #[serde(rename = "replace-text")]
    ReplaceText {
        /// Target text-node id.
        id: String,
        /// New literal value.
        value: String,
    },
    /// Insert a new subtree.
    Insert {
        /// Parent id.
        #[serde(rename = "parentId")]
        parent_id: String,
        /// Sibling id to insert before (`null` => append).
        #[serde(rename = "beforeId")]
        before_id: Option<String>,
        /// New subtree to splice in.
        node: JsonNode,
    },
    /// Remove a subtree by id.
    Remove {
        /// Target node id.
        id: String,
    },
}

/// Mutable host-instance graph node. Variant tag matches `kind` in the TS
/// source so the wire-format mapping is 1:1.
#[derive(Debug)]
pub enum HostNode {
    /// Element instance.
    Element {
        /// Element kind.
        kind: ElementKind,
        /// Stable id.
        id: String,
        /// Live (un-normalized) prop bag.
        props: RefCell<Map<String, Value>>,
        /// Ordered children.
        children: RefCell<Vec<Rc<HostNode>>>,
        /// Weak parent link (cycle break).
        parent: RefCell<Option<alloc::rc::Weak<HostNode>>>,
    },
    /// Text instance.
    Text {
        /// Stable id.
        id: String,
        /// Live literal value.
        value: RefCell<String>,
        /// Weak parent link (cycle break).
        parent: RefCell<Option<alloc::rc::Weak<HostNode>>>,
    },
}

impl HostNode {
    /// Returns the node id.
    #[must_use]
    pub fn id(&self) -> &str {
        match self {
            Self::Element { id, .. } | Self::Text { id, .. } => id,
        }
    }

    /// Returns true if this node is a text instance.
    #[must_use]
    pub const fn is_text(&self) -> bool {
        matches!(self, Self::Text { .. })
    }

    /// Returns the parent (if any).
    #[must_use]
    pub fn parent(&self) -> Option<Rc<HostNode>> {
        let weak = match self {
            Self::Element { parent, .. } | Self::Text { parent, .. } => parent.borrow().clone(),
        };
        weak.and_then(|w| w.upgrade())
    }

    fn set_parent(&self, parent: Option<&Rc<HostNode>>) {
        let new_weak = parent.map(Rc::downgrade);
        match self {
            Self::Element { parent: p, .. } | Self::Text { parent: p, .. } => {
                *p.borrow_mut() = new_weak;
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Id generator.
// ---------------------------------------------------------------------------

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

/// Returns a fresh string id (monotonically increasing). Mirrors `freshId()`.
#[must_use]
pub fn next_id() -> String {
    let v = NEXT_ID.fetch_add(1, Ordering::SeqCst).wrapping_add(1);
    v.to_string()
}

/// Test-only: resets the id counter so snapshots are deterministic.
/// Mirrors `resetIds()` from the TS source.
pub fn freshen_ids() {
    NEXT_ID.store(0, Ordering::SeqCst);
}

// ---------------------------------------------------------------------------
// Constructors.
// ---------------------------------------------------------------------------

/// Builds an `Element` host instance. Equivalent to
/// `hostConfig.createInstance(type, props)` from the TS reconciler.
#[must_use]
pub fn create_element(kind: ElementKind, props: Map<String, Value>) -> Rc<HostNode> {
    Rc::new(HostNode::Element {
        kind,
        id: next_id(),
        props: RefCell::new(props),
        children: RefCell::new(Vec::new()),
        parent: RefCell::new(None),
    })
}

/// Builds a `Text` host instance. Equivalent to
/// `hostConfig.createTextInstance(text)`.
#[must_use]
pub fn create_text_instance(text: impl Into<String>) -> Rc<HostNode> {
    Rc::new(HostNode::Text {
        id: next_id(),
        value: RefCell::new(text.into()),
        parent: RefCell::new(None),
    })
}

/// Attaches `child` to `parent`. Mirrors `appendInitialChild`/`appendChild`.
pub fn append_child(parent: &Rc<HostNode>, child: &Rc<HostNode>) {
    if let HostNode::Element { children, .. } = parent.as_ref() {
        child.set_parent(Some(parent));
        children.borrow_mut().push(Rc::clone(child));
    }
}

/// Detaches `child` from its current parent (if any). Mirrors `detach()`.
pub fn detach(child: &Rc<HostNode>) {
    let Some(parent) = child.parent() else {
        return;
    };
    if let HostNode::Element { children, .. } = parent.as_ref() {
        let mut kids = children.borrow_mut();
        if let Some(idx) = kids.iter().position(|c| Rc::ptr_eq(c, child)) {
            kids.remove(idx);
        }
    }
    child.set_parent(None);
}

// ---------------------------------------------------------------------------
// Normalization + serialization.
// ---------------------------------------------------------------------------

/// Normalize props before serialization — direct port of `normalizeProps`.
///
/// Rules (parity with TS):
///   * `children` key is dropped (handled out-of-band).
///   * Values that are `null`/`undefined`/`false` are dropped.
///   * Color-typed keys (`color`, `backgroundColor`, `borderColor`) get the
///     `m3:` prefix when they match a known token.
///   * Padding/margin shorthand expansion is the renderer's responsibility,
///     mirroring how the TS Box component pre-expands before commit.
#[must_use]
pub fn normalize_props(props: &Map<String, Value>) -> Map<String, Value> {
    let mut out = Map::with_capacity(props.len());
    for (key, raw) in props {
        if key == "children" {
            continue;
        }
        if matches!(raw, Value::Null) {
            continue;
        }
        if matches!(raw, Value::Bool(false)) {
            continue;
        }
        if (key == "color" || key == "backgroundColor" || key == "borderColor")
            && let Value::String(s) = raw
        {
            out.insert(key.clone(), Value::String(m3_tokens::encode_color(s)));
            continue;
        }
        out.insert(key.clone(), raw.clone());
    }
    out
}

/// Serializes a host node into the wire-format `JsonNode`. Mirrors
/// `serialize()` from `layout.ts`.
#[must_use]
pub fn serialize(node: &Rc<HostNode>) -> JsonNode {
    match node.as_ref() {
        HostNode::Text { id, value, .. } => JsonNode::TextNode {
            kind: TextNodeMarker::TextNode,
            id: id.clone(),
            value: value.borrow().clone(),
        },
        HostNode::Element {
            kind,
            id,
            props,
            children,
            ..
        } => JsonNode::Element {
            kind: *kind,
            id: id.clone(),
            props: normalize_props(&props.borrow()),
            children: children.borrow().iter().map(serialize).collect(),
        },
    }
}

/// Computes a minimal patch list between two prop bags. Used for element
/// instances only (text instances use `replace-text`). Direct port of
/// `diffProps()` from `layout.ts`.
#[must_use]
pub fn diff_props(
    id: &str,
    prev: &Map<String, Value>,
    next: &Map<String, Value>,
) -> Vec<JsonPatchOp> {
    let prev_norm = normalize_props(prev);
    let next_norm = normalize_props(next);
    let mut ops = Vec::new();

    for key in prev_norm.keys() {
        if !next_norm.contains_key(key) {
            ops.push(JsonPatchOp::RemoveProp {
                id: id.to_string(),
                key: key.clone(),
            });
        }
    }
    for (key, value) in &next_norm {
        let unchanged = prev_norm.get(key).is_some_and(|p| p == value);
        if !unchanged {
            ops.push(JsonPatchOp::ReplaceProp {
                id: id.to_string(),
                key: key.clone(),
                value: value.clone(),
            });
        }
    }
    ops
}

// PropValue helpers for ergonomic prop-bag construction in tests.
impl From<PropValue> for Value {
    fn from(p: PropValue) -> Self {
        p.to_json()
    }
}
