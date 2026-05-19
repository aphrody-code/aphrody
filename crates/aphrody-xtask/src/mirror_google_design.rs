// mirror_google_design.rs — Breadth-first mirror of https://design.google/
// into assets/scrape/google-design/.
//
// Pipeline:
//   1. Probe local bxc daemon (127.0.0.1:<port>); if unreachable, try
//      `bxc launch --port N` background spawn; if still unreachable, fall
//      back to a direct reqwest client (User-Agent: aphrody-design-mirror/0.1).
//   2. Fetch /robots.txt once and collect Disallow rules for User-agent: *.
//   3. BFS from the seed root, depth <= --depth (default 3), max --max-pages
//      (default 200), same-origin only (design.google host).
//   4. For each accepted URL: try bxc daemon CDP fetch first
//      (CDP DOM serialization yields hydrated HTML for Next.js/React);
//      fall back to direct reqwest GET on any error.
//   5. Persist:
//        pages/<slug>.html          — rendered HTML body
//        assets/<filename>          — same-origin CSS/img/font referenced
//        manifest.ndjson            — append-only, 1 JSON object per page
//        summary.json               — totals + duration on completion
//
// Idempotent: pages already present in manifest.ndjson are skipped on rerun.
// Concurrency: futures::stream::buffer_unordered(4) — never serial.

#![allow(clippy::too_many_lines)]

use std::{
    collections::{BTreeSet, HashSet, VecDeque},
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Write},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::{Context, Result, anyhow};
use clap::Args as ClapArgs;
use futures::stream::{self, StreamExt};
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::Mutex;
use url::Url;

const SEED_URL: &str = "https://design.google/";
const TARGET_HOST: &str = "design.google";
const USER_AGENT: &str = "aphrody-design-mirror/0.1 (+https://github.com/aphrody-code/aphrody)";
const ASSET_BUDGET_PER_PAGE: usize = 12;

#[derive(Debug, ClapArgs)]
pub(crate) struct Args {
    /// Maximum pages to mirror (BFS hard cap).
    #[arg(long, default_value_t = 200)]
    max_pages: usize,

    /// Maximum link-depth from the seed.
    #[arg(long, default_value_t = 3)]
    depth: u32,

    /// Output root (default: assets/scrape/google-design).
    #[arg(long)]
    out: Option<PathBuf>,

    /// Concurrent fetch buffer.
    #[arg(long, default_value_t = 4)]
    concurrency: usize,

    /// bxc daemon port (CDP); 0 disables bxc and uses reqwest only.
    #[arg(long, default_value_t = 9222)]
    bxc_port: u16,

    /// Per-page fetch timeout (seconds).
    #[arg(long, default_value_t = 30)]
    timeout: u64,

    /// Skip same-origin asset (CSS/img/font) capture.
    #[arg(long)]
    no_assets: bool,
}

/// One entry appended to manifest.ndjson per successfully-fetched page.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PageRecord {
    url: String,
    slug: String,
    fetched_at: String,
    status: u16,
    sha256: String,
    bytes: u64,
    depth: u32,
    links_out: Vec<String>,
    source: String, // "bxc" | "reqwest"
}

/// Final summary.json shape.
#[derive(Debug, Serialize)]
struct Summary {
    seed: String,
    total_pages: usize,
    total_assets: usize,
    total_bytes: u64,
    depth_max: u32,
    started_at: String,
    completed_at: String,
    duration_ms: u128,
    bxc_used: bool,
}

/// Minimal robots.txt rules for User-agent: *.
#[derive(Debug, Default, Clone)]
struct RobotsRules {
    disallow: Vec<String>,
}

impl RobotsRules {
    /// Parse User-agent: * Disallow lines. Other agents ignored.
    fn parse(body: &str) -> Self {
        let mut active = false;
        let mut disallow = Vec::new();
        for raw in body.lines() {
            let line = raw.split('#').next().unwrap_or("").trim();
            if line.is_empty() {
                continue;
            }
            let Some((key, value)) = line.split_once(':') else { continue };
            let key = key.trim().to_ascii_lowercase();
            let value = value.trim();
            match key.as_str() {
                "user-agent" => active = value == "*",
                "disallow" if active && !value.is_empty() => {
                    disallow.push(value.to_string());
                }
                "allow" => { /* deliberately ignored — disallow-only minimal */ }
                _ => {}
            }
        }
        Self { disallow }
    }

