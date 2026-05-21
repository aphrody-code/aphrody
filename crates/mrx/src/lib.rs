// SPDX-License-Identifier: Apache-2.0
//! mrx — cross-platform, serverless-friendly monorepo mapper + watcher.
//!
//! Unified crate merging the former `mrx-core`, `mrx-detect`, `mrx-audit`,
//! and `mrx-watch` crates into a single library with a `[[bin]]` entry point.
//!
//! # Modules
//!
//! - [`core`] — shared types serialised to `path.json` and `monorepo-map.json`.
//! - [`detect`] — recognise the shape of a JS/Rust monorepo root.
//! - [`audit`] — parallel monorepo audit engine (ignore + rayon + blake3).
//! - [`watch`] — long-running FS watcher (notify v8 + debouncer).

#![forbid(unsafe_code)]

pub mod core;
pub mod detect;
pub mod audit;
pub mod watch;

// Re-export the most common types at crate root for convenience.
pub use crate::core::*;
