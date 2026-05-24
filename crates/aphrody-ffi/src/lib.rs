// SPDX-License-Identifier: Apache-2.0
//! # aphrody-ffi — aphrody as a native library.
//!
//! A thin, stable **C ABI** over the `aphrody` library crate so the *entire*
//! command surface is reachable in-process from Bun (`bun:ffi`) and any other
//! C-ABI host (C / C++ / Zig / Python `ctypes` / .NET P/Invoke), not only by
//! spawning the `aphrody` binary.
//!
//! ## Exported symbols
//!
//! | Symbol | Purpose |
//! |---|---|
//! | `aphrody_abi_version() -> u32` | ABI version for the host to check. |
//! | `aphrody_version() -> *const c_char` | aphrody version (static, no free). |
//! | `aphrody_run(argc, argv) -> c_int` | run any command, inherited stdio. |
//! | `aphrody_run_json(args_json) -> c_int` | same, args as a JSON array. |
//! | `aphrody_run_captured(args_json) -> *mut c_char` | run + capture; returns `{"code","stdout","stderr"}` JSON (free with `aphrody_string_free`). |
//! | `aphrody_string_free(*mut c_char)` | free a string this library returned. |
//! | `aphrody_last_error() -> *const c_char` | last error on this thread (static, no free). |
//!
//! Arguments are the command arguments WITHOUT the program name — a synthetic
//! `argv[0]` is prepended internally — e.g. `["doctor", "--json"]`.
//!
//! The hand-written C header lives in `include/aphrody.h` and is kept in
//! lock-step with this module: a unit test parses `APHRODY_ABI_VERSION` from it
//! and asserts it equals the Rust `ABI_VERSION`, and the FFI-injected status
//! codes are mirrored by the `AphrodyStatus` enum (sysexits-compatible). The
//! run functions still return a plain `c_int` because a wrapped command may
//! exit with any process code, not only the closed set in `AphrodyStatus`.
//!
//! ## Robustness
//!
//! * Every entry point catches Rust panics (`std::panic::catch_unwind`) and
//!   maps them to an error code, so a panic never unwinds across the C boundary
//!   and tears the host process down.
//! * The wrapped command surface is exit-free (commands return an error rather
//!   than calling `process::exit`), so a command never kills the host.
//! * `mimalloc` is the global allocator (the `aphrody` library is
//!   allocator-agnostic, so the cdylib matches the binary's choice here).
//!
//! On wasm32 this crate compiles to an empty module — the native command
//! surface (tokio + reqwest + rustls) does not link on wasm.

