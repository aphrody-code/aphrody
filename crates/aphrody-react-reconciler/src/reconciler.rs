// SPDX-License-Identifier: Apache-2.0
//
// Top-level `Reconciler` façade. Mirrors the surface the TS `createRoot()` /
// `disposeRoot()` pair exposes plus the `resetAfterCommit()` decision tree
// from the TS HostConfig: first commit emits a `mount` frame, subsequent
// commits emit an `update` frame containing the accumulated patch list.
//
// The TS reconciler also routes deeper React-fiber events (insertBefore,
// removeChild, commitTextUpdate, hide/unhide). Those are exposed here as
// explicit Rust methods on `Reconciler` so a host can drive the tree from
// Rust without round-tripping through a JS runtime.

//! `Reconciler` — root container + commit pipeline.

use alloc::rc::Rc;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::cell::RefCell;

use serde_json::{Map, Value};
use thiserror::Error;

use crate::element::{
    HostNode, JsonNode, JsonPatchOp, append_child, create_element, create_text_instance, detach,
    diff_props, serialize, ElementKind,
};
use crate::hooks::HookState;
use crate::osc::{
    emit, MountFields, OscFrame, OscSink, UnmountFields, UpdateFields,
};

/// Errors surfaced by the reconciler façade.
#[derive(Debug, Error)]
pub enum ReconcilerError {
    /// `commit_text_update` was called on a non-text node.
    #[error("commit_text_update called on a non-text node")]
    NotATextNode,
    /// `commit_update` was called on a non-element node.
    #[error("commit_update called on a non-element node")]
    NotAnElementNode,
}

/// Root container + commit pipeline. One instance per logical React tree
/// (matches `ReconcilerRoot` in `reconciler.ts`).
pub struct Reconciler {
    root_id: String,
    root: Rc<HostNode>,
    mounted: RefCell<bool>,
    pending_patch: RefCell<Vec<JsonPatchOp>>,
    hooks: HookState,
}

impl core::fmt::Debug for Reconciler {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Reconciler")
            .field("root_id", &self.root_id)
            .field("mounted", &self.mounted.borrow())
            .field("pending_patch_len", &self.pending_patch.borrow().len())
            .finish()
    }
}

impl Reconciler {
    /// Creates a fresh reconciler with a synthetic `Root` element.
    #[must_use]
    pub fn new() -> Self {
        let root = create_element(ElementKind::Root, Map::new());
        let root_id = root.id().to_string();
        Self {
            root_id,
            root,
            mounted: RefCell::new(false),
            pending_patch: RefCell::new(Vec::new()),
            hooks: HookState::new(),
        }
    }

    /// Returns the root node id.
    #[must_use]
    pub fn root_id(&self) -> &str {
        &self.root_id
    }

    /// Returns a clone of the root host instance.
    #[must_use]
    pub fn root(&self) -> Rc<HostNode> {
        Rc::clone(&self.root)
    }

    /// Returns a reference to the hook state queue.
    #[must_use]
    pub fn hooks(&self) -> &HookState {
        &self.hooks
    }

    /// Appends `child` to the container root. Mirrors
    /// `appendChildToContainer()` — emits an `insert` patch if already
    /// mounted; otherwise the child is staged for the initial mount frame.
    pub fn append_child_to_container(&self, child: Rc<HostNode>) {
        append_child(&self.root, &child);
        if *self.mounted.borrow() {
            self.pending_patch.borrow_mut().push(JsonPatchOp::Insert {
                parent_id: self.root_id.clone(),
                before_id: None,
                node: serialize(&child),
            });
        }
    }

    /// Inserts `child` before `before_child` inside the container. Mirrors
    /// `insertInContainerBefore()`.
    pub fn insert_in_container_before(&self, child: Rc<HostNode>, before_child: &Rc<HostNode>) {
        detach(&child);
        if let HostNode::Element { children, .. } = self.root.as_ref() {
            let mut kids = children.borrow_mut();
            let idx = kids
                .iter()
                .position(|c| Rc::ptr_eq(c, before_child))
                .unwrap_or(kids.len());
            child.set_parent_to(&self.root);
            kids.insert(idx, Rc::clone(&child));
        }
        if *self.mounted.borrow() {
            self.pending_patch.borrow_mut().push(JsonPatchOp::Insert {
                parent_id: self.root_id.clone(),
                before_id: Some(before_child.id().to_string()),
                node: serialize(&child),
            });
        }
    }

    /// Removes `child` from its current parent + queues a remove patch.
    /// Mirrors `removeChild()`/`removeChildFromContainer()`.
    pub fn remove_child(&self, child: &Rc<HostNode>) {
        let child_id = child.id().to_string();
        detach(child);
        if *self.mounted.borrow() {
            self.pending_patch
                .borrow_mut()
                .push(JsonPatchOp::Remove { id: child_id });
        }
    }

