// SPDX-License-Identifier: Apache-2.0
//! # aphrody-sdk — public Rust SDK for embedding aphrody in third-party apps.
//!
//! This crate is the **stable programmatic surface** for consumers that want
//! to drive an aphrody agent from their own Rust program — without going
//! through the `aphrody` CLI binary. It is a port of
//! `packages/gemini-cli/packages/sdk/src/*.ts` (~300 LoC TypeScript) onto the
//! aphrody crate ecosystem.
//!
//! ## Anatomy
//!
//! The SDK is a thin **facade** over a curated subset of the workspace:
//!
//! | Module                | Wires                                                              |
//! |-----------------------|--------------------------------------------------------------------|
//! | [`agent`]             | [`gemini_runtime`] adapter + stub backend                          |
//! | [`session`]           | re-exports + manager around [`aphrody_session`]                    |
//! | [`tool`]              | re-exports + builder on top of [`aphrody_tools`]                   |
//! | [`skills`]            | re-exports + discovery façade for [`aphrody_skills_runtime`]       |
//! | [`fs`]                | sandboxed filesystem helpers (native + WASM stub)                  |
//! | [`shell`]             | sandboxed shell helpers (native + WASM stub)                       |
//! | [`types`]             | DTOs (`SdkConfig`, `SdkError`, `SessionContext`, `TurnResult`)     |
//!
//! Every public type carries a `PUBLIC API — semver stable from v1.0` marker
//! in its documentation. Bumping or breaking such a type is a major-version
//! event for the crate.
//!
//! ## Cross-platform
//!
//! The SDK compiles on the three priority targets defined in `CLAUDE.md` §0:
//!
//! 1. `x86_64-unknown-linux-gnu` (full functionality).
//! 2. `x86_64-pc-windows-msvc` (full functionality).
//! 3. `wasm32-unknown-unknown` (filesystem + shell return
//!    [`SdkError::Unsupported`]; everything else works).
//!
//! ## Quickstart
//!
//! ```no_run
//! use aphrody_sdk::{AphrodySdk, SdkConfig};
//!
//! # async fn run() -> Result<(), aphrody_sdk::SdkError> {
//! let sdk = AphrodySdk::new(SdkConfig::default()).await?;
//! let reply = sdk.agent().single_turn("hello").await?;
//! println!("{reply}");
//! # Ok(()) }
//! ```
//!
//! ## A note on `aphrody-chat`
//!
//! `aphrody-chat` (the heavyweight orchestrator that wires hooks, permissions,
//! cost tracking, etc.) is being authored in parallel as part of the same
//! mega-batch as this crate. To keep the SDK shippable today, we wire the
//! lighter-weight `gemini-runtime` backend directly here and ship a
//! deterministic `StubBackend` for tests / quickstart. When `aphrody-chat`
//! lands, the SDK will gain an `AgentBuilder::with_chat_loop(...)` constructor
//! that delegates to its `ChatLoop::single_turn` — without breaking the
//! public types declared here.

#![cfg_attr(not(test), forbid(unsafe_code))]
#![doc(html_root_url = "https://docs.rs/aphrody-sdk")]

pub mod agent;
pub mod fs;
pub mod session;
pub mod shell;
pub mod skills;
pub mod tool;
pub mod types;

use std::sync::Arc;

// ---------------------------------------------------------------------------
// Top-level re-exports — the surface a consumer reaches for first.
// ---------------------------------------------------------------------------

pub use crate::agent::{Agent, Backend, GeminiBackend, StreamChunk, StubBackend};
pub use crate::fs::{AgentFilesystem, SandboxFs};
pub use crate::session::{
    JsonlFileStore, Session, SessionId, SessionManager, SessionStore, Turn, TurnRole,
};
pub use crate::shell::{AgentShell, AgentShellOptions, AgentShellResult, SandboxShell};
pub use crate::skills::{skill_dir, SkillReference, Skills};
pub use crate::tool::{tool, Tool, ToolAction, ToolDescriptor, ToolRegistry};
pub use crate::types::{SdkConfig, SdkError, SdkResult, SessionContext, TurnResult};

// ---------------------------------------------------------------------------
// AphrodySdk — top-level facade
// ---------------------------------------------------------------------------

/// Top-level façade. Constructed once per process via [`AphrodySdk::new`].
///
/// Holds shared `Arc` references to every building block; cloning the SDK
/// is cheap (`Arc` bumps only) so it can be handed across async tasks
/// freely.
///
/// PUBLIC API — semver stable from v1.0.
#[derive(Clone)]
pub struct AphrodySdk {
    config: Arc<SdkConfig>,
    agent: Agent,
    session_manager: SessionManager,
    tool_registry: Arc<ToolRegistry>,
    skills: Arc<Skills>,
    fs: Arc<dyn AgentFilesystem>,
    shell: Arc<dyn AgentShell>,
}

impl std::fmt::Debug for AphrodySdk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AphrodySdk")
            .field("model", &self.config.model)
            .field("tool_count", &self.tool_registry.len())
            .field("skill_count", &self.skills.len())
            .finish_non_exhaustive()
    }
}

