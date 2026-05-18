// design_google.rs — Rust port of scripts/design-google-ingest.ts and
// scripts/design-google-curate.ts.
//
// Two sub-commands:
//   cargo xtask design-google ingest [--force-refresh]
//   cargo xtask design-google curate [--in <path>] [--out <path>]
//
// `ingest` discovers /library/* hrefs from https://design.google/library
// (optionally via cached HTML), unions with the curated seed list, and
// writes scripts/design-google.urls.json.
//
// NOTE: `curate` is a best-effort minimal port — the TS source is a
// 399-line interactive curation pipeline that consumes the bxc-cache JSON
// schema. Full port pending Phase 2 follow-up; this stub validates input
// shape and copies through.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result};
use clap::{Args as ClapArgs, Subcommand};
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const LIBRARY_URL: &str = "https://design.google/library";

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    #[command(subcommand)]
    pub op: Op,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Op {
    /// URL-discovery orchestrator. Writes scripts/design-google.urls.json.
    Ingest(IngestArgs),
    /// Curate the scraped design.google corpus (best-effort minimal port).
    Curate(CurateArgs),
}

#[derive(Debug, ClapArgs)]
pub(crate) struct IngestArgs {
    /// Bypass on-disk HTML cache.
    #[arg(long)]
    force_refresh: bool,

    /// Override the Edge cache directory (default: var/data/edge-cache).
    #[arg(long)]
    cache: Option<PathBuf>,
}

#[derive(Debug, ClapArgs)]
pub(crate) struct CurateArgs {
    /// Input bxc-scrape manifest (default: var/data/edge-cache/manifest.json).
    #[arg(long, name = "in")]
    input: Option<PathBuf>,

    /// Output curation report (default: assets/design-google/curated.json).
    #[arg(long)]
    out: Option<PathBuf>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct UrlEntry {
    url: String,
    category: String,
    tags: Vec<String>,
}

fn seed_site_pages() -> Vec<UrlEntry> {
    vec![
        UrlEntry { url: "https://design.google/".into(), category: "site".into(), tags: vec!["home".into()] },
        UrlEntry { url: "https://design.google/about".into(), category: "site".into(), tags: vec!["about".into()] },
        UrlEntry { url: "https://design.google/events".into(), category: "site".into(), tags: vec!["events".into()] },
        UrlEntry { url: "https://design.google/products".into(), category: "site".into(), tags: vec!["products".into()] },
        UrlEntry { url: "https://design.google/library".into(), category: "site".into(), tags: vec!["library-index".into()] },
    ]
}

fn seed_library_articles() -> Vec<UrlEntry> {
    let mk = |slug: &str, tags: &[&str]| UrlEntry {
        url: format!("https://design.google/library/{slug}"),
        category: "library".into(),
        tags: tags.iter().map(|s| (*s).to_string()).collect(),
    };
    vec![
        mk("gemini-ai-visual-design", &["gemini"]),
        mk("google-sans-flex-font", &["brand", "type"]),
        mk("expressive-material-design-google-research", &["m3"]),
        mk("design-notes-material-3-expressive-liam-spradlin", &["m3"]),
        mk("material-design-3", &["m3"]),
        mk("material-3-design-tokens", &["m3", "tokens"]),
        mk("color-theory-ruxandra-duru", &["foundations", "color"]),
        mk("david-reinfurt-teaches-design", &["foundations"]),
        mk("hardware-and-software-can-work-together", &["foundations"]),
        mk("open-source-custom-fonts-google", &["brand", "type"]),
        mk("transparent-screens", &["brand"]),
        mk("ux-design-system-dance", &["foundations", "ux"]),
        mk("accessibility", &["foundations", "a11y"]),
        mk("design-systems", &["foundations"]),
    ]
}

fn sha256_hex(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    hex(&hasher.finalize())
}

fn hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
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
        return Ok(PathBuf::from(String::from_utf8(out.stdout)?.trim()));
    }
    Ok(env::current_dir()?)
}

pub(crate) fn run(args: Args) -> Result<()> {
    match args.op {
        Op::Ingest(a) => run_ingest(a),
        Op::Curate(a) => run_curate(a),
    }
}

