// aphrody-xtask — workspace dev tasks (Rust port of scripts/*.ts).
//
// Invoked via the alias `cargo xtask <op>` (`.cargo/config.toml`).
// Each sub-command is a former Bun/Node/Shell script reimplemented in
// pure Rust. See `docs/PLAN_RUST_ONLY.md` Phase 2 for the full port table.

use anyhow::Result;
use clap::{Parser, Subcommand};

mod optimize_assets;
mod runtimes_detect;
mod skills_sync;

#[derive(Parser)]
#[command(
    name = "aphrody-xtask",
    version,
    about = "aphrody — workspace dev tasks (replaces scripts/*.ts)",
    long_about = "Cargo-driven xtask runner. Each sub-command replaces a former\n\
                  Bun/Node/Shell script. Run `cargo xtask <op> --help` for details."
)]
struct Cli {
    #[command(subcommand)]
    op: Op,
}

#[derive(Subcommand)]
enum Op {
    /// Sync a remote SKILL.md catalog into ./.claude/skills/.
    /// Replaces scripts/skills-sync.ts (Bun).
    SkillsSync(skills_sync::Args),

    /// Optimize repo assets (PNG via oxipng, CSS via lightningcss).
    /// Replaces scripts/optimize-assets.ts (Bun).
    OptimizeAssets(optimize_assets::Args),

    /// Probe CLI runtimes on PATH and emit a JSON manifest.
    /// Replaces scripts/runtimes-detect.ts (Bun). Bun/Node/Lightpanda/n2b
    /// entries dropped per the 100% Rust policy.
    RuntimesDetect(runtimes_detect::Args),
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(false)
        .init();

    let cli = Cli::parse();
    match cli.op {
        Op::SkillsSync(args) => skills_sync::run(args),
        Op::OptimizeAssets(args) => optimize_assets::run(args),
        Op::RuntimesDetect(args) => runtimes_detect::run(args),
    }
}
