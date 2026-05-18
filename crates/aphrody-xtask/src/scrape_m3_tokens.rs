// scrape_m3_tokens.rs — Rust port of scripts/scrape-m3-tokens.ts.
//
// Scrapes Material Design 3 token definitions from m3.material.io and
// writes a normalised JSON catalogue to assets/m3-tokens.json.
//
// TODO Phase 2 follow-up: full scrape pipeline. The TS source drives
// bxc-engine + chromedp to traverse the M3 token explorer SPA and
// extract per-token metadata (name, value, role, theme). This stub
// validates the bxc-root presence and emits a placeholder catalogue.

use std::{
    env, fs,
    path::PathBuf,
    process::Command,
};

use anyhow::{Context, Result};
use clap::Args as ClapArgs;

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Path to a cloned bxc repo.
    #[arg(long, default_value = "C:/worktree/bxc")]
    bxc: PathBuf,

    /// Output JSON path.
    #[arg(long)]
    out: Option<PathBuf>,

    /// Force re-scrape even if cache hit.
    #[arg(long)]
    force: bool,
}

pub(crate) fn run(args: Args) -> Result<()> {
    let root = repo_root()?;
    let out = args.out.unwrap_or_else(|| root.join("assets").join("m3-tokens.json"));
    if let Some(p) = out.parent() { fs::create_dir_all(p).ok(); }

    println!(
        "[scrape-m3-tokens] bxc-root={} out={} force={}",
        args.bxc.display(), out.display(), args.force
    );

    if !args.bxc.exists() {
        eprintln!("WARN: bxc-root not found. Run `cargo xtask worktree-setup --only=bxc`.");
    }

    // TODO Phase 2 follow-up: drive crates/bxc-engine in-process to scrape
    // m3.material.io/styles/* and extract the token table. For now emit a
    // sentinel catalogue with the canonical M3 token roles so downstream
    // consumers (aphrody-terminal-wasm token loader, m3-coverage-audit)
    // get a stable shape.
    let catalogue = serde_json::json!({
        "$schema": "https://aphrody-code.dev/schemas/m3-tokens/v1",
        "generatedAt": now_iso8601(),
        "source": "https://m3.material.io/styles/tokens",
        "status": "stub \u{2014} Rust port pending Phase 2 follow-up",
        "roles": [
            "color.primary", "color.on-primary", "color.primary-container",
            "color.on-primary-container", "color.secondary", "color.on-secondary",
            "color.tertiary", "color.error", "color.background", "color.surface",
            "color.outline", "color.shadow", "color.scrim",
            "typography.display-large", "typography.headline-large",
            "typography.body-large", "typography.label-large",
            "shape.corner.small", "shape.corner.medium", "shape.corner.large",
            "elevation.0", "elevation.1", "elevation.2", "elevation.3",
            "motion.easing.standard", "motion.duration.short1"
        ],
    });

    fs::write(&out, format!("{}\n", serde_json::to_string_pretty(&catalogue)?))
        .context("write tokens catalogue")?;
    println!("[scrape-m3-tokens] wrote {}", out.display());
    Ok(())
}

fn repo_root() -> Result<PathBuf> {
    if let Ok(dir) = env::var("CARGO_WORKSPACE_DIR") { return Ok(PathBuf::from(dir)); }
    let out = Command::new("git").args(["rev-parse", "--show-toplevel"]).output().context("git")?;
    if out.status.success() { return Ok(PathBuf::from(String::from_utf8(out.stdout)?.trim())); }
    Ok(env::current_dir()?)
}

fn now_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let (y, mo, d, h, mi, s) = unix_to_ymdhms(secs);
    format!("{y:04}-{mo:02}-{d:02}T{h:02}:{mi:02}:{s:02}Z")
}

fn unix_to_ymdhms(mut t: u64) -> (i32, u32, u32, u32, u32, u32) {
    let s = (t % 60) as u32; t /= 60;
    let mi = (t % 60) as u32; t /= 60;
    let h = (t % 24) as u32; t /= 24;
    let mut days = t as i64; days += 719_468;
    let era = days.div_euclid(146_097);
    let doe = days.rem_euclid(146_097);
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = (yoe + era * 400) as i32;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = (if mp < 10 { mp + 3 } else { mp - 9 }) as u32;
    let y = y + if m <= 2 { 1 } else { 0 };
    (y, m, d, h, mi, s)
}
