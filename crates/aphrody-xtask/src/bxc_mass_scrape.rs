// bxc_mass_scrape.rs — Rust port of scripts/bxc-mass-scrape.ts.
//
// Mass-scrape URLs via the bxc-engine. The TS source spawns N concurrent
// bxc pages with curl-impersonate / Chrome backends.
//
// TODO Phase 2 follow-up: full concurrent scrape pipeline. This stub
// validates inputs and emits a minimal manifest. For a working
// scraper today, use `crates/bxc-engine` directly or
// `cargo xtask edge-scrape`.

use std::{
    fs,
    path::PathBuf,
};

use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use serde::{Deserialize, Serialize};

const DEFAULT_BXC_ROOT: &str = "C:/src/bxc";

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Path to a cloned bxc repo (default: C:/src/bxc or $BXC_ROOT).
    #[arg(long)]
    bxc: Option<PathBuf>,

    /// URL list JSON file.
    #[arg(long)]
    urls: Option<PathBuf>,

    /// Cache directory.
    #[arg(long)]
    cache: Option<PathBuf>,

    /// Maximum concurrent bxc pages (1..32).
    #[arg(long, default_value = "6")]
    concurrency: usize,

    /// Profile (static | http | fast | stealth | max).
    #[arg(long, default_value = "static")]
    profile: String,

    /// Mode (static | full).
    #[arg(long, default_value = "static")]
    mode: String,

    /// Per-URL timeout (ms).
    #[arg(long, default_value = "60000")]
    timeout: u64,

    /// Retry attempts per URL (0..4).
    #[arg(long, default_value = "2")]
    retry: u32,

    /// Force re-scrape.
    #[arg(long)]
    force: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct UrlEntry { url: String, #[serde(default)] category: String, #[serde(default)] tags: Vec<String> }

#[derive(Debug, Deserialize)]
struct UrlsDoc { urls: Vec<UrlEntry> }

pub(crate) fn run(args: Args) -> Result<()> {
    let bxc_root = args
        .bxc
        .or_else(|| std::env::var_os("BXC_ROOT").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_BXC_ROOT));
    let cache = args.cache.unwrap_or_else(|| {
        std::env::current_dir().unwrap().join("var").join("data").join("bxc-cache")
    });
    fs::create_dir_all(&cache).context("mkdir cache")?;

    let urls_path = args.urls.unwrap_or_else(|| {
        std::env::current_dir().unwrap().join("scripts").join("bxc-mass-scrape.urls.json")
    });

    if !urls_path.exists() {
        eprintln!("URL list not found: {}", urls_path.display());
        std::process::exit(1);
    }

    println!(
        "[bxc-mass-scrape] bxc-root={} urls={} cache={} concurrency={} profile={} mode={}",
        bxc_root.display(), urls_path.display(), cache.display(),
        args.concurrency, args.profile, args.mode,
    );
    println!("[bxc-mass-scrape] timeout={}ms retry={} force={}", args.timeout, args.retry, args.force);

    let body = fs::read_to_string(&urls_path)?;
    let doc: UrlsDoc = serde_json::from_str(&body)?;
    println!("[bxc-mass-scrape] {} URLs loaded", doc.urls.len());

    if !bxc_root.exists() {
        eprintln!(
            "WARN: bxc-root not found at {}. Run `cargo xtask worktree-setup --only=bxc`.",
            bxc_root.display()
        );
    }

    // TODO Phase 2 follow-up: drive crates/bxc-engine in-process via async
    // tokio workers. For now, surface the action plan and emit a minimal
    // manifest so callers can fall back to edge-scrape or scripts/aphrody.
    let manifest = serde_json::json!({
        "$schema": "https://aphrody-code.dev/schemas/bxc-mass-scrape-manifest/v1",
        "engine": "bxc",
        "bxcRoot": bxc_root.to_string_lossy(),
        "urlsFile": urls_path.to_string_lossy(),
        "cache": cache.to_string_lossy(),
        "total": doc.urls.len(),
        "status": "stub \u{2014} use cargo xtask edge-scrape or crates/bxc-engine directly",
    });
    let manifest_path = cache.join("manifest.json");
    fs::write(&manifest_path, format!("{}\n", serde_json::to_string_pretty(&manifest)?))?;
    println!("[bxc-mass-scrape] wrote stub manifest {}", manifest_path.display());
    Ok(())
}
