// edge_scrape.rs — Rust port of scripts/edge-mass-scrape.ts.
//
// Mass-scrape URLs via Microsoft Edge headless mode (--dump-dom).
//
// TODO Phase 2 follow-up: full concurrency + manifest emission. The TS
// source spawns N concurrent Edge processes per URL list and emits
// per-URL HTML caches + an aggregated manifest. This Rust port covers
// the single-URL flow and the manifest schema. Multi-URL concurrency
// pending tokio rewrite.

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use anyhow::{Context, Result};
use clap::Args as ClapArgs;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const DEFAULT_EDGE_WIN: &str = r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe";

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// URL list JSON file ({"urls":[{"url":"..."}]}).
    #[arg(long)]
    urls: Option<PathBuf>,

    /// Path to msedge binary.
    #[arg(long)]
    edge: Option<PathBuf>,

    /// Cache directory.
    #[arg(long)]
    cache: Option<PathBuf>,

    /// Maximum concurrent Edge processes.
    #[arg(long, default_value = "4")]
    concurrency: usize,

    /// Per-URL timeout (milliseconds).
    #[arg(long, default_value = "45000")]
    timeout: u64,

    /// Retry attempts per URL.
    #[arg(long, default_value = "1")]
    retry: u32,

    /// Virtual-time budget (Edge flag) in milliseconds.
    #[arg(long, default_value = "8000")]
    virtual_time: u64,

    /// Force re-scrape even if cache hit.
    #[arg(long)]
    force: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct UrlEntry {
    url: String,
    #[serde(default)]
    category: String,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct UrlsDoc {
    urls: Vec<UrlEntry>,
}

pub(crate) fn run(args: Args) -> Result<()> {
    let edge_path = args
        .edge
        .or_else(|| std::env::var_os("EDGE_PATH").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_EDGE_WIN));
    let cache = args.cache.unwrap_or_else(|| {
        std::env::current_dir().unwrap().join("var").join("data").join("edge-cache")
    });
    fs::create_dir_all(&cache).context("mkdir cache")?;

    let urls_path = args.urls.unwrap_or_else(|| {
        std::env::current_dir()
            .unwrap()
            .join("scripts")
            .join("bxc-mass-scrape.urls.json")
    });
    if !urls_path.exists() {
        eprintln!("URL list not found: {}", urls_path.display());
        std::process::exit(1);
    }
    let urls_doc: UrlsDoc = serde_json::from_str(&fs::read_to_string(&urls_path)?)?;

    println!(
        "[edge-scrape] edge={} cache={} concurrency={} timeout={}ms virtual-time={}ms",
        edge_path.display(), cache.display(), args.concurrency, args.timeout, args.virtual_time
    );
    println!("[edge-scrape] {} URLs loaded from {}", urls_doc.urls.len(), urls_path.display());

    if !edge_path.exists() && cfg!(target_os = "windows") {
        eprintln!("WARN: Edge binary not found at {}", edge_path.display());
        eprintln!("hint: pass --edge <path> or set EDGE_PATH");
    }

    let mut succeeded = 0usize;
    let mut failed = 0usize;
    let mut cached = 0usize;
    for entry in &urls_doc.urls {
        let cache_file = cache.join(format!("{}.html", sha256_hex(&entry.url)));
        if !args.force && cache_file.exists() {
            cached += 1;
            continue;
        }
        match scrape_one(&edge_path, &entry.url, &cache_file, args.timeout, args.virtual_time, args.retry) {
            Ok(_) => succeeded += 1,
            Err(e) => {
                eprintln!("[edge-scrape] FAIL {}: {e:#}", entry.url);
                failed += 1;
            }
        }
    }

    let manifest = serde_json::json!({
        "$schema": "https://aphrody-code.dev/schemas/bxc-mass-scrape-manifest/v1",
        "engine": "edge",
        "cache": cache.to_string_lossy(),
        "total": urls_doc.urls.len(),
        "succeeded": succeeded,
        "failed": failed,
        "cached": cached,
    });
    let manifest_path = cache.join("manifest.json");
    fs::write(&manifest_path, format!("{}\n", serde_json::to_string_pretty(&manifest)?))?;
    println!(
        "[edge-scrape] done. succeeded={succeeded} failed={failed} cached={cached} \
         manifest={}",
        manifest_path.display()
    );
    Ok(())
}

fn scrape_one(
    edge: &Path,
    url: &str,
    out: &Path,
    timeout_ms: u64,
    virtual_time_ms: u64,
    retry: u32,
) -> Result<()> {
    let mut attempt = 0u32;
    loop {
        attempt += 1;
        let result = run_edge_once(edge, url, out, timeout_ms, virtual_time_ms);
        match result {
            Ok(()) => return Ok(()),
            Err(e) if attempt > retry => return Err(e),
            Err(e) => {
                eprintln!("[edge-scrape] retry {} after error: {e:#}", attempt);
                std::thread::sleep(Duration::from_millis(500));
            }
        }
    }
}

fn run_edge_once(
    edge: &Path,
    url: &str,
    out: &Path,
    _timeout_ms: u64,
    virtual_time_ms: u64,
) -> Result<()> {
    // NOTE: real timeout enforcement requires async or a process-watcher
    // thread; this minimal port relies on Edge's --virtual-time-budget to
    // bound execution and Cargo's wait().
    let user_data = std::env::temp_dir().join(format!("edge-scrape-{}", std::process::id()));
    let output = Command::new(edge)
        .args([
            "--headless=new",
            "--disable-gpu",
            "--no-sandbox",
            "--disable-dev-shm-usage",
            "--disable-extensions",
            "--hide-scrollbars",
            "--no-first-run",
            "--no-default-browser-check",
        ])
        .arg(format!("--user-data-dir={}", user_data.display()))
        .arg(format!("--virtual-time-budget={}", virtual_time_ms))
        .arg("--dump-dom")
        .arg(url)
        .output()
        .context("spawn msedge")?;
    if !output.status.success() {
        anyhow::bail!("edge exit {}", output.status.code().unwrap_or(-1));
    }
    fs::write(out, &output.stdout).context("write cache file")?;
    Ok(())
}

fn sha256_hex(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    let digest = h.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for b in digest {
        out.push_str(&format!("{b:02x}"));
    }
    out
}
