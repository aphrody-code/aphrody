// SPDX-License-Identifier: Apache-2.0
//! Pre-`main` process hardening.
//!
//! Call [`pre_main_hardening`] as the very first statement in `main`, before any
//! credential is read or any network client is built. On Unix it:
//!
//! * marks the process non-dumpable (`prctl(PR_SET_DUMPABLE, 0)` on Linux,
//!   `ptrace(PT_DENY_ATTACH)` on macOS) so a same-user process cannot attach a
//!   debugger and read decrypted secrets out of our address space;
//! * sets `RLIMIT_CORE` to zero so a crash never writes a core dump containing
//!   those secrets to disk;
//! * removes every `LD_*` (Linux/BSD) and `DYLD_*` (macOS) environment variable
//!   so a poisoned loader cannot inject code into us or our children.
//!
//! On Windows and wasm this is currently a no-op (documented below) so the call
//! site stays platform-agnostic.
//!
//! Unlike the upstream Codex implementation, failures here are **best-effort and
//! non-fatal**: aphrody must keep running headless even on a kernel that rejects
//! a given `prctl`. Each failure returns an [`HardeningError`] entry instead of
//! aborting the process, and [`pre_main_hardening`] swallows them (logging to
//! stderr) so it is safe to call unconditionally.

/// A single hardening step that did not apply, with the OS error behind it.
#[derive(Debug)]
#[non_exhaustive]
pub struct HardeningError {
    /// Human-readable name of the step that failed (e.g. `"PR_SET_DUMPABLE"`).
    pub step: &'static str,
    /// The underlying OS error.
    pub source: std::io::Error,
}

impl std::fmt::Display for HardeningError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "process hardening step `{}` failed: {}", self.step, self.source)
    }
}

impl std::error::Error for HardeningError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// Run [`pre_main_hardening`] only when guardrails are opted in via
/// [`crate::GUARD_ENV`]. This is the variant the binary's `main` calls: by
/// default (env unset) it is a complete no-op, preserving aphrody's
/// fully-autonomous, guardrail-off posture (cf. `CLAUDE.md` §0.1).
pub fn pre_main_hardening_if_enabled() {
    if crate::guardrails_enabled() {
        pre_main_hardening();
    }
}

/// Apply every platform-appropriate hardening step, best-effort.
///
/// Any step that fails is logged to stderr and otherwise ignored, so this is
/// safe to call as the first line of `main` on any target. Note this applies
/// hardening **unconditionally** — most callers want
/// [`pre_main_hardening_if_enabled`] instead.
pub fn pre_main_hardening() {
    for err in try_pre_main_hardening() {
        eprintln!("aphrody-guard: {err}");
    }
}

/// Like [`pre_main_hardening`] but returns the list of steps that failed instead
/// of logging them, so a caller can decide how loud to be.
#[must_use]
pub fn try_pre_main_hardening() -> Vec<HardeningError> {
    #[cfg(any(target_os = "linux", target_os = "android"))]
    {
        harden_linux()
    }

    #[cfg(target_os = "macos")]
    {
        harden_macos()
    }

    #[cfg(any(target_os = "freebsd", target_os = "openbsd", target_os = "netbsd"))]
    {
        harden_bsd()
    }

    #[cfg(not(unix))]
    {
        // Windows: a full restricted-token / job-object sandbox is tracked
        // separately; nothing portable to do pre-main yet. wasm: no process to
        // harden. Either way, no failures to report.
        Vec::new()
    }
}

#[cfg(any(target_os = "linux", target_os = "android"))]
fn harden_linux() -> Vec<HardeningError> {
    let mut errors = Vec::new();

    // SAFETY: prctl with PR_SET_DUMPABLE takes scalar args and touches no memory.
    let ret = unsafe { libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) };
    if ret != 0 {
        errors.push(HardeningError {
            step: "PR_SET_DUMPABLE",
            source: std::io::Error::last_os_error(),
        });
    }

    if let Err(source) = set_core_file_size_limit_to_zero() {
        errors.push(HardeningError { step: "RLIMIT_CORE", source });
    }

    remove_env_vars_with_prefix(b"LD_");
    errors
}

