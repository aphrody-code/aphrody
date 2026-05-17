// SPDX-License-Identifier: Apache-2.0
//! Auto-generate `docs/SUMMARY.md` (mdBook table of contents) from the docs/
//! directory tree.
//!
//! Usage:
//!   cargo run -p aphrody-summary
//!   cargo run -p aphrody-summary -- --check    # CI: fail if SUMMARY.md drifts
//!
//! All generation logic lives in the library crate (`aphrody_summary`). This
//! binary is a thin wrapper that adds the on-disk write step and the
//! `--check` mode used by CI.

use std::{env, fs};

use anyhow::Result;
use aphrody_summary::{generate, repo_root};

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let check_mode = args.contains(&"--check".to_string());

    let repo_root = repo_root()?;
    let docs_root = repo_root.join("docs");
    let summary_path = docs_root.join("SUMMARY.md");

    let generated = generate()?;

    if check_mode {
        let existing = fs::read_to_string(&summary_path).unwrap_or_default();
        if existing.trim() != generated.trim() {
            eprintln!("docs/SUMMARY.md is out of date. Run: cargo run -p aphrody-summary");
            std::process::exit(1);
        }
        println!("docs/SUMMARY.md is up to date.");
        return Ok(());
    }

    fs::write(&summary_path, &generated)?;
    let line_count = generated.lines().count();
    println!("Generated docs/SUMMARY.md ({} lines)", line_count);

    Ok(())
}
