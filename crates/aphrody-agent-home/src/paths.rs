// SPDX-License-Identifier: Apache-2.0
//! Workspace path resolution (AH-10 profile support).
//!
//! Mirrors `crates/cli/src/oc_cmd.rs::home_dir` exactly so the agent home and
//! the CLI agree on the on-disk layout, then layers profile / multi-agent
//! resolution on top:
//!
//! * `$APHRODY_WORKSPACE` — explicit override, wins over everything.
//! * `$APHRODY_PROFILE` — selects `~/.aphrody/workspace-<profile>`.
//! * agent id — selects `~/.aphrody/workspace-<profile>` per agent when set
//!   via [`HomeOptions::agent_id`].
//! * default — `~/.aphrody/workspace`.

use std::path::PathBuf;

use crate::HomeError;

/// Environment variable that overrides the home root (matches `oc_cmd.rs`).
pub const ENV_APHRODY_HOME: &str = "APHRODY_HOME";
/// Environment variable that overrides the workspace directory outright.
pub const ENV_APHRODY_WORKSPACE: &str = "APHRODY_WORKSPACE";
/// Environment variable that selects a named profile.
pub const ENV_APHRODY_PROFILE: &str = "APHRODY_PROFILE";

/// Resolve the home directory: `$APHRODY_HOME` overrides, else the platform
/// home (`USERPROFILE` on Windows, `HOME` elsewhere). Identical contract to
/// `oc_cmd.rs::home_dir`.
///
/// # Errors
/// [`HomeError::NoHome`] when no home variable is set.
pub fn home_dir() -> Result<PathBuf, HomeError> {
    if let Ok(h) = std::env::var(ENV_APHRODY_HOME) {
        if !h.is_empty() {
            return Ok(PathBuf::from(h));
        }
    }
    if cfg!(windows) {
        if let Ok(h) = std::env::var("USERPROFILE") {
            if !h.is_empty() {
                return Ok(PathBuf::from(h));
            }
        }
    }
    if let Ok(h) = std::env::var("HOME") {
        if !h.is_empty() {
            return Ok(PathBuf::from(h));
        }
    }
    Err(HomeError::NoHome)
}

/// Root aphrody state directory (`$APHRODY_HOME` overrides, else `~/.aphrody`).
///
/// # Errors
/// Propagates [`home_dir`] failure.
pub fn state_dir() -> Result<PathBuf, HomeError> {
    Ok(home_dir()?.join(".aphrody"))
}

/// Sanitise a profile / agent token into a filesystem-safe suffix: keep
/// ASCII alphanumerics, `-`, `_`; map everything else to `-`; lowercase.
/// Guarantees the resolved workspace can never escape `~/.aphrody/`.
#[must_use]
pub fn sanitize_profile(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.trim().chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('-');
        }
    }
    // Collapse leading/trailing separators that would make an ugly dir name.
    out.trim_matches('-').to_string()
}

/// Resolve the workspace directory for an optional profile + agent id.
///
/// Resolution order (first match wins):
/// 1. `$APHRODY_WORKSPACE` (verbatim override).
/// 2. explicit `agent_id` -> `~/.aphrody/workspace-<agent>`.
/// 3. `profile` arg or `$APHRODY_PROFILE` -> `~/.aphrody/workspace-<profile>`.
/// 4. default -> `~/.aphrody/workspace`.
///
/// # Errors
/// Propagates [`home_dir`] failure when no override is set.
pub fn resolve_workspace(
    profile: Option<&str>,
    agent_id: Option<&str>,
) -> Result<PathBuf, HomeError> {
    if let Ok(ws) = std::env::var(ENV_APHRODY_WORKSPACE) {
        if !ws.is_empty() {
            return Ok(PathBuf::from(ws));
        }
    }
    let base = state_dir()?;
    if let Some(agent) = agent_id {
        let s = sanitize_profile(agent);
        if !s.is_empty() {
            return Ok(base.join(format!("workspace-{s}")));
        }
    }
    let prof = profile
        .map(str::to_string)
        .or_else(|| std::env::var(ENV_APHRODY_PROFILE).ok())
        .map(|p| sanitize_profile(&p))
        .filter(|p| !p.is_empty());
    match prof {
        Some(p) => Ok(base.join(format!("workspace-{p}"))),
        None => Ok(base.join("workspace")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitize_strips_path_separators() {
        // Leading separators (from `..`, `/`) are trimmed, so a traversal
        // attempt collapses to a safe, contained suffix.
        assert_eq!(sanitize_profile("../escape"), "escape");
        assert_eq!(sanitize_profile("a/b\\c"), "a-b-c");
        assert_eq!(sanitize_profile("Prod Agent"), "prod-agent");
        assert_eq!(sanitize_profile("ok_name-1"), "ok_name-1");
    }

    #[test]
    fn sanitize_never_yields_path_traversal() {
        // No sanitised profile can contain a separator or `..`, so
        // `workspace-<profile>` always stays under `~/.aphrody/`.
        for raw in ["../../etc", "a/../../b", "..\\..\\win", "/abs/path"] {
            let s = sanitize_profile(raw);
            assert!(!s.contains('/') && !s.contains('\\') && !s.contains(".."), "leaked: {s}");
        }
    }

    #[test]
    fn sanitize_trims_separator_edges() {
        assert_eq!(sanitize_profile("...x..."), "x");
        assert_eq!(sanitize_profile("///"), "");
    }
}