#[cfg(target_os = "macos")]
fn harden_macos() -> Vec<HardeningError> {
    let mut errors = Vec::new();

    // SAFETY: ptrace(PT_DENY_ATTACH) takes scalar args and touches no memory.
    let ret = unsafe { libc::ptrace(libc::PT_DENY_ATTACH, 0, std::ptr::null_mut(), 0) };
    if ret == -1 {
        errors.push(HardeningError {
            step: "PT_DENY_ATTACH",
            source: std::io::Error::last_os_error(),
        });
    }

    if let Err(source) = set_core_file_size_limit_to_zero() {
        errors.push(HardeningError { step: "RLIMIT_CORE", source });
    }

    remove_env_vars_with_prefix(b"DYLD_");
    errors
}

#[cfg(any(target_os = "freebsd", target_os = "openbsd", target_os = "netbsd"))]
fn harden_bsd() -> Vec<HardeningError> {
    let mut errors = Vec::new();
    if let Err(source) = set_core_file_size_limit_to_zero() {
        errors.push(HardeningError { step: "RLIMIT_CORE", source });
    }
    remove_env_vars_with_prefix(b"LD_");
    errors
}

/// Mark the current Linux process non-dumpable on demand (e.g. from a worker
/// thread spawned after `main`). Returns the OS error on failure.
#[cfg(any(target_os = "linux", target_os = "android"))]
pub fn disable_process_dumping() -> std::io::Result<()> {
    // SAFETY: see `harden_linux`.
    let ret = unsafe { libc::prctl(libc::PR_SET_DUMPABLE, 0, 0, 0, 0) };
    if ret == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

#[cfg(unix)]
fn set_core_file_size_limit_to_zero() -> std::io::Result<()> {
    let rlim = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
    // SAFETY: setrlimit reads exactly one `rlimit` we own; the pointer is valid.
    let ret = unsafe { libc::setrlimit(libc::RLIMIT_CORE, &rlim) };
    if ret == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
    }
}

/// Remove every environment variable whose key starts with `prefix`.
///
/// Operates on raw bytes so non-UTF-8 keys (which `std::env::var` would hide)
/// are still matched and removed.
#[cfg(unix)]
fn remove_env_vars_with_prefix(prefix: &[u8]) {
    for key in env_keys_with_prefix(std::env::vars_os(), prefix) {
        // SAFETY: aphrody hardens before spawning threads, so there is no
        // concurrent getenv/setenv racing this removal.
        unsafe {
            std::env::remove_var(key);
        }
    }
}

#[cfg(unix)]
fn env_keys_with_prefix<I>(vars: I, prefix: &[u8]) -> Vec<std::ffi::OsString>
where
    I: IntoIterator<Item = (std::ffi::OsString, std::ffi::OsString)>,
{
    use std::os::unix::ffi::OsStrExt;
    vars.into_iter()
        .filter_map(|(key, _)| key.as_os_str().as_bytes().starts_with(prefix).then_some(key))
        .collect()
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use std::ffi::{OsStr, OsString};
    use std::os::unix::ffi::{OsStrExt, OsStringExt};

    #[test]
    fn env_keys_with_prefix_handles_non_utf8_entries() {
        let non_utf8_key = OsString::from_vec(vec![b'L', b'D', b'_', 0xF0]);
        assert!(non_utf8_key.clone().into_string().is_err());
        let value = OsString::from_vec(vec![0xF0, 0x9F, 0x92, 0xA9]);

        let keys = env_keys_with_prefix(
            vec![
                (OsStr::from_bytes(b"R\xD6DBURK").to_os_string(), value.clone()),
                (non_utf8_key.clone(), value),
            ],
            b"LD_",
        );
        assert_eq!(keys, vec![non_utf8_key]);
    }

    #[test]
    fn env_keys_with_prefix_filters_only_matching_keys() {
        let ld = OsStr::from_bytes(b"LD_PRELOAD");
        let vars = vec![
            (OsString::from("PATH"), OsString::from("/usr/bin")),
            (ld.to_os_string(), OsString::from("/evil.so")),
            (OsString::from("DYLD_FOO"), OsString::from("bar")),
        ];
        let keys = env_keys_with_prefix(vars, b"LD_");
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].as_os_str(), ld);
    }

    #[test]
    fn try_hardening_is_callable_and_non_fatal() {
        // Must not panic or abort the test process; failures (if any) are data.
        let _errors = try_pre_main_hardening();
    }
}
