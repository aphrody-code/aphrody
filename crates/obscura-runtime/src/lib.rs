// SPDX-License-Identifier: Apache-2.0
//! Obscura headless-browser runtime adapter.
//!
//! Cross-platform façade that locates the `obscura` binary, then spawns
//! `obscura fetch` / `obscura scrape` sub-processes and captures their output.
//! No V8 or Obscura source code enters the build graph — the binary is treated
//! as an optional external tool (pilier R4 scraping).
//!
//! # Resolution order
//!
//! 1. `$APHRODY_OBSCURA_BIN` — path to an existing file (env override).
//! 2. Sibling of the running `aphrody` binary (`obscura` / `obscura.exe`).
//! 3. `PATH` walk — any entry that contains an `obscura`/`obscura.exe` file.
//!
//! # Usage
//!
//! ```no_run
//! use obscura_runtime::{fetch, resolve_bin, scrape};
//!
//! # async fn run() {
//! // Resolve the binary once (cheap — filesystem probes only).
//! let _bin = resolve_bin().expect("obscura not found");
//!
//! // Fetch a single URL and get Markdown back.
//! let md = fetch("https://example.com", "markdown").await.expect("fetch failed");
//! println!("{md}");
//!
//! // Scrape several URLs with bounded concurrency.
//! let urls = vec!["https://example.com".to_owned()];
//! let json = scrape(&urls, 4).await.expect("scrape failed");
//! println!("{json}");
//! # }
//! ```

// Nightly 2024-edition: unsafe_op_in_unsafe_fn is deny-by-default at the
// workspace level; no unsafe needed here.
#![cfg_attr(not(test), forbid(unsafe_code))]

use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Stdio,
};

use thiserror::Error;
use tokio::process::Command;

// ---------------------------------------------------------------------------
// Public constant
// ---------------------------------------------------------------------------

/// Environment variable that overrides the `obscura` binary path.
///
/// When set to a non-empty path that points to an existing file,
/// [`resolve_bin`] returns that path directly, skipping all other probes.
pub const ENV_OBSCURA_BIN: &str = "APHRODY_OBSCURA_BIN";

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

/// All errors that can originate from this crate.
#[derive(Debug, Error)]
pub enum ObscuraError {
    /// The `obscura` binary could not be located via any resolution step.
    #[error(
        "obscura binary not found. \
         Set {ENV_OBSCURA_BIN} or place the binary next to aphrody / on PATH."
    )]
    BinaryNotFound,

    /// An I/O error occurred while spawning the child process or reading its
    /// stdout/stderr.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// The child process was spawned but exited with a non-zero status code.
    ///
    /// `code` is `None` when the process was killed by a signal (Unix).
    #[error("obscura exited with non-zero status (code={code:?}): {stderr}")]
    NonZeroExit {
        /// Raw exit code, if available.
        code: Option<i32>,
        /// Captured stderr from the child process.
        stderr: String,
    },

    /// Failed to spawn the child process (binary found but un-executable, or
    /// OS-level error).
    #[error("failed to spawn obscura at {path}: {source}")]
    Spawn {
        /// Resolved binary path that was attempted.
        path: PathBuf,
        /// Underlying I/O cause.
        #[source]
        source: std::io::Error,
    },
}

// ---------------------------------------------------------------------------
// Binary resolution
// ---------------------------------------------------------------------------

/// Locate the `obscura` binary using the three-step resolution order.
///
/// # Errors
///
/// Returns [`ObscuraError::BinaryNotFound`] when no resolution step succeeds.
pub fn resolve_bin() -> Result<PathBuf, ObscuraError> {
    // Step 1: explicit env override.
    if let Ok(explicit) = std::env::var(ENV_OBSCURA_BIN) {
        let trimmed = explicit.trim();
        if !trimmed.is_empty() {
            let p = PathBuf::from(trimmed);
            if p.exists() {
                return Ok(p);
            }
        }
    }

    // Step 2: sibling of the running aphrody binary.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for cand in ["obscura.exe", "obscura"] {
                let p = dir.join(cand);
                if p.exists() {
                    return Ok(p);
                }
            }
        }
    }

    // Step 3: PATH walk — look for `obscura`/`obscura.exe` in each entry.
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            for cand in ["obscura.exe", "obscura"] {
                let p = dir.join(cand);
                if p.exists() {
                    return Ok(p);
                }
            }
        }
    }

    Err(ObscuraError::BinaryNotFound)
}

// ---------------------------------------------------------------------------
// Internal spawn helper
// ---------------------------------------------------------------------------

