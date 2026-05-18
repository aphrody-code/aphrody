// optimize_assets.rs — Rust port of scripts/optimize-assets.ts.
//
// Walks the workspace and runs Rust-native asset optimizers on PNG / CSS
// files. SVG (svgo, Node) and AVIF transcoding (sharp-cli, C++/libvips) are
// dropped per the 100% Rust policy — re-add via `image` + `ravif` crates
// or `usvg`/`resvg` in a follow-up if needed.
//
// External tools invoked (all distributed as standalone Rust binaries):
//   - oxipng        — lossless PNG compression
//   - lightningcss  — CSS minification + bundling
//
// Usage:
//     cargo xtask optimize-assets [--root <dir>] [--ext png,css]

use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use walkdir::WalkDir;

const IGNORED_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "dist",
    "build",
    "target",
    "out",
    "test_outputs",
    "opt",
    "docs",
    "var",
    ".vscode",
    ".idea",
    ".cursor",
];

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Root directory to walk. Defaults to current working dir.
    #[arg(long)]
    root: Option<PathBuf>,

    /// Comma-separated list of extensions to process. Default: `png,css`.
    #[arg(long, default_value = "png,css")]
    ext: String,
}

pub(crate) fn run(args: Args) -> Result<()> {
    let root = match args.root {
        Some(p) => p,
        None => env::current_dir().context("get cwd")?,
    };
    let exts: Vec<String> = args
        .ext
        .split(',')
        .map(|s| s.trim().trim_start_matches('.').to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    println!(
        "\u{1f680} optimize-assets — root={} ext=[{}]",
        root.display(),
        exts.join(",")
    );

    let mut processed = 0usize;
    let mut failed = 0usize;
    for entry in WalkDir::new(&root)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !IGNORED_DIRS.iter().any(|d| *d == name.as_ref())
        })
        .filter_map(Result::ok)
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        let ext_lc = ext.to_ascii_lowercase();
        if !exts.contains(&ext_lc) {
            continue;
        }

        match ext_lc.as_str() {
            "png" => {
                if let Err(e) = optimize_png(path) {
                    eprintln!("\u{274c} oxipng failed on {}: {e:#}", path.display());
                    failed += 1;
                    continue;
                }
            }
            "css" => {
                if let Err(e) = optimize_css(path) {
                    eprintln!("\u{274c} lightningcss failed on {}: {e:#}", path.display());
                    failed += 1;
                    continue;
                }
            }
            _ => continue,
        }
        processed += 1;
    }

    println!(
        "\u{2705} optimize-assets — done. {processed} processed, {failed} failed."
    );
    if failed > 0 {
        std::process::exit(1);
    }
    Ok(())
}

fn optimize_png(file: &Path) -> Result<()> {
    println!("[oxipng] {}", file.display());
    let status = Command::new("oxipng")
        .args(["-o", "4", "--strip", "safe", "-a"])
        .arg(file)
        .status()
        .context("spawn oxipng (install: cargo install oxipng)")?;
    if !status.success() {
        anyhow::bail!("oxipng exited with {status}");
    }
    Ok(())
}

fn optimize_css(file: &Path) -> Result<()> {
    println!("[lightningcss] {}", file.display());
    let status = Command::new("lightningcss")
        .args(["--minify", "--bundle", "--targets", ">= 0.25%"])
        .arg(file)
        .arg("-o")
        .arg(file)
        .status()
        .context("spawn lightningcss (install: cargo install lightningcss-cli)")?;
    if !status.success() {
        anyhow::bail!("lightningcss exited with {status}");
    }
    Ok(())
}
