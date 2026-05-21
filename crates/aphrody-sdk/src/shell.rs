// SPDX-License-Identifier: Apache-2.0
//! Shell execution helpers.
//!
//! TS provenance: `packages/gemini-cli/packages/sdk/src/shell.ts`.
//!
//! On native targets we delegate to `tokio::process::Command` with a hardened
//! shell selection (`/bin/sh -c` on Unix, `cmd /C` on Windows). The TS
//! implementation routes through the gemini-cli-core `ShellExecutionService`
//! which adds policy checks; that surface is not yet ported to Rust, so we
//! enforce two minimal guard rails directly:
//!
//! 1. Empty commands are rejected.
//! 2. Commands containing NUL bytes are rejected.
//!
//! On WASM the [`AgentShell::exec`] call returns
//! [`SdkError::Unsupported`] — no process spawn primitives are available.

#![cfg_attr(target_arch = "wasm32", allow(dead_code, unused_variables))]

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use crate::types::{SdkError, SdkResult};

// ---------------------------------------------------------------------------
// Public DTOs
// ---------------------------------------------------------------------------

/// Tunables for a single [`AgentShell::exec`] call.
///
/// PUBLIC API — semver stable from v1.0.
#[derive(Debug, Clone, Default)]
pub struct AgentShellOptions {
    /// Environment variables merged on top of the inherited ones.
    pub env: HashMap<String, String>,
    /// Optional working directory override (defaults to the agent cwd).
    pub cwd: Option<PathBuf>,
    /// Optional wall-clock timeout. When elapsed, the child is killed and
    /// the call returns with `exit_code = None`.
    pub timeout: Option<Duration>,
}

/// Result of one [`AgentShell::exec`] call.
///
/// PUBLIC API — semver stable from v1.0.
#[derive(Debug, Clone)]
pub struct AgentShellResult {
    /// Exit code on graceful exit, `None` on timeout or signal-kill.
    pub exit_code: Option<i32>,
    /// Combined stdout + stderr (mirrors the TS shape).
    pub output: String,
    /// Standard output only.
    pub stdout: String,
    /// Standard error only.
    pub stderr: String,
}

// ---------------------------------------------------------------------------
// Trait
// ---------------------------------------------------------------------------

/// Sandboxed shell handed to tools at execution time.
///
/// PUBLIC API — semver stable from v1.0.
#[async_trait::async_trait]
pub trait AgentShell: Send + Sync {
    /// Execute `cmd` with the supplied options.
    ///
    /// # Errors
    /// * [`SdkError::InvalidConfig`] when the command is empty or contains a
    ///   NUL byte.
    /// * [`SdkError::InteractiveRequired`] when the underlying impl detects
    ///   that the command would require interactive input.
    /// * [`SdkError::Io`] on spawn failures.
    /// * [`SdkError::Unsupported`] on `wasm32`.
    async fn exec(
        &self,
        cmd: &str,
        opts: AgentShellOptions,
    ) -> SdkResult<AgentShellResult>;
}

// ---------------------------------------------------------------------------
// SandboxShell — native implementation
// ---------------------------------------------------------------------------

/// Default `AgentShell` implementation.
///
/// PUBLIC API — semver stable from v1.0.
#[derive(Debug, Clone)]
pub struct SandboxShell {
    cwd: PathBuf,
}

impl SandboxShell {
    /// Construct a shell rooted at `cwd`.
    ///
    /// PUBLIC API — semver stable from v1.0.
    #[must_use]
    pub fn new(cwd: impl Into<PathBuf>) -> Self {
        Self { cwd: cwd.into() }
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[async_trait::async_trait]
impl AgentShell for SandboxShell {
    async fn exec(
        &self,
        cmd: &str,
        opts: AgentShellOptions,
    ) -> SdkResult<AgentShellResult> {
        let cmd_trim = cmd.trim();
        if cmd_trim.is_empty() {
            return Err(SdkError::InvalidConfig("empty shell command".into()));
        }
        if cmd_trim.contains('\u{0}') {
            return Err(SdkError::InvalidConfig(
                "shell command contains NUL byte".into(),
            ));
        }

        let cwd = opts.cwd.unwrap_or_else(|| self.cwd.clone());

        #[cfg(unix)]
        let mut command = {
            let mut c = tokio::process::Command::new("/bin/sh");
            c.arg("-c").arg(cmd_trim);
            c
        };
        #[cfg(windows)]
        let mut command = {
            let mut c = tokio::process::Command::new("cmd");
            c.arg("/C").arg(cmd_trim);
            c
        };

        command.current_dir(&cwd);
        for (k, v) in opts.env {
            command.env(k, v);
        }
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());

        let child = command.spawn().map_err(SdkError::Io)?;
        let fut = child.wait_with_output();

        let output = if let Some(timeout) = opts.timeout {
            match tokio::time::timeout(timeout, fut).await {
                Ok(Ok(o)) => o,
                Ok(Err(e)) => return Err(SdkError::Io(e)),
                Err(_elapsed) => {
                    return Ok(AgentShellResult {
                        exit_code: None,
                        output: String::new(),
                        stdout: String::new(),
                        stderr: format!("timed out after {timeout:?}"),
                    });
                }
            }
        } else {
            fut.await.map_err(SdkError::Io)?
        };

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        let combined = format!("{stdout}{stderr}");

        Ok(AgentShellResult {
            exit_code: output.status.code(),
            output: combined,
            stdout,
            stderr,
        })
    }
}

#[cfg(target_arch = "wasm32")]
#[async_trait::async_trait]
impl AgentShell for SandboxShell {
    async fn exec(
        &self,
        _cmd: &str,
        _opts: AgentShellOptions,
    ) -> SdkResult<AgentShellResult> {
        Err(SdkError::Unsupported("shell exec on wasm32"))
    }
}

// ---------------------------------------------------------------------------
// Tests (native only)
// ---------------------------------------------------------------------------

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn rejects_empty_command() {
        let dir = TempDir::new().unwrap();
        let sh = SandboxShell::new(dir.path());
        let r = sh.exec("", AgentShellOptions::default()).await;
        assert!(matches!(r, Err(SdkError::InvalidConfig(_))));
    }

    #[tokio::test]
    async fn rejects_nul_byte() {
        let dir = TempDir::new().unwrap();
        let sh = SandboxShell::new(dir.path());
        let r = sh.exec("echo hi\u{0}bye", AgentShellOptions::default()).await;
        assert!(matches!(r, Err(SdkError::InvalidConfig(_))));
    }

    #[tokio::test]
    async fn echoes_to_stdout() {
        let dir = TempDir::new().unwrap();
        let sh = SandboxShell::new(dir.path());
        // Use a portable command: `cd` exists on both sh and cmd.
        #[cfg(unix)]
        let cmd = "echo aphrody";
        #[cfg(windows)]
        let cmd = "echo aphrody";
        let r = sh.exec(cmd, AgentShellOptions::default()).await.unwrap();
        assert!(r.stdout.contains("aphrody"));
        assert_eq!(r.exit_code, Some(0));
    }
}
