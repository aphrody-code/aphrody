// SPDX-License-Identifier: Apache-2.0
//! `aphrody-guard` — safety guardrails for the autonomous `aphrody` agent.
//!
//! aphrody runs with **no human in the loop** (cf. `CLAUDE.md` §0.1) while it
//! holds live credentials under `secrets/`. Two classes of guardrail live here:
//!
//! * [`harden`] — pre-`main` process hardening: block core dumps and ptrace
//!   attach, and strip the dynamic-loader injection vectors (`LD_*` / `DYLD_*`)
//!   from the environment before any work happens.
//! * [`command_safety`] — a conservative, cross-platform classifier that maps a
//!   tokenised shell command to [`Decision::Allow`] (provably read-only),
//!   [`Decision::Prompt`] (unknown — escalate / require approval), or
//!   [`Decision::Forbidden`] (known-destructive), so an agent can auto-approve
//!   only the commands it can prove are harmless.
//!
//! Design distilled from OpenAI Codex's `process-hardening` and `shell-command`
//! crates (Apache-2.0), re-implemented here with zero external parser
//! dependencies (no `tree-sitter`, no `starlark`) to stay lean and `cargo deny`
//! clean.

pub mod command_safety;
pub mod harden;

pub use command_safety::{
    Decision, classify_command, classify_command_enforced, is_dangerous_command,
    is_known_safe_command,
};
pub use harden::{pre_main_hardening, pre_main_hardening_if_enabled};

/// Environment variable that opts **into** all of aphrody's guardrails.
///
/// aphrody is designed to run fully autonomous with **no human in the loop**
/// (cf. `CLAUDE.md` §0.1), so every guardrail in this crate — process hardening
/// and command-safety enforcement alike — is **disabled by default**. Set this
/// to `1` / `true` / `on` / `yes` to turn them on.
pub const GUARD_ENV: &str = "APHRODY_GUARD";

/// Returns `true` only when [`GUARD_ENV`] is explicitly set to an affirmative
/// value. The default (unset / anything else) is `false`: guardrails off.
#[must_use]
pub fn guardrails_enabled() -> bool {
    matches!(
        std::env::var(GUARD_ENV)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "on" | "yes"
    )
}
