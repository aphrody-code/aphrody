// SPDX-License-Identifier: Apache-2.0
//! `notebooklm` — pure-Rust client for the Google NotebookLM Boq RPC surface.
//!
//! This crate ports the public RPC catalogue + envelope codec from three
//! permissively-licensed references (icebear0828/notebooklm-client,
//! teng-lin/notebooklm-py, jacob-bd/notebooklm-mcp-cli) into a single Rust
//! crate that fits the aphrody workspace: no browser automation in-process,
//! no Python/Node bridge, no CDP fallback.
//!
//! ## Modules
//!
//! * [`rpc_ids`]   — verbatim catalogue of the 49 Boq RPC identifiers, the
//!                   nine `ARTIFACT_TYPE_*` discriminants and the five URL
//!                   constants. Source of truth for every call below.
//! * [`error`]     — typed [`NotebookError`] / [`Result`] surface.
//! * [`types`]     — strongly-typed data model ([`Notebook`], [`Source`],
//!                   [`Artifact`], [`ChatReply`], [`AccountInfo`], …).
//! * [`auth`]      — [`Auth`] enum (cookie jar OR OAuth Bearer) + env loader
//!                   + Cookie-Editor JSON importer.
//! * [`boq`]       — `f.req` encoder + XSSI-prefix stripper + chunk parser.
//! * [`transport`] — pure-HTTP [`HttpTransport`] (reqwest, Chrome UA, system
//!                   proxy honoured, no curl-impersonate).
//! * [`client`]    — [`NotebookClient`] façade; every domain module hangs
//!                   `impl NotebookClient` blocks off it.
//! * [`notebooks`] — list / get / create / rename / delete + remove_recently_viewed.
//! * [`sources`]   — add_url / add_text / add_file / delete / refresh /
//!                   update + get_summary / get_content.
//! * [`chat`]      — list / delete threads + send_message.
//! * [`artifacts`] — generate / poll / list / rename / delete / get_interactive_html
//!                   / export / share.
//! * [`research`]  — create_web_search (fast/deep) + poll + import.
//!
//! ## Quick start
//!
//! ```no_run
//! use notebooklm::{Auth, NotebookClient, SessionTokens};
//!
//! # async fn run() -> notebooklm::Result<()> {
//! let auth = Auth::from_env()?;
//! let tokens = SessionTokens::default(); // populate from page bootstrap
//! let client = NotebookClient::new(auth, tokens)?;
//! let notebooks = client.list_notebooks().await?;
//! for nb in notebooks {
//!     println!("{} — {}", nb.id, nb.title);
//! }
//! # Ok(())
//! # }
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]

pub mod artifacts;
pub mod auth;
pub mod boq;
pub mod chat;
pub mod client;
pub mod error;
pub mod notebooks;
pub mod research;
pub mod rpc_ids;
pub mod sources;
pub mod transport;
pub mod types;

pub use auth::{Auth, CookieJar, SessionCookie};
pub use client::NotebookClient;
pub use error::{NotebookError, Result};
pub use research::ResearchMode;
pub use transport::{HttpTransport, SessionTokens};
pub use types::{
    AccountInfo, Artifact, ArtifactKind, ChatReply, ChatThread, Notebook, ResearchHit, Source,
    SourceKind,
};

/// Curated re-exports for downstream agents that want a single `use`.
pub mod prelude {
    pub use crate::auth::{Auth, CookieJar, SessionCookie};
    pub use crate::client::NotebookClient;
    pub use crate::error::{NotebookError, Result};
    pub use crate::research::ResearchMode;
    pub use crate::transport::{HttpTransport, SessionTokens};
    pub use crate::types::{
        AccountInfo, Artifact, ArtifactKind, ChatReply, ChatThread, Notebook, ResearchHit,
        Source, SourceKind,
    };
}