    fn is_allowed(&self, path: &str) -> bool {
        !self.disallow.iter().any(|rule| path.starts_with(rule))
    }
}

/// Canonicalize URL: strip fragment, drop default ports, lowercase host.
fn canonicalize(raw: &str) -> Option<Url> {
    let mut u = Url::parse(raw).ok()?;
    u.set_fragment(None);
    if u.scheme() != "http" && u.scheme() != "https" {
        return None;
    }
    Some(u)
}

/// Derive a filesystem-safe slug from a URL path.
/// "/" -> "index"; "/foo/bar/" -> "foo__bar"; non [A-Za-z0-9._-] -> "_".
pub(crate) fn slug_from_url(url: &Url) -> String {
    let path = url.path();
    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() {
        return "index".into();
    }
    let mut out = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        match ch {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '.' | '-' | '_' => out.push(ch),
            '/' => out.push_str("__"),
            _ => out.push('_'),
        }
    }
    if let Some(q) = url.query() {
        let mut h = Sha256::new();
        h.update(q.as_bytes());
        let digest = h.finalize();
        out.push('-');
        for b in &digest[..3] {
            out.push_str(&format!("{b:02x}"));
        }
    }
    if out.len() > 180 {
        out.truncate(180);
    }
    out
}

fn sha256_hex(data: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(data);
    let d = h.finalize();
    let mut out = String::with_capacity(d.len() * 2);
    for b in d {
        out.push_str(&format!("{b:02x}"));
    }
    out
}

fn repo_root() -> Result<PathBuf> {
    if let Ok(dir) = std::env::var("CARGO_WORKSPACE_DIR") {
        return Ok(PathBuf::from(dir));
    }
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("git rev-parse")?;
    if out.status.success() {
        return Ok(PathBuf::from(String::from_utf8(out.stdout)?.trim()));
    }
    Ok(std::env::current_dir()?)
}

fn now_iso8601() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Append a PageRecord as one NDJSON line. Idempotent at the record level
/// (caller decides whether to call based on prior manifest scan).
fn append_manifest(path: &Path, rec: &PageRecord) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("open manifest {}", path.display()))?;
    let line = serde_json::to_string(rec)?;
    f.write_all(line.as_bytes())?;
    f.write_all(b"\n")?;
    Ok(())
}

/// Load already-mirrored URLs from manifest.ndjson (for idempotent reruns).
fn load_seen(path: &Path) -> Result<HashSet<String>> {
    let mut seen = HashSet::new();
    if !path.exists() {
        return Ok(seen);
    }
    let f = fs::File::open(path)?;
    for line in BufReader::new(f).lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(rec) = serde_json::from_str::<PageRecord>(&line) {
            seen.insert(rec.url);
        }
    }
    Ok(seen)
}

/// Pull every <a href>, plus same-origin CSS/img/font for asset capture.
fn extract_links_and_assets(html: &str, base: &Url) -> (BTreeSet<String>, BTreeSet<String>) {
    let doc = Html::parse_document(html);
    let mut links = BTreeSet::new();
    let mut assets = BTreeSet::new();

    let a_sel = Selector::parse("a[href]").unwrap();
    for el in doc.select(&a_sel) {
        if let Some(href) = el.value().attr("href") {
            if let Ok(joined) = base.join(href) {
                if let Some(c) = canonicalize(joined.as_str()) {
                    if c.host_str() == Some(TARGET_HOST) {
                        links.insert(c.to_string());
                    }
                }
            }
        }
    }

    for (sel_str, attr) in [
        ("link[rel=stylesheet][href]", "href"),
        ("img[src]", "src"),
        ("source[src]", "src"),
        ("link[rel=preload][href]", "href"),
    ] {
        let sel = Selector::parse(sel_str).unwrap();
        for el in doc.select(&sel) {
            if let Some(v) = el.value().attr(attr) {
                if let Ok(joined) = base.join(v) {
                    if joined.host_str() == Some(TARGET_HOST) {
                        assets.insert(joined.to_string());
                    }
                }
            }
        }
    }
    (links, assets)
}

