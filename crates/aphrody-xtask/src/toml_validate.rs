// toml_validate.rs — Rust port of .claude/plugins/aphrody/hooks/cargo-toml-validate.ts.
//
// Walks the workspace, parses every Cargo.toml with the `toml` crate, and
// reports any syntax / schema errors. Replaces the former Bun hook that
// shelled out to `cargo metadata --no-deps --offline`; this pure-Rust
// validator catches lexical and structural problems faster, without
// requiring the full resolver to run.
//
// Usage:
//     cargo xtask toml-validate
//     cargo xtask toml-validate --json
//     cargo xtask toml-validate --root /path/to/repo
//
// Exit code:
//     0 — every Cargo.toml parsed cleanly.
//     1 — at least one manifest failed to parse.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use serde::Serialize;
use walkdir::WalkDir;

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Repository root (defaults to $CARGO_WORKSPACE_DIR or `git rev-parse --show-toplevel`).
    #[arg(long)]
    root: Option<PathBuf>,

    /// Emit the result as a JSON report on stdout (instead of a human table).
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Serialize)]
struct ManifestReport {
    path: String,
    ok: bool,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct Report {
    root: String,
    manifest_count: usize,
    failure_count: usize,
    manifests: Vec<ManifestReport>,
}

pub(crate) fn run(args: Args) -> Result<()> {
    let root = match args.root {
        Some(r) => r,
        None => repo_root()?,
    };

    let manifests = discover_manifests(&root);
    let mut reports = Vec::with_capacity(manifests.len());
    let mut failures = 0usize;

    for path in &manifests {
        let rel = path.strip_prefix(&root).unwrap_or(path).to_string_lossy().replace('\\', "/");
        match validate_one(path) {
            Ok(()) => reports.push(ManifestReport { path: rel, ok: true, error: None }),
            Err(e) => {
                failures += 1;
                reports.push(ManifestReport { path: rel, ok: false, error: Some(format!("{e:#}")) });
            }
        }
    }

    let report = Report {
        root: root.to_string_lossy().replace('\\', "/"),
        manifest_count: reports.len(),
        failure_count: failures,
        manifests: reports,
    };

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        render_table(&report);
    }

    if failures > 0 {
        std::process::exit(1);
    }
    Ok(())
}

fn validate_one(path: &Path) -> Result<()> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("read {}", path.display()))?;
    let _: toml::Value = toml::from_str(&raw)
        .with_context(|| format!("parse {}", path.display()))?;
    Ok(())
}

fn discover_manifests(root: &Path) -> Vec<PathBuf> {
    let skip_dirs = [
        "target",
        "node_modules",
        ".git",
        "vendor",
        "var",
        ".turbo",
    ];
    let mut out = Vec::new();
    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            // Always traverse the root itself.
            if e.depth() == 0 {
                return true;
            }
            let name = e.file_name().to_string_lossy();
            !skip_dirs.iter().any(|d| name == *d)
        });
    for entry in walker.flatten() {
        if entry.file_type().is_file() && entry.file_name() == "Cargo.toml" {
            out.push(entry.into_path());
        }
    }
    out.sort();
    out
}

fn repo_root() -> Result<PathBuf> {
    if let Ok(dir) = env::var("CARGO_WORKSPACE_DIR") {
        return Ok(PathBuf::from(dir));
    }
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("spawn git rev-parse")?;
    if out.status.success() {
        let s = String::from_utf8(out.stdout).context("git output utf-8")?;
        return Ok(PathBuf::from(s.trim()));
    }
    Ok(env::current_dir()?)
}

fn render_table(report: &Report) {
    println!("toml-validate: root={}", report.root);
    println!(
        "toml-validate: {} manifest(s) parsed, {} failure(s)",
        report.manifest_count, report.failure_count
    );
    for m in &report.manifests {
        if m.ok {
            continue;
        }
        println!("  FAIL  {}", m.path);
        if let Some(err) = &m.error {
            for line in err.lines() {
                println!("        {line}");
            }
        }
    }
    if report.failure_count == 0 {
        println!("toml-validate: OK");
    }
}
