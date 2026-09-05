// SPDX-License-Identifier: Apache-2.0
// Copyright AGNTCY Contributors (https://github.com/agntcy)
//
// Workspace path resolution. Ports `setTargetDir` from
// `packages/gemini-cli/packages/a2a-server/src/config/config.ts`.
//
// Priority (highest first):
//   1. explicit override passed to the function
//   2. env var `CODER_AGENT_WORKSPACE_PATH`
//   3. process cwd (unchanged)
//
// Unlike the TS reference, we DO NOT call `process.chdir` — modifying
// the working directory of a multi-threaded server is racy and breaks
// concurrent task execution. Callers should treat the returned path
// as "the workspace the next operation should target" and pass it
// down explicitly.

use std::path::{Path, PathBuf};

/// Env var used by gemini-cli A2A reference to override the workspace
/// directory.
pub const WORKSPACE_PATH_ENV: &str = "CODER_AGENT_WORKSPACE_PATH";

/// Resolve the workspace directory. See module docs for precedence.
///
/// Returns the original cwd if no override resolves successfully.
pub fn resolve_workspace(explicit: Option<&Path>) -> PathBuf {
    if let Some(p) = explicit {
        if let Ok(canon) = std::fs::canonicalize(p) {
            return canon;
        }
        return p.to_path_buf();
    }
    if let Some(env) = std::env::var_os(WORKSPACE_PATH_ENV) {
        let raw = PathBuf::from(env);
        if let Ok(canon) = std::fs::canonicalize(&raw) {
            return canon;
        }
        return raw;
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// True when an override is configured (explicit or via env). Useful
/// for HTTP handlers that need to short-circuit `/executeCommand`
/// when a command requires a workspace and none is configured —
/// mirrors the TS check `if (commandToExecute?.requiresWorkspace) { ... }`
pub fn workspace_override_configured() -> bool {
    std::env::var_os(WORKSPACE_PATH_ENV).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_explicit_takes_precedence() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path();
        let resolved = resolve_workspace(Some(p));
        assert_eq!(
            std::fs::canonicalize(&resolved).unwrap(),
            std::fs::canonicalize(p).unwrap()
        );
    }

    #[test]
    fn test_falls_back_to_cwd_when_no_override() {
        // SAFETY: env mutation in tests; this binary is single-threaded
        // for unit tests by default.
        unsafe {
            std::env::remove_var(WORKSPACE_PATH_ENV);
        }
        let p = resolve_workspace(None);
        assert_eq!(p, std::env::current_dir().unwrap());
    }

    #[test]
    fn test_workspace_override_configured_reflects_env() {
        // SAFETY: env mutation in tests.
        unsafe {
            std::env::set_var(WORKSPACE_PATH_ENV, "/tmp");
        }
        assert!(workspace_override_configured());
        unsafe {
            std::env::remove_var(WORKSPACE_PATH_ENV);
        }
        assert!(!workspace_override_configured());
    }
}