#[cfg(not(target_arch = "wasm32"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(not(target_arch = "wasm32"))]
pub use native::*;

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use std::cell::RefCell;
    use std::ffi::{c_char, c_int, CStr, CString, OsString};
    use std::panic::catch_unwind;
    use std::ptr;
    use std::sync::{Mutex, OnceLock};

    use aphrody_stdio_capture::with_captured_stdio;

    /// ABI version of the exported symbol set. Bump on any incompatible change.
    ///
    /// Kept in lock-step with `APHRODY_ABI_VERSION` in `include/aphrody.h`; the
    /// `header_abi_version_matches` test parses the header and asserts they
    /// agree, so the two cannot silently drift.
    pub(crate) const ABI_VERSION: u32 = 1;

    /// FFI-layer status codes, mirroring `enum AphrodyStatus` in
    /// `include/aphrody.h`. `#[repr(i32)]` makes the enum layout-compatible
    /// with the `c_int` the run functions return, so a value can be produced as
    /// `AphrodyStatus::Usage as c_int`.
    ///
    /// These are only the codes the FFI boundary itself injects; a wrapped
    /// command may still return any other process exit code unchanged.
    #[repr(i32)]
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(crate) enum AphrodyStatus {
        /// Malformed request: invalid argv, JSON, or UTF-8 (sysexits EX_USAGE).
        Usage = 64,
        /// Internal error: a caught Rust panic (sysexits EX_SOFTWARE).
        Software = 70,
    }

    thread_local! {
        /// Last error recorded on this thread (the C ABI has no Result channel).
        static LAST_ERROR: RefCell<Option<CString>> = const { RefCell::new(None) };
    }

    fn set_last_error<M: Into<Vec<u8>>>(msg: M) {
        let value = CString::new(msg).unwrap_or_else(|_| {
            CString::new("error message contained an interior NUL byte")
                .expect("static message has no NUL")
        });
        LAST_ERROR.with(|cell| *cell.borrow_mut() = Some(value));
    }

    /// Returns the runtime ABI version of this library (see `APHRODY_NODISCARD`
    /// / `[[nodiscard]]` on the C declaration).
    #[unsafe(no_mangle)]
    #[must_use]
    pub extern "C" fn aphrody_abi_version() -> u32 {
        ABI_VERSION
    }

    /// Returns the aphrody version as a static, NUL-terminated UTF-8 string.
    ///
    /// The pointer is owned by the library and valid for the entire program
    /// lifetime — do NOT free it. Never NULL (`APHRODY_RETURNS_NONNULL`).
    #[unsafe(no_mangle)]
    #[must_use]
    pub extern "C" fn aphrody_version() -> *const c_char {
        static VERSION: OnceLock<CString> = OnceLock::new();
        VERSION
            .get_or_init(|| {
                CString::new(env!("CARGO_PKG_VERSION")).expect("crate version has no NUL")
            })
            .as_ptr()
    }

    /// Run an aphrody command with inherited stdout/stderr, returning the exit
    /// code.
    ///
    /// `argv` holds the command arguments WITHOUT the program name (a synthetic
    /// `argv[0]` is prepended internally), e.g. `["doctor", "--json"]`.
    ///
    /// # Safety
    /// `argv` must point to `argc` valid, NUL-terminated C strings, or be NULL
    /// iff `argc == 0`. The pointers are only read for the duration of the call.
    #[unsafe(no_mangle)]
    #[must_use]
    pub unsafe extern "C" fn aphrody_run(argc: c_int, argv: *const *const c_char) -> c_int {
        let outcome = catch_unwind(|| {
            // SAFETY: forwarded contract from this function's `# Safety` clause.
            match unsafe { collect_args(argc, argv) } {
                Some(args) => run_argv(args),
                None => {
                    set_last_error("invalid argv: null pointer or negative argc");
                    AphrodyStatus::Usage as c_int
                }
            }
        });
        outcome.unwrap_or_else(|payload| {
            set_last_error(format!("aphrody panicked: {}", panic_message(&*payload)));
            AphrodyStatus::Software as c_int
        })
    }

    /// Run an aphrody command from a JSON array of argument strings with
    /// inherited stdout/stderr, returning the exit code. Ergonomic alternative
    /// to `aphrody_run` for hosts that pass a JSON string more easily than a
    /// `char**` (e.g. Bun).
    ///
    /// # Safety
    /// `args_json` must be a valid NUL-terminated UTF-8 C string, or NULL
    /// (treated as `[]`).
    #[unsafe(no_mangle)]
    #[must_use]
    pub unsafe extern "C" fn aphrody_run_json(args_json: *const c_char) -> c_int {
        let outcome = catch_unwind(|| {
            // SAFETY: forwarded contract.
            match unsafe { parse_json_args(args_json) } {
                Ok(args) => run_argv(args),
                Err(message) => {
                    set_last_error(message);
                    AphrodyStatus::Usage as c_int
                }
            }
        });
        outcome.unwrap_or_else(|payload| {
            set_last_error(format!("aphrody panicked: {}", panic_message(&*payload)));
            AphrodyStatus::Software as c_int
        })
    }

    /// Run an aphrody command from a JSON array of arguments with stdout AND
    /// stderr captured, returning a newly-allocated NUL-terminated JSON
    /// document: `{"code":<int>,"stdout":"<text>","stderr":"<text>"}`.
    ///
    /// Returns NULL only on allocation failure (the cause is then retrievable
    /// via `aphrody_last_error`). The caller MUST free a non-NULL result with
    /// `aphrody_string_free`.
    ///
    /// # Safety
    /// `args_json` must be a valid NUL-terminated UTF-8 C string, or NULL
    /// (treated as `[]`).
    #[unsafe(no_mangle)]
    #[must_use]
    pub unsafe extern "C" fn aphrody_run_captured(args_json: *const c_char) -> *mut c_char {
        let outcome = catch_unwind(|| {
            // SAFETY: forwarded contract.
            match unsafe { parse_json_args(args_json) } {
                Ok(args) => {
                    // Serialise the process-global stdio redirect across threads.
                    let _guard = capture_lock();
                    let (code, out, err) = with_captured_stdio(|| run_argv(args));
                    json_result(code, &out, &err)
                }
                Err(message) => json_result(AphrodyStatus::Usage as i32, "", &message),
            }
        });
        let json = outcome.unwrap_or_else(|payload| {
            let message = format!("aphrody panicked: {}", panic_message(&*payload));
            set_last_error(message.clone());
            json_result(AphrodyStatus::Software as i32, "", &message)
        });
        string_to_c(json)
    }

    /// Free a string previously returned by this library (e.g.
    /// `aphrody_run_captured`). Passing NULL is a no-op.
    ///
    /// # Safety
    /// `ptr` must be a pointer this library returned and that has not already
    /// been freed, or NULL.
    #[unsafe(no_mangle)]
    pub unsafe extern "C" fn aphrody_string_free(ptr: *mut c_char) {
        if ptr.is_null() {
            return;
        }
        // SAFETY: the caller guarantees `ptr` came from `CString::into_raw` in
        // this library and is freed exactly once.
        drop(unsafe { CString::from_raw(ptr) });
    }

    /// Returns the last error recorded on THIS thread as a static
    /// NUL-terminated string, or NULL if there is none.
    ///
    /// The pointer is valid until the next library call on this thread — copy it
    /// if you need to keep it. Do NOT free it.
    #[unsafe(no_mangle)]
    #[must_use]
    pub extern "C" fn aphrody_last_error() -> *const c_char {
        LAST_ERROR.with(|cell| match cell.borrow().as_ref() {
            Some(value) => value.as_ptr(),
            None => ptr::null(),
        })
    }

    // ---- internal helpers -------------------------------------------------

    /// Process-wide tokio runtime, created lazily on first use, so repeated FFI
    /// calls do not pay the worker-thread spawn cost on every invocation.
    fn runtime() -> &'static tokio::runtime::Runtime {
        static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();
        RUNTIME.get_or_init(|| {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("aphrody-ffi: failed to build the tokio runtime")
        })
    }

    /// Serialise stdout/stderr capture: the redirect swaps process-global file
    /// descriptors, so two concurrent captures (e.g. Bun Workers) would corrupt
    /// each other's save/restore. Lock poison is recovered (the `()` payload is
    /// meaningless).
    fn capture_lock() -> std::sync::MutexGuard<'static, ()> {
        static CAPTURE_LOCK: Mutex<()> = Mutex::new(());
        CAPTURE_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Best-effort message extracted from a `catch_unwind` panic payload.
    fn panic_message(payload: &(dyn std::any::Any + Send)) -> &str {
        payload
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("unknown panic")
    }

    /// Build the argument vector (synthetic program name + caller args) and
    /// dispatch on the persistent runtime.
    fn run_argv(args: Vec<String>) -> i32 {
        let mut argv: Vec<OsString> = Vec::with_capacity(args.len() + 1);
        argv.push(OsString::from("aphrody"));
        argv.extend(args.into_iter().map(OsString::from));
        runtime().block_on(aphrody::run_async(argv))
    }

    /// Collect `argc` C strings into owned `String`s (lossy UTF-8).
    ///
    /// # Safety
    /// `argv` must point to `argc` valid NUL-terminated strings (or be NULL iff
    /// `argc == 0`).
    unsafe fn collect_args(argc: c_int, argv: *const *const c_char) -> Option<Vec<String>> {
        let argc = usize::try_from(argc).ok()?;
        if argc == 0 {
            return Some(Vec::new());
        }
        if argv.is_null() {
            return None;
        }
        let mut out = Vec::with_capacity(argc);
        for i in 0..argc {
            // SAFETY: caller guarantees `argv` has `argc` valid entries.
            let entry = unsafe { *argv.add(i) };
            if entry.is_null() {
                return None;
            }
            // SAFETY: caller guarantees each entry is a NUL-terminated string.
            let text = unsafe { CStr::from_ptr(entry) };
            out.push(text.to_string_lossy().into_owned());
        }
        Some(out)
    }

    /// Parse a JSON array of strings from a C string.
    ///
    /// # Safety
    /// `ptr` must be a valid NUL-terminated string, or NULL (treated as `[]`).
    unsafe fn parse_json_args(ptr: *const c_char) -> Result<Vec<String>, String> {
        if ptr.is_null() {
            return Ok(Vec::new());
        }
        // SAFETY: caller guarantees a NUL-terminated string.
        let raw = unsafe { CStr::from_ptr(ptr) };
        let text = raw
            .to_str()
            .map_err(|_| "args_json is not valid UTF-8".to_string())?;
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Ok(Vec::new());
        }
        serde_json::from_str::<Vec<String>>(trimmed)
            .map_err(|err| format!("args_json must be a JSON array of strings: {err}"))
    }

    /// Render the captured run result as a compact JSON document.
    fn json_result(code: i32, stdout: &str, stderr: &str) -> String {
        serde_json::json!({
            "code": code,
            "stdout": stdout,
            "stderr": stderr,
        })
        .to_string()
    }

    /// Move a `String` into a freshly-allocated C string (caller frees via
    /// `aphrody_string_free`); NULL + recorded error on interior NUL.
    fn string_to_c(text: String) -> *mut c_char {
        match CString::new(text) {
            Ok(value) => value.into_raw(),
            Err(_) => {
                set_last_error("result contained an interior NUL byte");
                ptr::null_mut()
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn abi_version_is_one() {
            assert_eq!(aphrody_abi_version(), 1);
            assert_eq!(aphrody_abi_version(), ABI_VERSION);
        }

        /// The hand-written header must declare the same ABI revision the Rust
        /// symbol reports, so the two never drift. Parses `APHRODY_ABI_VERSION`
        /// out of `include/aphrody.h` (relative to `CARGO_MANIFEST_DIR`).
        #[test]
        fn header_abi_version_matches() {
            let header = std::fs::read_to_string(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/include/aphrody.h"
            ))
            .expect("aphrody.h is readable");

            let line = header
                .lines()
                .find(|l| l.trim_start().starts_with("#define APHRODY_ABI_VERSION"))
                .expect("header defines APHRODY_ABI_VERSION");

            // `#define APHRODY_ABI_VERSION 1u` -> strip the `u`/`U` suffix.
            let value: u32 = line
                .rsplit_once("APHRODY_ABI_VERSION")
                .map(|(_, rest)| rest.trim())
                .and_then(|tok| tok.trim_end_matches(['u', 'U', 'l', 'L']).parse().ok())
                .expect("APHRODY_ABI_VERSION is a parseable integer literal");

            assert_eq!(value, ABI_VERSION, "header and Rust ABI versions disagree");
        }

        /// The typed status codes must keep the documented sysexits values; the
        /// `#[repr(i32)]` lets us round-trip them through `c_int`.
        #[test]
        fn status_codes_match_documented_values() {
            assert_eq!(AphrodyStatus::Usage as c_int, 64);
            assert_eq!(AphrodyStatus::Software as c_int, 70);
        }

        /// A negative `argc` is a usage error, never a panic, and records a
        /// diagnostic on this thread.
        #[test]
        fn run_rejects_negative_argc() {
            // SAFETY: NULL argv with a negative argc is an explicitly handled,
            // rejected input — no pointer is dereferenced.
            let code = unsafe { aphrody_run(-1, ptr::null()) };
            assert_eq!(code, AphrodyStatus::Usage as c_int);
            assert!(!aphrody_last_error().is_null());
        }

        /// `aphrody_run_json` with malformed JSON returns the usage code rather
        /// than panicking, and string_free tolerates the resulting NULL paths.
        #[test]
        fn run_json_rejects_malformed_json() {
            let json = CString::new("not json").unwrap();
            // SAFETY: valid NUL-terminated string; bad *content* is handled.
            let code = unsafe { aphrody_run_json(json.as_ptr()) };
            assert_eq!(code, AphrodyStatus::Usage as c_int);
        }

        #[test]
        fn string_free_tolerates_null() {
            // SAFETY: NULL is an explicitly documented no-op for string_free.
            unsafe { aphrody_string_free(ptr::null_mut()) };
        }

        #[test]
        fn version_is_non_empty_and_stable() {
            let ptr = aphrody_version();
            assert!(!ptr.is_null());
            // SAFETY: aphrody_version returns a valid static C string.
            let text = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
            assert!(!text.is_empty());
            // Same static across calls.
            assert_eq!(aphrody_version(), ptr);
        }

        #[test]
        fn parse_json_args_round_trips() {
            let json = CString::new(r#"["doctor","--json"]"#).unwrap();
            // SAFETY: valid NUL-terminated string.
            let args = unsafe { parse_json_args(json.as_ptr()) }.unwrap();
            assert_eq!(args, vec!["doctor".to_string(), "--json".to_string()]);
        }

        #[test]
        fn parse_json_args_null_is_empty() {
            // SAFETY: NULL is an explicitly handled input.
            let args = unsafe { parse_json_args(ptr::null()) }.unwrap();
            assert!(args.is_empty());
        }

        #[test]
        fn parse_json_args_rejects_non_array() {
            let json = CString::new(r#"{"not":"an array"}"#).unwrap();
            // SAFETY: valid NUL-terminated string.
            let result = unsafe { parse_json_args(json.as_ptr()) };
            assert!(result.is_err());
        }

        #[test]
        fn collect_args_builds_vector() {
            let first = CString::new("scan").unwrap();
            let second = CString::new("tree").unwrap();
            let argv = [first.as_ptr(), second.as_ptr()];
            // SAFETY: argv has 2 valid entries.
            let args = unsafe { collect_args(2, argv.as_ptr()) }.unwrap();
            assert_eq!(args, vec!["scan".to_string(), "tree".to_string()]);
        }

        #[test]
        fn collect_args_zero_is_empty() {
            // SAFETY: argc 0 with NULL argv is explicitly allowed.
            let args = unsafe { collect_args(0, ptr::null()) }.unwrap();
            assert!(args.is_empty());
        }

        #[test]
        fn json_result_has_expected_shape() {
            let doc = json_result(3, "hello", "oops");
            let value: serde_json::Value = serde_json::from_str(&doc).unwrap();
            assert_eq!(value["code"], 3);
            assert_eq!(value["stdout"], "hello");
            assert_eq!(value["stderr"], "oops");
        }

        #[test]
        fn last_error_round_trips() {
            set_last_error("boom");
            let ptr = aphrody_last_error();
            assert!(!ptr.is_null());
            // SAFETY: non-null static-lifetime string set on this thread.
            let text = unsafe { CStr::from_ptr(ptr) }.to_str().unwrap();
            assert_eq!(text, "boom");
        }
    }
}
