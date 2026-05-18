// aphrody-xtask — workspace dev tasks (Rust port of scripts/*.ts).
//
// Invoked via the alias `cargo xtask <op>` (`.cargo/config.toml`).
// Each sub-command is a former Bun/Node/Shell script reimplemented in
// pure Rust. See `docs/PLAN_RUST_ONLY.md` Phase 2 for the full port table.

use anyhow::Result;
use clap::{Parser, Subcommand};

mod audit_openclaw;
mod audit_openclaw_ext;
mod bxc_mass_scrape;
mod design_google;
mod design_import;
mod edge_scrape;
mod m3_audit;
mod optimize_assets;
mod plugin_port;
mod runtimes_detect;
mod scrape_m3_tokens;
mod skill_schema_check;
mod skills_harvest;
mod skills_sync;
mod skills_watch;
mod toml_validate;
mod worktree_check;
mod worktree_setup;

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

    /// Parse every Cargo.toml in the workspace and report schema errors.
    /// Replaces .claude/plugins/aphrody/hooks/cargo-toml-validate.ts (Bun).
    TomlValidate(toml_validate::Args),

    /// CI/pre-commit gate verifying every hardcoded worktree exists.
    /// Replaces scripts/check-worktrees.ts.
    WorktreeCheck(worktree_check::Args),

    /// Bootstrap the local worktree clones (git clone --depth 1).
    /// Replaces scripts/setup-worktrees.ts.
    WorktreeSetup(worktree_setup::Args),

    /// Vendor @openclaw/plugin-package-contract into packages/.
    /// Replaces scripts/plugin-contract-port.ts.
    PluginPort(plugin_port::Args),

    /// design.google ingest + curate pipeline.
    /// Replaces scripts/design-google-{ingest,curate}.ts.
    DesignGoogle(design_google::Args),

    /// Import design-systems / design-templates from upstream open-design.
    /// Replaces scripts/design-{systems,templates}-import.ts.
    DesignImport(design_import::Args),

    /// Rank openclaw extensions by port-priority.
    /// Replaces scripts/openclaw-extensions-audit.ts.
    AuditOpenclawExt(audit_openclaw_ext::Args),

    /// Cross-repo diff of aphrody / open-design / openclaw inventories.
    /// Replaces scripts/aphrody-vs-open-design-openclaw.audit.ts.
    AuditOpenclaw(audit_openclaw::Args),

    /// Validate skill schemas + emit `.od.yaml` sidecars.
    /// Replaces scripts/skill-schema-align.ts.
    SkillSchemaCheck(skill_schema_check::Args),

    /// Mass-scrape URLs via Microsoft Edge headless.
    /// Replaces scripts/edge-mass-scrape.ts.
    EdgeScrape(edge_scrape::Args),

    /// Mass-scrape URLs via the bxc-engine (stub: use edge-scrape today).
    /// Replaces scripts/bxc-mass-scrape.ts.
    BxcMassScrape(bxc_mass_scrape::Args),

    /// Harvest SKILL.md inventories from open-design / vercel clones.
    /// Replaces scripts/skills-harvest-open-design.ts.
    SkillsHarvest(skills_harvest::Args),

    /// Watch `.claude/skills` and emit reload signals.
    /// Replaces scripts/skills-hot-reload.ts.
    SkillsWatch(skills_watch::Args),

    /// Scrape Material Design 3 tokens into assets/m3-tokens.json.
    /// Replaces scripts/scrape-m3-tokens.ts.
    ScrapeM3Tokens(scrape_m3_tokens::Args),

    /// Audit CSS/HTML files for Material Design 3 token coverage.
    /// Replaces scripts/m3-coverage-audit.ts.
    M3Audit(m3_audit::Args),
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
        Op::TomlValidate(args) => toml_validate::run(args),
        Op::WorktreeCheck(args) => worktree_check::run(args),
        Op::WorktreeSetup(args) => worktree_setup::run(args),
        Op::PluginPort(args) => plugin_port::run(args),
        Op::DesignGoogle(args) => design_google::run(args),
        Op::DesignImport(args) => design_import::run(args),
        Op::AuditOpenclawExt(args) => audit_openclaw_ext::run(args),
        Op::AuditOpenclaw(args) => audit_openclaw::run(args),
        Op::SkillSchemaCheck(args) => skill_schema_check::run(args),
        Op::EdgeScrape(args) => edge_scrape::run(args),
        Op::BxcMassScrape(args) => bxc_mass_scrape::run(args),
        Op::SkillsHarvest(args) => skills_harvest::run(args),
        Op::SkillsWatch(args) => skills_watch::run(args),
        Op::ScrapeM3Tokens(args) => scrape_m3_tokens::run(args),
        Op::M3Audit(args) => m3_audit::run(args),
    }
}