impl AphrodySdk {
    /// Construct an SDK instance from a configuration. The backend defaults to
    /// [`StubBackend::echo()`] — no external `gemini` CLI is probed or spawned
    /// at startup. Set `APHRODY_GEMINI_CLI=1` to opt into the external
    /// [`GeminiBackend`] (used only when the `gemini` binary is on `PATH`).
    ///
    /// # Errors
    /// * [`SdkError::InvalidConfig`] when [`SdkConfig::validate`] fails.
    /// * [`SdkError::Io`] when the configured `cwd` cannot be canonicalized.
    pub async fn new(config: SdkConfig) -> SdkResult<Self> {
        config.validate()?;

        let cwd = config
            .cwd
            .clone()
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(|| std::path::PathBuf::from("."));

        // Backend selection. The SDK no longer probes / bootstraps the external
        // `gemini` CLI at construction by default — that eager `gemini --version`
        // subprocess spawn was a startup dependency on a tool the keyless paths
        // now replace (and it cost a process launch on every `new()`). The
        // external CLI backend is opt-in via `APHRODY_GEMINI_CLI` (truthy) for
        // hosts that explicitly want it; otherwise the universal in-process
        // [`StubBackend`] is used with zero subprocess launches. Consumers that
        // need the native keyless transport drive `aphrody-chat`'s
        // `GeminiWebBackend` directly.
        let want_cli = std::env::var("APHRODY_GEMINI_CLI")
            .ok()
            .is_some_and(|v| matches!(v.trim(), "1" | "true" | "yes" | "on"));
        let backend: Arc<dyn Backend> = if want_cli {
            match gemini_runtime::detect().await {
                Ok(_) => {
                    tracing::debug!(
                        target: "aphrody_sdk",
                        "GeminiBackend selected (APHRODY_GEMINI_CLI opt-in; gemini CLI present)",
                    );
                    Arc::new(GeminiBackend)
                }
                Err(e) => {
                    tracing::debug!(
                        target: "aphrody_sdk",
                        error = %e,
                        "APHRODY_GEMINI_CLI set but gemini CLI absent; falling back to StubBackend",
                    );
                    Arc::new(StubBackend::echo())
                }
            }
        } else {
            tracing::debug!(
                target: "aphrody_sdk",
                "StubBackend selected (no external gemini-cli probe at startup; set \
                 APHRODY_GEMINI_CLI=1 to opt in)",
            );
            Arc::new(StubBackend::echo())
        };

        let cfg_arc = Arc::new(config.clone());
        let agent = Agent::with_backend(Arc::clone(&cfg_arc), backend);
        let session_manager = SessionManager::new(
            config.session_dir.clone(),
            serde_json::json!({
                "model": config.model,
                "sdk_version": env!("CARGO_PKG_VERSION"),
            }),
        );

        let mut skills = Skills::new();
        skills.add_dirs(&config.skill_dirs)?;

        let tool_registry = ToolRegistry::new();

        let fs: Arc<dyn AgentFilesystem> = Arc::new(SandboxFs::new(cwd.clone()));
        let shell: Arc<dyn AgentShell> = Arc::new(SandboxShell::new(cwd));

        Ok(Self {
            config: cfg_arc,
            agent,
            session_manager,
            tool_registry: Arc::new(tool_registry),
            skills: Arc::new(skills),
            fs,
            shell,
        })
    }

    /// Borrow the configured agent.
    #[must_use]
    pub fn agent(&self) -> &Agent {
        &self.agent
    }

    /// Borrow the session manager.
    #[must_use]
    pub fn session_manager(&self) -> &SessionManager {
        &self.session_manager
    }

    /// Borrow the tool registry.
    #[must_use]
    pub fn tool_registry(&self) -> &ToolRegistry {
        &self.tool_registry
    }

    /// Borrow the discovered skill set.
    #[must_use]
    pub fn skills(&self) -> &Skills {
        &self.skills
    }

    /// Borrow the configured filesystem facade.
    #[must_use]
    pub fn fs(&self) -> &Arc<dyn AgentFilesystem> {
        &self.fs
    }

    /// Borrow the configured shell facade.
    #[must_use]
    pub fn shell(&self) -> &Arc<dyn AgentShell> {
        &self.shell
    }

    /// Borrow the active configuration snapshot.
    #[must_use]
    pub fn config(&self) -> &SdkConfig {
        &self.config
    }

    /// Crate version stamped in at build time.
    pub const VERSION: &'static str = env!("CARGO_PKG_VERSION");
}

/// Module-level alias for `AphrodySdk::VERSION`.
pub const VERSION: &str = AphrodySdk::VERSION;

// ---------------------------------------------------------------------------
// In-crate smoke tests (additional integration coverage lives under tests/).
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn new_with_default_config_succeeds() {
        let sdk = AphrodySdk::new(SdkConfig::default())
            .await
            .expect("default config must construct an sdk");
        assert!(sdk.tool_registry().is_empty());
        assert!(sdk.skills().is_empty());
        assert!(!AphrodySdk::VERSION.is_empty());
    }

    #[tokio::test]
    async fn agent_round_trips_via_facade() {
        // Force the StubBackend path by skipping gemini detection — the SDK
        // already falls back to it on missing binary, so this matches the
        // default test environment.
        let sdk = AphrodySdk::new(SdkConfig::default()).await.unwrap();
        let reply = sdk.agent().single_turn("ping").await.unwrap();
        // StubBackend::echo prefixes "echo: ".
        assert!(reply.contains("ping"));
    }
}