fn run_ingest(args: IngestArgs) -> Result<()> {
    let root = repo_root()?;
    let cache_dir = args
        .cache
        .unwrap_or_else(|| root.join("var").join("data").join("edge-cache"));
    let output = root
        .join("scripts")
        .join("design-google.urls.json");

    fs::create_dir_all(&cache_dir).ok();

    let cache_path = cache_dir.join(format!("{}.html", sha256_hex(LIBRARY_URL)));

    let discovered = if !args.force_refresh && cache_path.exists() {
        let meta = fs::metadata(&cache_path)?;
        if meta.len() > 8_000 {
            println!("[discover] using cached library index ({} B)", meta.len());
            let html = fs::read_to_string(&cache_path)?;
            extract_library_hrefs(&html)
        } else {
            println!(
                "[discover] cache present but too small ({} B); using seeds only",
                meta.len()
            );
            Vec::new()
        }
    } else {
        // NOTE: Edge headless spawn omitted in the Rust port — the TS source
        // shells out to msedge --headless=new --dump-dom which is environment-
        // specific. To refresh hrefs, place a cached HTML at
        //     <cache>/<sha256(LIBRARY_URL)>.html
        // (e.g. via `cargo xtask edge-scrape --urls scripts/design-google.urls.json`).
        println!(
            "[discover] no cache at {} \u{2014} using seed catalogue only",
            cache_path.display()
        );
        Vec::new()
    };

    let merged = union_entries(&[seed_site_pages(), seed_library_articles(), discovered]);

    let doc = serde_json::json!({
        "$schema": "https://aphrody-code.dev/schemas/bxc-mass-scrape-urls/v1",
        "version": "1.0.0",
        "description": "design.google end-to-end ingest URL list \u{2014} discovered via \
            cargo xtask design-google ingest + seed catalogue.",
        "urls": merged,
    });
    fs::create_dir_all(output.parent().unwrap_or(Path::new("."))).ok();
    fs::write(&output, format!("{}\n", serde_json::to_string_pretty(&doc)?))
        .context("write design-google.urls.json")?;

    let mut by_category: std::collections::BTreeMap<&str, usize> = std::collections::BTreeMap::new();
    for e in &merged {
        *by_category.entry(e.category.as_str()).or_insert(0) += 1;
    }
    println!("[ingest] wrote {}", output.display());
    println!("[ingest] total URLs: {}", merged.len());
    for (cat, n) in by_category {
        println!("[ingest]   {:<8} {}", cat, n);
    }
    Ok(())
}

fn extract_library_hrefs(html: &str) -> Vec<UrlEntry> {
    let re = Regex::new(r#"href="(/library/[a-z0-9-]+)""#).expect("static regex");
    let mut seen = std::collections::BTreeSet::new();
    for cap in re.captures_iter(html) {
        if let Some(m) = cap.get(1) {
            seen.insert(format!("https://design.google{}", m.as_str()));
        }
    }
    seen.into_iter()
        .map(|url| UrlEntry { url, category: "library".into(), tags: vec!["discovered".into()] })
        .collect()
}

fn union_entries(sources: &[Vec<UrlEntry>]) -> Vec<UrlEntry> {
    let mut map: std::collections::BTreeMap<String, UrlEntry> = std::collections::BTreeMap::new();
    for src in sources {
        for e in src {
            let key = e.url.to_ascii_lowercase();
            map.entry(key).or_insert_with(|| e.clone());
        }
    }
    let mut out: Vec<UrlEntry> = map.into_values().collect();
    out.sort_by(|a, b| a.url.cmp(&b.url));
    out
}

fn run_curate(args: CurateArgs) -> Result<()> {
    // NOTE: best-effort minimal port. Full TS source is 399 LOC of curation
    // heuristics over the bxc-scrape manifest schema. Phase 2 follow-up:
    // reimplement quality-scoring, dedup, semantic tagging.
    let root = repo_root()?;
    let input = args.input.unwrap_or_else(|| {
        root.join("var").join("data").join("edge-cache").join("manifest.json")
    });
    let output = args
        .out
        .unwrap_or_else(|| root.join("assets").join("design-google").join("curated.json"));

    if !input.exists() {
        println!("[curate] input manifest not found: {}", input.display());
        println!("[curate] run `cargo xtask edge-scrape` or `cargo xtask bxc-mass-scrape` first.");
        return Ok(());
    }

    let body = fs::read_to_string(&input).context("read input manifest")?;
    let v: serde_json::Value = serde_json::from_str(&body).context("parse manifest as JSON")?;

    let count = v
        .get("urls")
        .and_then(|x| x.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    println!("[curate] read {count} URL records from {}", input.display());

    fs::create_dir_all(output.parent().unwrap_or(Path::new("."))).ok();
    let curated = serde_json::json!({
        "$schema": "https://aphrody-code.dev/schemas/design-google-curated/v1",
        "generatedAt": now_iso8601(),
        "source": input.to_string_lossy(),
        "count": count,
        "status": "passthrough \u{2014} Rust port pending Phase 2 follow-up",
        "items": v.get("urls").cloned().unwrap_or(serde_json::Value::Array(vec![])),
    });
    fs::write(&output, format!("{}\n", serde_json::to_string_pretty(&curated)?))
        .context("write curated.json")?;
    println!("[curate] wrote {}", output.display());
    Ok(())
}

fn now_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    format!("@{secs}")
}