/// Probe bxc daemon at 127.0.0.1:<port>. CDP exposes /json/version.
async fn bxc_daemon_alive(client: &reqwest::Client, port: u16) -> bool {
    let url = format!("http://127.0.0.1:{port}/json/version");
    match client.get(&url).timeout(Duration::from_secs(2)).send().await {
        Ok(r) => r.status().is_success(),
        Err(_) => false,
    }
}

/// Spawn `bxc launch --port N` detached in the background; return success.
///
/// Lookup order:
/// 1. `BXC_BIN` env var (explicit override).
/// 2. `bxc` / `bxc.exe` on `$PATH`.
/// 3. `~/.local/bin/bxc{,.exe}` (per project install convention, cf.
///    memory `project_aphrody_install_convention`).
fn spawn_bxc_daemon(port: u16) -> Result<()> {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(explicit) = std::env::var("BXC_BIN") {
        candidates.push(PathBuf::from(explicit));
    }
    candidates.push(PathBuf::from("bxc"));
    candidates.push(PathBuf::from("bxc.exe"));
    if let Some(home) = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")) {
        let home = PathBuf::from(home);
        candidates.push(home.join(".local").join("bin").join("bxc"));
        candidates.push(home.join(".local").join("bin").join("bxc.exe"));
    }
    for exe in candidates {
        let mut cmd = Command::new(&exe);
        cmd.args(["launch", "--port", &port.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .stdin(Stdio::null());
        if let Ok(child) = cmd.spawn() {
            tracing::info!("[bxc] spawned daemon on :{} (pid {})", port, child.id());
            return Ok(());
        }
    }
    Err(anyhow!(
        "bxc executable not found via BXC_BIN, $PATH, or ~/.local/bin"
    ))
}

/// Fetch a single page via the bxc daemon if available, else direct reqwest.
async fn fetch_page(
    client: &reqwest::Client,
    target: &Url,
    timeout: Duration,
    use_bxc: bool,
    bxc_port: u16,
) -> Result<(u16, String, String)> {
    if use_bxc {
        // Try `bxc fetch <url> --dump html --output -` via subprocess.
        // The daemon path (/json/new + WebSocket) would be lower-latency but
        // CDP plumbing is invasive; subprocess is the documented contract.
        if let Ok(out) = tokio::process::Command::new("bxc")
            .args([
                "fetch",
                target.as_str(),
                "--dump",
                "html",
                "--quiet",
                "--timeout",
                &timeout.as_secs().to_string(),
                "--port",
                &bxc_port.to_string(),
            ])
            .output()
            .await
        {
            if out.status.success() && !out.stdout.is_empty() {
                let body = String::from_utf8_lossy(&out.stdout).into_owned();
                return Ok((200, body, "bxc".into()));
            }
            tracing::debug!(
                "[bxc] fetch failed for {} ({}) — falling back to reqwest",
                target,
                out.status
            );
        }
    }

    let resp = client
        .get(target.as_str())
        .timeout(timeout)
        .send()
        .await
        .with_context(|| format!("reqwest GET {target}"))?;
    let status = resp.status().as_u16();
    let body = resp.text().await.context("read body")?;
    Ok((status, body, "reqwest".into()))
}

/// Download a binary asset (CSS/img/font), no JS rendering needed.
async fn download_asset(
    client: &reqwest::Client,
    target: &Url,
    out_path: &Path,
    timeout: Duration,
) -> Result<u64> {
    let resp = client
        .get(target.as_str())
        .timeout(timeout)
        .send()
        .await
        .with_context(|| format!("asset GET {target}"))?;
    if !resp.status().is_success() {
        return Err(anyhow!("asset {} -> HTTP {}", target, resp.status()));
    }
    let bytes = resp.bytes().await?;
    if let Some(p) = out_path.parent() {
        fs::create_dir_all(p)?;
    }
    fs::write(out_path, &bytes)?;
    Ok(bytes.len() as u64)
}

fn asset_local_name(asset_url: &Url) -> String {
    let last = asset_url
        .path_segments()
        .and_then(|s| s.last())
        .filter(|s| !s.is_empty())
        .unwrap_or("asset");
    let mut h = Sha256::new();
    h.update(asset_url.as_str().as_bytes());
    let digest = h.finalize();
    let prefix: String = digest[..4].iter().map(|b| format!("{b:02x}")).collect();
    let safe: String = last
        .chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '.' | '-' | '_' => c,
            _ => '_',
        })
        .collect();
    format!("{prefix}-{safe}")
}