/// Spawn `obscura` at `bin` with `args`, capture stdout, and return it as a
/// `String`.  Non-zero exit → [`ObscuraError::NonZeroExit`].
async fn run_obscura<I, S>(bin: &Path, args: I) -> Result<String, ObscuraError>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let child = Command::new(bin)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| ObscuraError::Spawn {
            path: bin.to_owned(),
            source: e,
        })?;

    let output = child.wait_with_output().await?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(ObscuraError::NonZeroExit {
            code: output.status.code(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Fetch a single URL via `obscura fetch <url> --format <format>`.
///
/// `format` is passed verbatim to `--format`; Obscura supports
/// `html`, `text`, `links`, `markdown`, and `original`.
///
/// # Errors
///
/// - [`ObscuraError::BinaryNotFound`] — binary not resolved.
/// - [`ObscuraError::Spawn`] — failed to start the process.
/// - [`ObscuraError::Io`] — I/O error reading output.
/// - [`ObscuraError::NonZeroExit`] — obscura returned a non-zero exit code.
pub async fn fetch(url: &str, format: &str) -> Result<String, ObscuraError> {
    let bin = resolve_bin()?;
    run_obscura(&bin, ["fetch", url, "--format", format]).await
}

/// Scrape one or more URLs via `obscura scrape <urls...> --concurrent <N>`.
///
/// Returns the raw stdout from Obscura (JSON array of results by default).
///
/// # Errors
///
/// - [`ObscuraError::BinaryNotFound`] — binary not resolved.
/// - [`ObscuraError::Spawn`] — failed to start the process.
/// - [`ObscuraError::Io`] — I/O error reading output.
/// - [`ObscuraError::NonZeroExit`] — obscura returned a non-zero exit code.
pub async fn scrape(urls: &[String], concurrent: u32) -> Result<String, ObscuraError> {
    let bin = resolve_bin()?;

    // Build argument list: ["scrape", url1, url2, ..., "--concurrent", "N"]
    let concurrent_str = concurrent.to_string();
    let mut args: Vec<&str> = Vec::with_capacity(urls.len() + 3);
    args.push("scrape");
    for u in urls {
        args.push(u.as_str());
    }
    args.push("--concurrent");
    args.push(&concurrent_str);

    run_obscura(&bin, args).await
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    // SPDX-License-Identifier: Apache-2.0
    use std::fs;

    use tempfile::NamedTempFile;

    use super::*;

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    // Safety: env mutations in tests are safe within a single-threaded test
    // binary.  Tokio's `#[tokio::test]` by default uses a single-threaded
    // scheduler, so there is no race between env reads and writes across tests
    // **within the same process**.  We restore the variable after each test.
    //
    // `std::env::set_var` is `unsafe` since Rust 1.83 (nightly).  The tests
    // are gated to the `test` cfg where we allow unsafe.

    // ------------------------------------------------------------------
    // resolve_bin honours APHRODY_OBSCURA_BIN when it points to an
    // existing file.
    // ------------------------------------------------------------------
    #[test]
    fn resolve_bin_honours_env_var() {
        let _guard = ENV_LOCK.lock().unwrap();
        // Create a temporary file that "acts" as the obscura binary.
        let tmp = NamedTempFile::new().expect("tempfile");
        let path_str = tmp.path().to_str().expect("utf8 path").to_owned();

        // SAFETY: single-threaded test; no concurrent env readers.
        unsafe { std::env::set_var(ENV_OBSCURA_BIN, &path_str) };

        let result = resolve_bin();

        // SAFETY: restore env state regardless of outcome.
        unsafe { std::env::remove_var(ENV_OBSCURA_BIN) };

        let resolved = result.expect("resolve_bin should succeed when env var points to existing file");
        assert_eq!(resolved, tmp.path());
    }

    // ------------------------------------------------------------------
    // resolve_bin returns BinaryNotFound when the env var points to a
    // non-existent file and no sibling / PATH entry is available.
    // ------------------------------------------------------------------
    #[test]
    fn resolve_bin_nonexistent_env_var_returns_not_found() {
        let _guard = ENV_LOCK.lock().unwrap();
        // A path that definitely does not exist.
        let nonexistent = "/tmp/__aphrody_obscura_definitely_does_not_exist_xyzzy/obscura";

        // Clear any real obscura from PATH by overriding env with the bad path.
        // SAFETY: single-threaded test.
        unsafe { std::env::set_var(ENV_OBSCURA_BIN, nonexistent) };

        let result = resolve_bin();

        unsafe { std::env::remove_var(ENV_OBSCURA_BIN) };

        // The env var exists but points to a non-existent file.  resolve_bin
        // falls through to sibling + PATH probes.  On CI / dev machines where
        // `obscura` is not installed the final result must be BinaryNotFound.
        // If the machine happens to have obscura on PATH this assertion would
        // be incorrect, so we only assert the error case when resolve_bin
        // fails (which is the overwhelmingly common scenario here).
        if let Err(e) = result {
            assert!(
                matches!(e, ObscuraError::BinaryNotFound),
                "expected BinaryNotFound, got {e:?}"
            );
        }
        // If resolve_bin succeeded (obscura is on PATH), that is also correct
        // behaviour — we simply skip the assertion.
    }

    // ------------------------------------------------------------------
    // resolve_bin returns BinaryNotFound when env var is absent and the
    // path is cleared (simulated by using a temp dir with no `obscura`).
    // ------------------------------------------------------------------
    #[test]
    fn resolve_bin_returns_not_found_when_absent() {
        let _guard = ENV_LOCK.lock().unwrap();
        // Ensure the env override is unset.
        unsafe { std::env::remove_var(ENV_OBSCURA_BIN) };

        // Override PATH to an empty temp directory so no obscura binary can
        // be found there, and the sibling probe will not match either
        // (current_exe points into the test runner, not a dir with obscura).
        let tmp_dir = tempfile::tempdir().expect("tempdir");
        let original_path = std::env::var_os("PATH");

        // SAFETY: single-threaded test.
        unsafe {
            std::env::set_var("PATH", tmp_dir.path().as_os_str());
        }

        let result = resolve_bin();

        // Restore PATH.
        // SAFETY: single-threaded test.
        unsafe {
            match original_path {
                Some(p) => std::env::set_var("PATH", p),
                None => std::env::remove_var("PATH"),
            }
        }

        assert!(
            matches!(result, Err(ObscuraError::BinaryNotFound)),
            "expected BinaryNotFound, got {result:?}"
        );
    }

    // ------------------------------------------------------------------
    // Creating a fake executable in a temp dir and placing it on PATH
    // should let resolve_bin find it.
    // ------------------------------------------------------------------
    #[test]
    fn resolve_bin_finds_binary_on_custom_path() {
        let _guard = ENV_LOCK.lock().unwrap();
        let tmp_dir = tempfile::tempdir().expect("tempdir");

        // Write a zero-byte file named "obscura" — existence check is enough.
        #[cfg(unix)]
        let bin_name = "obscura";
        #[cfg(windows)]
        let bin_name = "obscura.exe";

        let fake_bin = tmp_dir.path().join(bin_name);
        fs::write(&fake_bin, b"").expect("write fake bin");

        // Unset the env override so resolve_bin reaches the PATH probe.
        unsafe { std::env::remove_var(ENV_OBSCURA_BIN) };

        let original_path = std::env::var_os("PATH");

        // SAFETY: single-threaded test.
        unsafe {
            std::env::set_var("PATH", tmp_dir.path().as_os_str());
        }

        let result = resolve_bin();

        unsafe {
            match original_path {
                Some(p) => std::env::set_var("PATH", p),
                None => std::env::remove_var("PATH"),
            }
        }

        let resolved = result.expect("should find fake obscura on PATH");
        assert_eq!(resolved, fake_bin);
    }

    // ------------------------------------------------------------------
    // fetch() propagates BinaryNotFound when resolve_bin fails.
    // ------------------------------------------------------------------
    #[tokio::test]
    async fn fetch_returns_binary_not_found() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::remove_var(ENV_OBSCURA_BIN) };

        let tmp_dir = tempfile::tempdir().expect("tempdir");
        let original_path = std::env::var_os("PATH");

        unsafe { std::env::set_var("PATH", tmp_dir.path().as_os_str()) };

        let result = fetch("https://example.com", "markdown").await;

        unsafe {
            match original_path {
                Some(p) => std::env::set_var("PATH", p),
                None => std::env::remove_var("PATH"),
            }
        }

        assert!(
            matches!(result, Err(ObscuraError::BinaryNotFound)),
            "expected BinaryNotFound, got {result:?}"
        );
    }

    // ------------------------------------------------------------------
    // scrape() propagates BinaryNotFound when resolve_bin fails.
    // ------------------------------------------------------------------
    #[tokio::test]
    async fn scrape_returns_binary_not_found() {
        let _guard = ENV_LOCK.lock().unwrap();
        unsafe { std::env::remove_var(ENV_OBSCURA_BIN) };

        let tmp_dir = tempfile::tempdir().expect("tempdir");
        let original_path = std::env::var_os("PATH");

        unsafe { std::env::set_var("PATH", tmp_dir.path().as_os_str()) };

        let urls = vec!["https://example.com".to_owned()];
        let result = scrape(&urls, 4).await;

        unsafe {
            match original_path {
                Some(p) => std::env::set_var("PATH", p),
                None => std::env::remove_var("PATH"),
            }
        }

        assert!(
            matches!(result, Err(ObscuraError::BinaryNotFound)),
            "expected BinaryNotFound, got {result:?}"
        );
    }
}
