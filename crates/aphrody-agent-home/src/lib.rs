// SPDX-License-Identifier: Apache-2.0
//! # aphrody-agent-home — the agent's persistent home.
//!
//! This crate centralises the **persistent identity of the agent**: its
//! workspace, its soul (persona), its identity (name / vibe / emoji), the user
//! it serves, its local tool conventions, its memory, and the assembly of all
//! of that into a bounded system prompt. It is the Rust counterpart of
//! `var/openclaw/src/agents/{workspace,bootstrap-budget,identity}.ts`, unified
//! and hardened.
//!
//! ## What it ports, and how it pushes past openclaw
//!
//! openclaw reads workspace files with `readFileSync` into a per-process
//! `Map<path, {content, identity}>`, single-threaded, recopying a `String`
//! per session. aphrody:
//!
//! * **maps files zero-copy** ([`mmap::MappedBytes`], `Arc<Mmap>` backed by the
//!   OS page cache) so bootstrap files are shared by every session and thread;
//! * **content-addresses** the cache ([`cache::FileCache`], blake3 + a
//!   persisted `(dev,ino,size,mtime)` identity) so identical files dedup and a
//!   restart skips re-hashing;
//! * **truncates streaming** ([`budget::BudgetWriter`]) on grapheme boundaries
//!   in one pass, emitting the same stats + signature as
//!   `bootstrap-budget.ts`;
//! * **hot-reloads atomically** ([`watch::HomeWatcher`], `notify` + `arc-swap`)
//!   so SOUL/IDENTITY/USER edits swap the live home without a restart;
//! * is **`Send + Sync` by construction**, shareable across the tokio runtime;
//! * **validates at parse** ([`soul::Soul`] is typed, with anti-pattern lints).
//!
//! ## Cross-platform
//!
//! The typed model + budget + assembly compile everywhere (Linux #1, Windows
//! #2, wasm32 #3). The mmap path falls back to an in-memory read on wasm; the
//! `notify` watcher and the `gix` git backup (feature `git`) are host-only and
//! cfg-gated out of wasm with a real no-op / disabled path.
//!
//! ## Quick example
//!
//! ```no_run
//! use aphrody_agent_home::{AgentHome, HomeOptions, BootstrapBudget};
//!
//! # fn main() -> Result<(), aphrody_agent_home::HomeError> {
//! let home = AgentHome::open(HomeOptions::default())?;
//! let view = home.system_prompt(&BootstrapBudget::default());
//! let prompt = view.render();
//! assert!(prompt.contains("You are"));
//! # Ok(()) }
//! ```

// Crate-wide `unsafe` is denied (not `forbid`), because `forbid` is
// unoverridable and the single justified `memmap2::Mmap::map` call lives in
// `mmap.rs` behind a scoped, documented `#![allow(unsafe_code)]`. `deny`
// still hard-fails the build for any stray `unsafe` outside that one module.
#![deny(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)] // errors are documented on public fns that need it
#![allow(clippy::module_name_repetitions)]

pub mod assemble;
pub mod boot;
pub mod budget;
pub mod cache;
pub mod error;
pub mod filenames;
pub mod frontmatter;
pub mod guard;
pub mod heartbeat;
pub mod home;
pub mod identity;
pub mod mmap;
pub mod onboard;
pub mod paths;
pub mod soul;
pub mod tools;
pub mod user;

#[cfg(all(not(target_arch = "wasm32"), feature = "git"))]
pub mod git;

#[cfg(not(target_arch = "wasm32"))]
pub mod watch;

pub use crate::assemble::{PromptSection, SystemPromptView};
pub use crate::budget::{BootstrapBudget, BudgetWriter, TruncationReport};
pub use crate::cache::{CacheRecord, FileCache, FileIdentity, WorkspaceState};
pub use crate::error::HomeError;
pub use crate::filenames::BootstrapFile;
pub use crate::guard::WorkspaceGuard;
pub use crate::home::{AgentHome, HomeOptions};
pub use crate::onboard::{OnboardOptions, SeedReport};
pub use crate::identity::Identity;
pub use crate::mmap::MappedBytes;
pub use crate::soul::{Bluntness, Brevity, Humor, Soul};
pub use crate::tools::ToolsDoc;
pub use crate::user::UserProfile;

/// Crate version, surfaced through the CLI doctor / version introspection.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
