// SPDX-License-Identifier: Apache-2.0
//! `bxc-engine` library — production HTTP API + CDP client + scrapers.
//!
//! ## Layout
//!
//! | Module    | Role                                                              |
//! |-----------|-------------------------------------------------------------------|
//! | [`scraper`] | Pure-HTTP recon / detect / scrape (reqwest + scraper).          |
//! | [`cdp`]     | Chrome DevTools Protocol WebSocket client (Linux/Win only).     |
//! | [`daemon`]  | Axum server on `127.0.0.1:8765` (Linux/Win only).               |
//!
//! The daemon exposes the JSON API contract consumed by
//! `crates/cli/src/scrape.rs::ScrapeClient` — i.e. exact route match
//! with the bxc Bun `packages/bxc/src/cli/api.ts`:
//!
//! * `GET  /healthz`
//! * `GET|POST /api/recon`
//! * `GET|POST /api/detect`
//! * `GET|POST /api/scrape`
//! * `GET  /api/tokens`         (M3 design-token harvester — Rust addition)
//!
//! ## Cross-platform contract (CLAUDE.md §0)
//!
//! * Linux x86_64 #1 — first-class.
//! * Windows x86_64 #2 — first-class.
//! * wasm32 #3 — scraper compiles (reqwest with the workspace features is
//!   wasm-friendly); daemon + cdp are `cfg(not(target_arch = "wasm32"))`.

pub mod scraper;

#[cfg(not(target_arch = "wasm32"))]
pub mod cdp;

#[cfg(not(target_arch = "wasm32"))]
pub mod daemon;

/// Crate version, exposed for `/healthz` + `X-Bxc-Version` header.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
