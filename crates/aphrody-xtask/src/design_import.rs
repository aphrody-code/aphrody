// design_import.rs — Rust port of scripts/design-systems-import.ts and
// scripts/design-templates-import.ts.
//
// Imports design-system / design-template payloads from upstream
// open-design clones into `assets/design-systems/` and `assets/design-templates/`.
//
// TODO Phase 2 follow-up: the TS source pulls source data from
// `C:/src/open-design` and emits per-system manifest + tokens.css
// + DESIGN.md across ~10 systems. Full implementation requires walking
// the open-design tree, parsing tokens definitions, and re-emitting
// CSS. This stub validates the worktree presence and surfaces
// instructions to refresh manually.

use std::{
    path::PathBuf,
    process::Command,
};

use anyhow::Result;
use clap::{Args as ClapArgs, Subcommand};

const DEFAULT_OPEN_DESIGN: &str = "C:/src/open-design";

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    #[command(subcommand)]
    pub op: Op,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Op {
    /// Import design-systems into assets/design-systems/.
    Systems(SystemsArgs),
    /// Import design-templates into assets/design-templates/.
    Templates(TemplatesArgs),
}

#[derive(Debug, ClapArgs)]
pub(crate) struct SystemsArgs {
    /// Path to the open-design clone.
    #[arg(long, default_value = DEFAULT_OPEN_DESIGN)]
    source: PathBuf,

    /// Only re-import the specified system slug (e.g. `apple,m3`).
    #[arg(long)]
    only: Option<String>,

    /// Dry-run mode.
    #[arg(long)]
    dry_run: bool,
}

#[derive(Debug, ClapArgs)]
pub(crate) struct TemplatesArgs {
    /// Path to the open-design clone.
    #[arg(long, default_value = DEFAULT_OPEN_DESIGN)]
    source: PathBuf,

    /// Only re-import the specified template slug.
    #[arg(long)]
    only: Option<String>,

    /// Dry-run mode.
    #[arg(long)]
    dry_run: bool,
}

pub(crate) fn run(args: Args) -> Result<()> {
    match args.op {
        Op::Systems(a) => run_systems(a),
        Op::Templates(a) => run_templates(a),
    }
}

fn run_systems(args: SystemsArgs) -> Result<()> {
    println!("[design-import systems] source={}", args.source.display());
    if !args.source.exists() {
        eprintln!("ERROR: open-design clone not found at {}", args.source.display());
        eprintln!("hint: `cargo xtask worktree-setup --only=open-design`");
        std::process::exit(1);
    }
    if let Some(only) = &args.only {
        println!("[design-import systems] filter: {only}");
    }
    if args.dry_run {
        println!("[design-import systems] DRY RUN \u{2014} no files written");
    }
    println!(
        "[design-import systems] TODO Phase 2 follow-up: walk {}/systems/* and \
         emit per-system assets/design-systems/<slug>/{{DESIGN.md,tokens.css,components.html}}.",
        args.source.display()
    );
    surface_open_design_inventory(&args.source, "systems");
    Ok(())
}

fn run_templates(args: TemplatesArgs) -> Result<()> {
    println!("[design-import templates] source={}", args.source.display());
    if !args.source.exists() {
        eprintln!("ERROR: open-design clone not found at {}", args.source.display());
        eprintln!("hint: `cargo xtask worktree-setup --only=open-design`");
        std::process::exit(1);
    }
    if let Some(only) = &args.only {
        println!("[design-import templates] filter: {only}");
    }
    if args.dry_run {
        println!("[design-import templates] DRY RUN \u{2014} no files written");
    }
    println!(
        "[design-import templates] TODO Phase 2 follow-up: walk {}/templates/* and \
         emit per-template assets/design-templates/<slug>/{{SKILL.md,example.html}}.",
        args.source.display()
    );
    surface_open_design_inventory(&args.source, "templates");
    Ok(())
}

fn surface_open_design_inventory(source: &PathBuf, kind: &str) {
    let probe = source.join(kind);
    if probe.exists() {
        let out = Command::new("git")
            .arg("-C").arg(source)
            .args(["rev-parse", "HEAD"])
            .output();
        if let Ok(o) = out {
            if o.status.success() {
                println!(
                    "[design-import {kind}] open-design HEAD = {}",
                    String::from_utf8_lossy(&o.stdout).trim()
                );
            }
        }
        if let Ok(rd) = std::fs::read_dir(&probe) {
            let count = rd.filter_map(Result::ok).filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false)).count();
            println!("[design-import {kind}] found {count} directories under {}", probe.display());
        }
    }
}
