// SPDX-License-Identifier: Apache-2.0
//! aphrody desktop shell (Tauri v2).
//!
//! Path (a): every command runs the aphrody CLI IN-PROCESS via
//! [`aphrody::run_captured`] (Rust -> Rust, no subprocess, no FFI hop). The
//! captured stdout/stderr + exit code are returned straight to the M3 webview,
//! so a GUI action is the exact same code path the terminal would run.

use std::sync::Mutex;

use tauri::Manager;

/// [`aphrody::run_captured`] redirects the PROCESS-GLOBAL stdout/stderr handles
/// for the duration of a run (see the `aphrody-stdio-capture` crate). Tauri can
/// dispatch commands concurrently, so every capture is serialised behind this
/// lock; two simultaneous redirects would interleave each other's output.
static EXEC_LOCK: Mutex<()> = Mutex::new(());

/// Result of an in-process command run, shaped for the webview. Owned by the
/// shell (rather than re-exporting [`aphrody::CapturedRun`]) so the IPC surface
/// stays stable even if the CLI type evolves.
#[derive(serde::Serialize)]
struct ExecResult {
    /// Process exit code (0 on success).
    code: i32,
    /// Captured standard output (lossy UTF-8).
    stdout: String,
    /// Captured standard error (lossy UTF-8).
    stderr: String,
}

impl From<aphrody::CapturedRun> for ExecResult {
    fn from(run: aphrody::CapturedRun) -> Self {
        Self { code: run.code, stdout: run.stdout, stderr: run.stderr }
    }
}

/// Run an aphrody command in-process and return its captured output.
///
/// `args` is the argument vector WITHOUT the program name (e.g.
/// `["re", "triage", "/path"]`); the conventional `argv[0]` is prepended here.
/// The blocking capture runs on Tauri's blocking pool so the UI thread never
/// freezes, and is serialised by [`EXEC_LOCK`].
#[tauri::command]
async fn aphrody_exec(args: Vec<String>) -> Result<ExecResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = EXEC_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut argv = Vec::with_capacity(args.len() + 1);
        argv.push(String::from("aphrody"));
        argv.extend(args);
        ExecResult::from(aphrody::run_captured(argv))
    })
    .await
    .map_err(|err| format!("aphrody_exec failed to join the worker thread: {err}"))
}

/// Static shell + host metadata for the UI header. The CLI's own version comes
/// from `aphrody_exec(["version", "--json"])`; this is the cheap, synchronous
/// info the header needs at first paint.
#[derive(serde::Serialize)]
struct Meta {
    /// The desktop shell's own version (this crate).
    app_version: &'static str,
    /// `std::env::consts::OS` (e.g. "windows", "linux", "macos").
    target_os: &'static str,
    /// `std::env::consts::ARCH` (e.g. "x86_64", "aarch64").
    target_arch: &'static str,
    /// `std::env::consts::FAMILY` (e.g. "windows", "unix").
    family: &'static str,
}

#[tauri::command]
fn aphrody_meta() -> Meta {
    Meta {
        app_version: env!("CARGO_PKG_VERSION"),
        target_os: std::env::consts::OS,
        target_arch: std::env::consts::ARCH,
        family: std::env::consts::FAMILY,
    }
}

/// Build and run the desktop application. Shared by the desktop binary
/// ([`crate`]'s `src/main.rs`) and the mobile entry point (Tauri v2, P5).
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // single-instance MUST be the first plugin (Tauri requirement): it
        // intercepts a second launch and focuses the existing window instead of
        // opening a duplicate.
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_log::Builder::new().build())
        .invoke_handler(tauri::generate_handler![aphrody_exec, aphrody_meta])
        .run(tauri::generate_context!())
        .expect("aphrody desktop: fatal error while running the Tauri application");
}
