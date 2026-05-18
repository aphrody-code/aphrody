// SPDX-License-Identifier: Apache-2.0
//! Backend implementations for the aphrody-browser OSC dispatcher.
//!
//! Three backends are provided:
//!
//! - [`bxc`] — spawns the `bxc` CLI as a subprocess and communicates over stdin/stdout using
//!   newline-delimited JSON-RPC.
//! - [`agent_browser`] — spawns `agent-browser` (vercel-labs) using the same JSON-RPC stdio
//!   transport; supports full Chromium via CDP.
//! - [`edge`] — invokes `msedge --headless=new --dump-dom` for a DOM snapshot; read-only fallback,
//!   unsupported on Linux/WASM.

pub mod agent_browser;
pub mod bxc;
pub mod edge;
