// SPDX-License-Identifier: Apache-2.0
//! `aphrody-design` — unified design infrastructure crate.
//!
//! Merges the former `aphrody-design-sidecar` (streaming artifact pipeline)
//! and `aphrody-design-daemon` (SQLite project store + design-systems registry)
//! into a single crate with two submodules.
//!
//! # Modules
//!
//! - [`sidecar`] — streaming `<artifact>` block parser, per-format normalizers,
//!   multi-chunk merge, SHA-256 manifest digest, and end-to-end pipeline.
//! - [`daemon`] — SQLite-backed project store, conversation/message/tab CRUD,
//!   and design-systems registry loader (`DESIGN.md` + tokens.css + components).

#![forbid(unsafe_code)]

pub mod sidecar;
pub mod daemon;

pub use m3_tokens as tokens;
pub use aphrody_icons as icons;
