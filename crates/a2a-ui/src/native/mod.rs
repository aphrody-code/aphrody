// SPDX-License-Identifier: Apache-2.0
//! Native ratatui TUI backend for the A2A coord channel viewer.
//!
//! Gated behind the `native` cargo feature — the wasm32 cdylib target
//! never sees this module, keeping the WASM bundle lean.
//!
//! # Entry point
//!
//! ```no_run
//! use std::path::Path;
//! use a2a_ui::native::{NativeTuiArgs, run_native_tui};
//!
//! run_native_tui(Path::new("C:/winclean/.coord"), NativeTuiArgs::default()).unwrap();
//! ```

pub mod app;
pub mod key;
pub mod mailbox;
pub mod ui;

use std::path::Path;

use anyhow::Result;

/// Arguments for [`run_native_tui`].
#[derive(Debug, Clone)]
pub struct NativeTuiArgs {
    /// Refresh interval in seconds.  Defaults to `2`.
    pub refresh_secs: u64,
    /// Which peer side to show on startup: `"aphrody"` or `"winclean"`.
    pub initial_side: String,
}

impl Default for NativeTuiArgs {
    fn default() -> Self {
        Self {
            refresh_secs: 2,
            initial_side: "aphrody".to_owned(),
        }
    }
}

/// Open the full ratatui TUI showing the A2A coord channel mailboxes.
///
/// Reads `inbox-from-aphrody.jsonl` and `inbox-from-winclean.jsonl` from
/// `coord_dir`, starts the event loop, and blocks until the user quits.
///
/// # Errors
///
/// Returns an error if the terminal cannot be initialized, if the initial
/// mailbox files cannot be read (missing files are treated as empty), or if
/// a fatal I/O error occurs during the event loop.
pub fn run_native_tui(coord_dir: &Path, args: NativeTuiArgs) -> Result<()> {
    use tokio::runtime::Builder;

    let rt = Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| anyhow::anyhow!("failed to build tokio runtime: {e}"))?;

    rt.block_on(app::run(coord_dir, args))
}
