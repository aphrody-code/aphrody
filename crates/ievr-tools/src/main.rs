// SPDX-License-Identifier: Apache-2.0
//! ievr-tools CLI — analyse the IEVR binary inventory.
//!
//! # Usage
//!
//! ```text
//! ievr-tools summary [--json <path>]
//! ievr-tools by-ext  [--json <path>]
//! ievr-tools top [N] [--json <path>]
//! ```

use clap::{Parser, Subcommand};
use ievr_tools::Inventory;

/// Default path produced by the peer-Claude inventory scan.
const DEFAULT_JSON: &str = "C:/winclean/var/data/ievr-binaries-inventory.json";

#[derive(Parser)]
#[command(
    name = "ievr-tools",
    version,
    about = "Analyse the IEVR binary inventory JSON produced by the peer-Claude scan."
)]
struct Cli {
    /// Path to the inventory JSON file.
    #[arg(long, default_value = DEFAULT_JSON)]
    json: String,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Print summary statistics (total binaries, total size).
    Summary,
    /// Group binaries by file extension, sorted by total byte footprint.
    ByExt,
    /// Print the top N largest binaries (default: 10).
    Top {
        /// How many entries to display.
        #[arg(default_value_t = 10)]
        n: usize,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let inv = Inventory::load(&cli.json)?;

    match cli.command {
        Command::Summary => {
            println!("install_root : {}", inv.install_root);
            println!("ts           : {}", inv.ts);
            println!("binaries     : {}", inv.count());
            println!(
                "total size   : {} bytes  ({:.2} GB)",
                inv.total_size(),
                inv.total_size() as f64 / 1_073_741_824.0
            );
        }
        Command::ByExt => {
            println!(
                "{:>12}  {:>6}  {:>15}  {}",
                "ext", "count", "bytes", "MB"
            );
            println!("{}", "-".repeat(50));
            for (ext, count, bytes) in inv.by_ext() {
                let ext_display = if ext.is_empty() { "(none)" } else { &ext };
                println!(
                    "{:>12}  {:>6}  {:>15}  {:.2}",
                    ext_display,
                    count,
                    bytes,
                    bytes as f64 / 1_048_576.0
                );
            }
        }
        Command::Top { n } => {
            println!(
                "{:>4}  {:>15}  {}",
                "#", "bytes", "rel_path"
            );
            println!("{}", "-".repeat(70));
            for (i, b) in inv.largest(n).iter().enumerate() {
                println!("{:>4}. {:>15}  {}", i + 1, b.size_bytes, b.rel_path);
            }
        }
    }

    Ok(())
}
