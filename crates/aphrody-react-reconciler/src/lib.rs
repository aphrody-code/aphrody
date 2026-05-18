// SPDX-License-Identifier: Apache-2.0
//
// aphrody-react-reconciler — host-side primitives ported from
// `packages/aphrody-jsx` (1962 LoC TypeScript). This crate provides a
// cross-platform (linux / windows / wasm32) Rust implementation of:
//
//   * an `Element` tree with mutable parent links (host instances + text nodes)
//   * `create_element` / `create_text_instance` constructors
//   * a deterministic prop-diff algorithm matching `layout.ts::diffProps`
//   * a JSON serializer matching `serialize()` (incl. M3 color marker rewrite)
//   * the `aphrody-jsx-*` OSC frame codec (encoder + decoder + streaming parser)
//   * a small `use_state`-style commit hook for ergonomic host integration
//   * `Reconciler::commit()` driving mount/update/unmount frame emission
//
// Out of scope (TODO Phase 4.2 follow-up): the React-fiber scheduling
// machinery itself. The TS source delegates to the external `react-reconciler`
// package; porting React's concurrent fiber engine to Rust is a multi-thousand
// line project that belongs to a dedicated `aphrody-jsx-react-vm` crate.
// What we ship here is everything the TS HostConfig owned plus enough
// host-side state to drive a renderer end-to-end from a Rust caller.

#![cfg_attr(not(test), warn(missing_docs))]
#![doc = "Cross-platform host-side React reconciler primitives for aphrody-terminal."]

extern crate alloc;

pub mod element;
pub mod hooks;
pub mod m3_tokens;
pub mod osc;
pub mod reconciler;

#[cfg(target_arch = "wasm32")]
pub mod wasm;

pub use element::{
    append_child, create_element, create_text_instance, detach, diff_props, freshen_ids, next_id,
    normalize_props, serialize, ElementKind, HostNode, JsonNode, JsonPatchOp, PropValue,
    TextNodeMarker,
};
pub use hooks::{Effect, HookState, UseState};
pub use osc::{
    decode_frame, encode_frame, FocusFields, InputFields, MountFields, OscFrame, OscFrameParser,
    OscOpcode, OscSink, UnmountFields, UpdateFields, VecSink, WindowSizeFields,
};
pub use reconciler::{Reconciler, ReconcilerError};

/// Library version (mirrors `Cargo.toml`).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
