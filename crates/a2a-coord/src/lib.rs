// SPDX-License-Identifier: Apache-2.0
//! File-based A2A coordination + HTTP bridge for aphrody.
//!
//! - [`AiManifest`] — rich `ai.json` (peers: Claude Code, Grok, agy, bxc)
//! - [`Envelope`] — JSONL mailbox format (v1, compatible with `a2a-ui`)
//! - [`PeerInvoker`] — headless CLI dispatch with verified Grok flags
//! - [`listener::serve`] — `127.0.0.1:8788` with `/ping`, `/msg`, A2A JSON-RPC, agent card

#![forbid(unsafe_code)]

pub mod card;
pub mod envelope;
pub mod executor;
pub mod listener;
pub mod mailbox;
pub mod manifest;
pub mod peer;

pub use card::agent_card_from_manifest;
pub use envelope::{Envelope, EnvelopeKind};
pub use listener::{ListenerConfig, serve};
pub use manifest::{AiManifest, CoordConfig, PeerDef, find_repo_root};
pub use executor::CoordPeerExecutor;
pub use peer::{PeerId, PeerInvokeResult, PeerInvoker};