// SPDX-License-Identifier: Apache-2.0
//! `a2a-tui` — standalone A2A coord channel viewer.
//!
//! ```bash
//! cargo run -p a2a-ui --features native --example a2a-tui -- \
//!     --coord C:/winclean/.coord
//! ```

#![forbid(unsafe_code)]

use std::path::PathBuf;

use clap::Parser;

/// A2A coord channel native TUI viewer.
///
/// Opens the ratatui full-screen TUI for the aphrody / winclean coord mailboxes.
/// Press `q` or Ctrl+C to quit.
#[derive(Parser, Debug)]
#[command(name = "a2a-tui", version, about, long_about = None)]
struct Cli {
    /// Path to the `.coord` directory containing the JSONL mailbox files.
    ///
    /// Defaults to `C:/winclean/.coord` on Windows and
    /// `~/.coord` on other platforms if omitted.
    #[arg(long, value_name = "DIR")]
    coord: Option<PathBuf>,

    /// Auto-refresh interval in seconds.  Must be >= 1.
    #[arg(long, default_value = "2", value_name = "SECS")]
    refresh: u64,

    /// Which peer side to show on startup (`aphrody` or `winclean`).
    #[arg(long, default_value = "aphrody", value_name = "SIDE")]
    side: String,
}

fn default_coord_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        PathBuf::from(r"C:\winclean\.coord")
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("HOME")
            .map(|h| PathBuf::from(h).join(".coord"))
            .unwrap_or_else(|_| PathBuf::from(".coord"))
    }
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let coord_dir = cli.coord.unwrap_or_else(default_coord_dir);

    if !coord_dir.exists() {
        anyhow::bail!(
            "coord directory does not exist: {}",
            coord_dir.display()
        );
    }

    let refresh_secs = cli.refresh.max(1);

    let args = a2a_ui::native::NativeTuiArgs {
        refresh_secs,
        initial_side: cli.side,
    };

    a2a_ui::native::run_native_tui(&coord_dir, args)
}