pub(crate) fn run(args: Args) -> Result<()> {
    // rustls workspace uses `rustls-no-provider` → install ring before any
    // TLS handshake. Safe to call multiple times (returns Err on dup).
    let _ = rustls::crypto::ring::default_provider().install_default();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("build tokio runtime")?;
    rt.block_on(run_async(args))
}

async fn run_async(args: Args) -> Result<()> {
    let root = repo_root()?;
    let out_root = args.out.unwrap_or_else(|| root.join("assets/scrape/google-design"));
    let pages_dir = out_root.join("pages");
    let assets_dir = out_root.join("assets");
    let manifest_path = out_root.join("manifest.ndjson");
    let summary_path = out_root.join("summary.json");
    fs::create_dir_all(&pages_dir)?;
    fs::create_dir_all(&assets_dir)?;

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .pool_max_idle_per_host(8)
        .build()
        .context("build reqwest client")?;

    // Optional bxc daemon.
    let mut use_bxc = false;
    if args.bxc_port != 0 {
        if bxc_daemon_alive(&client, args.bxc_port).await {
            tracing::info!("[bxc] daemon already up on :{}", args.bxc_port);
            use_bxc = true;
        } else {
            match spawn_bxc_daemon(args.bxc_port) {
                Ok(()) => {
                    // Allow CDP a moment to bind.
                    tokio::time::sleep(Duration::from_millis(1500)).await;
                    if bxc_daemon_alive(&client, args.bxc_port).await {
                        use_bxc = true;
                        tracing::info!("[bxc] daemon ready on :{}", args.bxc_port);
                    } else {
                        tracing::warn!(
                            "[bxc] daemon spawn did not bind :{} — falling back to reqwest",
                            args.bxc_port
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!("[bxc] {} — using direct reqwest only", e);
                }
            }
        }
    }

    // robots.txt (Disallow-only minimal honour).
    let robots = match client
        .get(format!("https://{TARGET_HOST}/robots.txt"))
        .timeout(Duration::from_secs(10))
        .send()
        .await
    {
        Ok(r) if r.status().is_success() => {
            let body = r.text().await.unwrap_or_default();
            tracing::info!("[robots] parsed {} bytes", body.len());
            RobotsRules::parse(&body)
        }
        _ => {
            tracing::warn!("[robots] fetch failed — proceeding without rules");
            RobotsRules::default()
        }
    };

    let seen_urls = Arc::new(Mutex::new(load_seen(&manifest_path)?));
    tracing::info!("[manifest] {} URLs already mirrored (skip)", seen_urls.lock().await.len());

    let mut frontier: VecDeque<(Url, u32)> = VecDeque::new();
    let seed = canonicalize(SEED_URL).context("seed parse")?;
    frontier.push_back((seed.clone(), 0));

    let mut queued: HashSet<String> = HashSet::new();
    queued.insert(seed.to_string());

    let started_at = now_iso8601();
    let t0 = Instant::now();
    let mut depth_max = 0u32;
    let mut total_pages = 0usize;
    let mut total_assets = 0usize;
    let mut total_bytes: u64 = 0;
    let mut bxc_hits = 0usize;
    let timeout = Duration::from_secs(args.timeout);

    // Shared async-safe client handle for inner closures.
    let client = Arc::new(client);
    let bxc_port = args.bxc_port;
    let concurrency = args.concurrency.max(1);
    let max_pages = args.max_pages;
    let max_depth = args.depth;
    let no_assets = args.no_assets;

    'outer: while !frontier.is_empty() && total_pages < max_pages {
        // Drain up to `concurrency` URLs from the frontier into a batch.
        let mut batch: Vec<(Url, u32)> = Vec::with_capacity(concurrency);
        while batch.len() < concurrency && total_pages + batch.len() < max_pages {
            let Some((u, d)) = frontier.pop_front() else { break };
            if d > max_depth {
                continue;
            }
            if !robots.is_allowed(u.path()) {
                tracing::debug!("[robots] skip {}", u);
                continue;
            }
            if seen_urls.lock().await.contains(u.as_str()) {
                continue;
            }
            batch.push((u, d));
        }
        if batch.is_empty() {
            if frontier.is_empty() {
                break 'outer;
            }
            continue;
        }

        // Stage 1: concurrent page fetches → (PageRecord, body_bytes, asset_urls).
        type PageOutput = (PageRecord, Vec<u8>, Vec<Url>);
        let fetches = stream::iter(batch.into_iter()).map(|(url, depth)| {
            let client = Arc::clone(&client);
            async move {
                let res =
                    fetch_page(client.as_ref(), &url, timeout, use_bxc, bxc_port).await;
                let (status, body, source) = match res {
                    Ok(t) => t,
                    Err(e) => {
                        tracing::warn!("[fetch] {} — {:#}", url, e);
                        return None;
                    }
                };
                let (links, assets) = extract_links_and_assets(&body, &url);
                let slug = slug_from_url(&url);
                let rec = PageRecord {
                    url: url.to_string(),
                    slug,
                    fetched_at: now_iso8601(),
                    status,
                    sha256: sha256_hex(body.as_bytes()),
                    bytes: body.len() as u64,
                    depth,
                    links_out: links.into_iter().collect(),
                    source,
                };
                let asset_urls: Vec<Url> = assets
                    .iter()
                    .filter_map(|s| Url::parse(s).ok())
                    .take(ASSET_BUDGET_PER_PAGE)
                    .collect();
                Some((rec, body.into_bytes(), asset_urls))
            }
        });
        let page_outputs: Vec<PageOutput> = fetches
            .buffer_unordered(concurrency)
            .filter_map(|x| async move { x })
            .collect()
            .await;

        // Stage 2: write HTML bodies + collect unique asset jobs.
        let mut asset_jobs: Vec<(Url, PathBuf)> = Vec::new();
        let mut asset_seen: HashSet<String> = HashSet::new();
        for (rec, body, asset_urls) in &page_outputs {
            let html_path = pages_dir.join(format!("{}.html", rec.slug));
            if let Some(p) = html_path.parent() {
                let _ = fs::create_dir_all(p);
            }
            if let Err(e) = fs::write(&html_path, body) {
                tracing::warn!("[write] {}: {}", html_path.display(), e);
            }
            if no_assets {
                continue;
            }
            for au in asset_urls {
                let key = au.to_string();
                if !asset_seen.insert(key) {
                    continue;
                }
                let local = assets_dir.join(asset_local_name(au));
                if !local.exists() {
                    asset_jobs.push((au.clone(), local));
                }
            }
        }

        // Stage 3: concurrent asset downloads.
        if !asset_jobs.is_empty() {
            let asset_results: Vec<Result<u64>> = stream::iter(asset_jobs.into_iter())
                .map(|(u, path)| {
                    let client = Arc::clone(&client);
                    async move { download_asset(client.as_ref(), &u, &path, timeout).await }
                })
                .buffer_unordered(concurrency)
                .collect()
                .await;
            for r in asset_results {
                match r {
                    Ok(n) => {
                        total_assets += 1;
                        total_bytes = total_bytes.saturating_add(n);
                    }
                    Err(e) => tracing::debug!("[asset] {:#}", e),
                }
            }
        }

        // Stage 4: commit manifest entries + enqueue out-links.
        for (rec, _body, _assets) in page_outputs {
            depth_max = depth_max.max(rec.depth);
            total_bytes = total_bytes.saturating_add(rec.bytes);
            if rec.source == "bxc" {
                bxc_hits += 1;
            }
            append_manifest(&manifest_path, &rec)?;
            seen_urls.lock().await.insert(rec.url.clone());
            total_pages += 1;

            if rec.depth < max_depth {
                for link in &rec.links_out {
                    if queued.contains(link) {
                        continue;
                    }
                    if let Some(c) = canonicalize(link) {
                        if c.host_str() != Some(TARGET_HOST) {
                            continue;
                        }
                        queued.insert(c.to_string());
                        frontier.push_back((c, rec.depth + 1));
                    }
                }
            }
            if total_pages >= max_pages {
                break 'outer;
            }
        }

        tracing::info!(
            "[crawl] pages={}/{}  assets={}  frontier={}  depth_max={}",
            total_pages,
            max_pages,
            total_assets,
            frontier.len(),
            depth_max
        );
    }

    let completed_at = now_iso8601();
    let summary = Summary {
        seed: SEED_URL.to_string(),
        total_pages,
        total_assets,
        total_bytes,
        depth_max,
        started_at,
        completed_at,
        duration_ms: t0.elapsed().as_millis(),
        bxc_used: bxc_hits > 0,
    };
    fs::write(&summary_path, format!("{}\n", serde_json::to_string_pretty(&summary)?))
        .with_context(|| format!("write {}", summary_path.display()))?;

    tracing::info!(
        "[done] pages={} assets={} bytes={} depth={} bxc_pages={} in {:.2}s — {}",
        total_pages,
        total_assets,
        total_bytes,
        depth_max,
        bxc_hits,
        t0.elapsed().as_secs_f64(),
        summary_path.display()
    );
    Ok(())
}

// -------------------------------------------------------------------------
// Unit tests
// -------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn robots_parses_disallow_for_star_only() {
        let body = "User-agent: *\nDisallow: /api/\nDisallow: /private\n\n\
                    User-agent: Googlebot\nDisallow: /everything\n";
        let r = RobotsRules::parse(body);
        assert_eq!(r.disallow, vec!["/api/".to_string(), "/private".to_string()]);
        assert!(r.is_allowed("/library/foo"));
        assert!(!r.is_allowed("/api/x"));
        assert!(!r.is_allowed("/private"));
    }

    #[test]
    fn slug_handles_root_nested_and_query() {
        let root = slug_from_url(&Url::parse("https://design.google/").unwrap());
        assert_eq!(root, "index");

        let nested = slug_from_url(&Url::parse("https://design.google/library/material-3").unwrap());
        assert_eq!(nested, "library__material-3");

        let with_q =
            slug_from_url(&Url::parse("https://design.google/search?q=fonts&p=2").unwrap());
        assert!(with_q.starts_with("search-"), "got {with_q}");
        // suffix is 6 hex chars derived from the query
        assert_eq!(with_q.len(), "search-".len() + 6);
    }

    #[test]
    fn ndjson_append_is_idempotent_via_load_seen() {
        let dir = tempdir().unwrap();
        let p = dir.path().join("manifest.ndjson");
        let rec = PageRecord {
            url: "https://design.google/library/x".into(),
            slug: "library__x".into(),
            fetched_at: "2026-05-18T00:00:00Z".into(),
            status: 200,
            sha256: "deadbeef".into(),
            bytes: 42,
            depth: 1,
            links_out: vec!["https://design.google/about".into()],
            source: "reqwest".into(),
        };
        append_manifest(&p, &rec).unwrap();
        append_manifest(&p, &rec).unwrap(); // duplicate physical line OK
        let seen = load_seen(&p).unwrap();
        assert_eq!(seen.len(), 1, "load_seen dedupes by url");
        assert!(seen.contains("https://design.google/library/x"));
    }

    #[test]
    fn depth_limit_is_respected_by_bfs_gate() {
        // Pure-logic surrogate for the BFS depth gate used in run_async.
        let max_depth = 2u32;
        let frontier: Vec<(String, u32)> = vec![
            ("https://design.google/".into(), 0),
            ("https://design.google/a".into(), 1),
            ("https://design.google/a/b".into(), 2),
            ("https://design.google/a/b/c".into(), 3),
        ];
        let kept: Vec<_> = frontier.iter().filter(|(_, d)| *d <= max_depth).collect();
        assert_eq!(kept.len(), 3);
        assert!(kept.iter().all(|(_, d)| *d <= max_depth));
    }

    #[test]
    fn extract_links_keeps_only_same_origin() {
        // r##"..."## — payload contains `"#frag"` which would close r#.
        let html = r##"
            <html><body>
              <a href="/library/x">x</a>
              <a href="https://design.google/about">about</a>
              <a href="https://google.com/elsewhere">off-origin</a>
              <a href="#frag">frag-only</a>
              <link rel="stylesheet" href="/static/app.css">
              <img src="https://design.google/img/hero.png">
              <img src="https://cdn.other.com/x.png">
            </body></html>
        "##;
        let base = Url::parse("https://design.google/").unwrap();
        let (links, assets) = extract_links_and_assets(html, &base);
        assert!(links.contains("https://design.google/library/x"));
        assert!(links.contains("https://design.google/about"));
        assert!(!links.iter().any(|l| l.contains("google.com/elsewhere")));
        assert!(assets.contains("https://design.google/static/app.css"));
        assert!(assets.contains("https://design.google/img/hero.png"));
        assert!(!assets.iter().any(|a| a.contains("cdn.other.com")));
    }
}
