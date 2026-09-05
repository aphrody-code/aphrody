// SPDX-License-Identifier: Apache-2.0
//! Desktop binary entry point. Thin shell over [`aphrody_app_lib::run`]
//! (mirroring the CLI's bin/lib split), keeping the mimalloc global allocator
//! local to the binary — the latency objective shared with the CLI.

// Hide the spare console window on Windows release builds (keep it in debug so
// `tauri_plugin_log` output is visible). No-op on Linux/macOS.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

fn main() {
    aphrody_app_lib::run();
}