    /// Replaces the text content of `text_instance`. Mirrors
    /// `commitTextUpdate()`.
    pub fn commit_text_update(
        &self,
        text_instance: &Rc<HostNode>,
        next_text: &str,
    ) -> Result<(), ReconcilerError> {
        match text_instance.as_ref() {
            HostNode::Text { id, value, .. } => {
                *value.borrow_mut() = next_text.to_string();
                if *self.mounted.borrow() {
                    self.pending_patch.borrow_mut().push(JsonPatchOp::ReplaceText {
                        id: id.clone(),
                        value: next_text.to_string(),
                    });
                }
                Ok(())
            }
            HostNode::Element { .. } => Err(ReconcilerError::NotATextNode),
        }
    }

    /// Replaces the prop bag of an element instance and queues any resulting
    /// diff patches. Mirrors `commitUpdate()`.
    pub fn commit_update(
        &self,
        instance: &Rc<HostNode>,
        new_props: Map<String, Value>,
    ) -> Result<(), ReconcilerError> {
        match instance.as_ref() {
            HostNode::Element { id, props, .. } => {
                let prev = props.borrow().clone();
                *props.borrow_mut() = new_props.clone();
                if *self.mounted.borrow() {
                    let ops = diff_props(id, &prev, &new_props);
                    self.pending_patch.borrow_mut().extend(ops);
                }
                Ok(())
            }
            HostNode::Text { .. } => Err(ReconcilerError::NotAnElementNode),
        }
    }

    /// Final stage of a commit cycle. On the first call emits a `mount`
    /// frame containing the full subtree; on subsequent calls emits an
    /// `update` frame containing the accumulated patch list (skipped when
    /// empty). Mirrors `resetAfterCommit()`.
    pub fn commit(&self, sink: &mut dyn OscSink) {
        let mut mounted = self.mounted.borrow_mut();
        if !*mounted {
            let tree = serialize(&self.root);
            emit(
                sink,
                &OscFrame::Mount(MountFields {
                    id: self.root_id.clone(),
                    tree,
                }),
            );
            *mounted = true;
            self.pending_patch.borrow_mut().clear();
        } else {
            let patch: Vec<JsonPatchOp> =
                core::mem::take(&mut *self.pending_patch.borrow_mut());
            if patch.is_empty() {
                return;
            }
            emit(
                sink,
                &OscFrame::Update(UpdateFields {
                    id: self.root_id.clone(),
                    patch,
                }),
            );
        }
        drop(mounted);
        // Drain any pending effects queued by hook setters during commit.
        self.hooks.drain_and_run();
    }

    /// Emits an `unmount` frame and drops the root reference. Mirrors
    /// `disposeRoot()`.
    pub fn dispose(self, sink: &mut dyn OscSink) {
        emit(
            sink,
            &OscFrame::Unmount(UnmountFields {
                id: self.root_id.clone(),
            }),
        );
    }

    /// Returns a clone of the currently-pending patch list (useful for
    /// tests that don't want to commit yet).
    #[must_use]
    pub fn pending_patch(&self) -> Vec<JsonPatchOp> {
        self.pending_patch.borrow().clone()
    }

    /// Returns the wire-format root tree snapshot.
    #[must_use]
    pub fn snapshot(&self) -> JsonNode {
        serialize(&self.root)
    }

    /// Convenience constructor: append an element child by kind + props.
    /// Returns the freshly created child node.
    pub fn append_element(&self, kind: ElementKind, props: Map<String, Value>) -> Rc<HostNode> {
        let child = create_element(kind, props);
        self.append_child_to_container(Rc::clone(&child));
        child
    }

    /// Convenience constructor: append a text child literal.
    /// Returns the freshly created text node.
    pub fn append_text(&self, text: impl Into<String>) -> Rc<HostNode> {
        let child = create_text_instance(text);
        self.append_child_to_container(Rc::clone(&child));
        child
    }
}

impl Default for Reconciler {
    fn default() -> Self {
        Self::new()
    }
}

// Bridging helper: HostNode::set_parent is module-private in element.rs,
// so we use a tiny shim trait restricted to this module.
trait HostNodeParentShim {
    fn set_parent_to(&self, parent: &Rc<HostNode>);
}

impl HostNodeParentShim for HostNode {
    fn set_parent_to(&self, parent: &Rc<HostNode>) {
        // SAFETY: we go through the same `RefCell` path the element module
        // uses internally — there's no aliasing concern.
        let new_weak = Some(Rc::downgrade(parent));
        match self {
            Self::Element { parent: p, .. } | Self::Text { parent: p, .. } => {
                *p.borrow_mut() = new_weak;
            }
        }
    }
}
