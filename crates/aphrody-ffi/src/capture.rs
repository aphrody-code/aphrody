// SPDX-License-Identifier: Apache-2.0
//! Cross-platform stdout/stderr capture for the `*_captured` FFI entry points.
//!
//! Redirects the process stdout AND stderr to temporary files for the duration
//! of a closure, then reads them back. The redirect targets the exact mechanism
//! Rust's `std` writes through on each platform:
//!
//! * **Unix** — `dup2` over file descriptors 1 and 2 (Rust writes to fd 1/2).
//! * **Windows** — `SetStdHandle(STD_OUTPUT_HANDLE / STD_ERROR_HANDLE, ...)`.
//!   Rust's std re-fetches the standard handle per write via `GetStdHandle`, so
//!   replacing it redirects `println!`; a CRT `_dup2` would NOT (Rust does not
//!   write through the CRT fd on Windows).
//!
//! Output is buffered to a real temp file (never a pipe), so arbitrarily large
//! output cannot deadlock. Hosts (e.g. Bun) call the FFI symbol synchronously,
//! so no foreign writes race the redirect window.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};

#[cfg(unix)]
use unix::Redirect;
#[cfg(windows)]
use windows::Redirect;

/// Run `f` with stdout + stderr redirected to temp files; return
/// `(exit_code, captured_stdout, captured_stderr)`.
///
/// Capture is best-effort: if a redirect cannot be installed the closure still
/// runs against the normal streams and the matching captured string is empty.
/// The closure's return value (the exit code) is always propagated unchanged.
pub(crate) fn with_captured_stdio<F: FnOnce() -> i32>(f: F) -> (i32, String, String) {
    // Flush buffered output before redirecting so earlier output does not leak
    // into the captured files.
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();

    let out_file = tempfile::tempfile().ok();
    let err_file = tempfile::tempfile().ok();

    let out_redirect = out_file
        .as_ref()
        .and_then(|file| Redirect::stdout(file).ok());
    let err_redirect = err_file
        .as_ref()
        .and_then(|file| Redirect::stderr(file).ok());

    let code = f();

    // Flush the command's final output to the temp files before restoring.
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();

    // Restore the original handles (via Drop) before reading back.
    drop(out_redirect);
    drop(err_redirect);

    let stdout = out_file.map(read_to_string).unwrap_or_default();
    let stderr = err_file.map(read_to_string).unwrap_or_default();
    (code, stdout, stderr)
}

/// Rewind a temp file and read it as a lossy-UTF-8 string.
fn read_to_string(mut file: File) -> String {
    let mut bytes = Vec::new();
    if file.seek(SeekFrom::Start(0)).is_ok() && file.read_to_end(&mut bytes).is_ok() {
        String::from_utf8_lossy(&bytes).into_owned()
    } else {
        String::new()
    }
}

#[cfg(unix)]
mod unix {
    use std::fs::File;
    use std::io;
    use std::os::unix::io::AsRawFd;

    /// RAII redirect of a standard file descriptor to another file; restores the
    /// original descriptor on drop.
    pub(super) struct Redirect {
        saved: libc::c_int,
        target: libc::c_int,
    }

    impl Redirect {
        pub(super) fn stdout(file: &File) -> io::Result<Self> {
            Self::install(file, libc::STDOUT_FILENO)
        }

        pub(super) fn stderr(file: &File) -> io::Result<Self> {
            Self::install(file, libc::STDERR_FILENO)
        }

        fn install(file: &File, target: libc::c_int) -> io::Result<Self> {
            // SAFETY: `dup` duplicates a valid descriptor; it returns -1 on
            // error, which we surface immediately.
            let saved = unsafe { libc::dup(target) };
            if saved < 0 {
                return Err(io::Error::last_os_error());
            }
            // SAFETY: `dup2` points `target` at the temp file's descriptor.
            if unsafe { libc::dup2(file.as_raw_fd(), target) } < 0 {
                let err = io::Error::last_os_error();
                // SAFETY: close the dup we just made on the error path.
                unsafe { libc::close(saved) };
                return Err(err);
            }
            Ok(Self { saved, target })
        }
    }

    impl Drop for Redirect {
        fn drop(&mut self) {
            // SAFETY: restore the saved descriptor over `target`, then close the
            // dup. Errors on teardown are not actionable and are ignored.
            unsafe {
                libc::dup2(self.saved, self.target);
                libc::close(self.saved);
            }
        }
    }
}

#[cfg(windows)]
mod windows {
    use std::fs::File;
    use std::io;
    use std::os::windows::io::AsRawHandle;

    use windows_sys::Win32::Foundation::HANDLE;
    use windows_sys::Win32::System::Console::{
        GetStdHandle, SetStdHandle, STD_ERROR_HANDLE, STD_HANDLE, STD_OUTPUT_HANDLE,
    };

    /// RAII redirect of a standard Win32 handle to another file; restores the
    /// original handle on drop.
    pub(super) struct Redirect {
        saved: HANDLE,
        which: STD_HANDLE,
    }

    impl Redirect {
        pub(super) fn stdout(file: &File) -> io::Result<Self> {
            Self::install(file, STD_OUTPUT_HANDLE)
        }

        pub(super) fn stderr(file: &File) -> io::Result<Self> {
            Self::install(file, STD_ERROR_HANDLE)
        }

        fn install(file: &File, which: STD_HANDLE) -> io::Result<Self> {
            // SAFETY: `GetStdHandle` returns the current standard handle (or
            // INVALID_HANDLE_VALUE, which round-trips harmlessly on restore).
            let saved = unsafe { GetStdHandle(which) };
            // std `RawHandle` and windows-sys `HANDLE` are both `*mut c_void`.
            let target: HANDLE = file.as_raw_handle();
            // SAFETY: redirect the standard handle to the temp file.
            if unsafe { SetStdHandle(which, target) } == 0 {
                return Err(io::Error::last_os_error());
            }
            Ok(Self { saved, which })
        }
    }

    impl Drop for Redirect {
        fn drop(&mut self) {
            // SAFETY: restore the previously-saved standard handle.
            unsafe {
                SetStdHandle(self.which, self.saved);
            }
        }
    }
}
